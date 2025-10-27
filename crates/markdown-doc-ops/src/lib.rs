//! High-level operations shared by markdown-doc commands.

mod anchors;
mod lines;
mod lint;
mod paths;
pub mod refactor;
mod schema;
mod toc;

use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::SystemTime;

use globset::{Glob, GlobSet, GlobSetBuilder};
use ignore::gitignore::Gitignore;
use markdown_doc_config::Config;
use markdown_doc_format::{
    CatalogEntry, CatalogFormat, CatalogRenderData, HeadingSummary, LintFormat, LintRenderData,
    Renderer, ValidateFinding, ValidateFormat, ValidateRenderData,
};
use markdown_doc_parser::{DocumentSection, ParserContext};
use markdown_doc_utils::atomic_write;
use rayon::prelude::*;
use similar::TextDiff;
use thiserror::Error;
use walkdir::WalkDir;

use crate::anchors::normalize_anchor_fragment;
use crate::paths::normalize_path;
use crate::refactor::graph::LinkGraph;
use crate::refactor::rewrite::{plan_file_moves, FileMove, RewriteError};
use crate::schema::SchemaEngine;

/// Primary entry point for catalog and lint operations.
pub struct Operations {
    config: Config,
    parser: ParserContext,
    renderer: Renderer,
    schema_engine: SchemaEngine,
    ignore_filter: Option<Gitignore>,
}

impl Operations {
    /// Assemble the operation layer from config by wiring parser + renderer.
    pub fn new(config: Config) -> Self {
        let parser = ParserContext::new(config.clone());
        let renderer = Renderer::from_config(config.clone());
        let schema_engine = SchemaEngine::new(&config.schemas);
        let ignore_filter = load_ignore_filter(&config.project.root);
        Self {
            config,
            parser,
            renderer,
            schema_engine,
            ignore_filter,
        }
    }

    /// Generate documentation catalog data and optionally write the Markdown output to disk.
    pub fn catalog(&self, options: CatalogOptions) -> Result<CatalogOutcome, OperationError> {
        let targets = self.collect_targets(&options.scan)?;

        let entries: Vec<CatalogEntry> = targets
            .par_iter()
            .map(|path| {
                let sections =
                    self.parser
                        .sections_for_path(path)
                        .map_err(|source| OperationError::Io {
                            path: path.clone(),
                            source,
                        })?;
                Ok(build_catalog_entry(path, sections))
            })
            .collect::<Result<Vec<_>, OperationError>>()?;

        let mut sorted_entries = entries;
        sorted_entries.sort_by(|a, b| a.path.cmp(&b.path));

        let data = CatalogRenderData {
            generated_at: SystemTime::now(),
            entries: sorted_entries.clone(),
        };

        let rendered = match options.format {
            CatalogFormat::Markdown => self.renderer.render_catalog_markdown(&data),
            CatalogFormat::Json => self.renderer.render_catalog_json(&data)?,
        };

        if options.write_to_disk {
            let output_path = options
                .output_path
                .unwrap_or_else(|| self.config.catalog.output.clone());
            let absolute = self.resolve_output_path(&output_path);
            atomic_write(&absolute, &rendered)?;
        }

        Ok(CatalogOutcome {
            rendered,
            format: options.format,
            entries: sorted_entries,
        })
    }

    /// Execute configured lint rules and return a renderable report plus exit code.
    pub fn lint(&self, options: LintOptions) -> Result<LintOutcome, OperationError> {
        let targets = self.collect_targets(&options.scan)?;

        let result = lint::run(lint::LintRunInput {
            config: &self.config,
            parser: &self.parser,
            targets: &targets,
            root: &self.config.project.root,
            schema_engine: &self.schema_engine,
        })?;

        let report = LintRenderData {
            files_scanned: result.files_scanned,
            error_count: result.error_count,
            warning_count: result.warning_count,
            findings: result.findings.clone(),
        };

        let rendered = self.render_lint(&report, options.format)?;
        let exit_code = if result.error_count > 0 { 1 } else { 0 };

        Ok(LintOutcome {
            rendered,
            report,
            exit_code,
        })
    }

    /// Temporary wrapper retaining the legacy method name for compatibility.
    pub fn lint_broken_links(&self, options: LintOptions) -> Result<LintOutcome, OperationError> {
        self.lint(options)
    }

    /// Execute schema validation across the selected targets.
    pub fn validate(&self, options: ValidateOptions) -> Result<ValidateOutcome, OperationError> {
        let targets = self.collect_targets(&options.scan)?;

        let schema_override = match &options.schema {
            Some(name) => Some(
                self.schema_engine
                    .schema_by_name(name)
                    .ok_or_else(|| OperationError::SchemaNotFound { name: name.clone() })?,
            ),
            None => None,
        };

        let mut findings = Vec::new();

        for path in &targets {
            let absolute = self.config.project.root.join(path);
            let contents =
                std::fs::read_to_string(&absolute).map_err(|source| OperationError::Io {
                    path: absolute,
                    source,
                })?;

            let line_offsets = lines::compute_line_offsets(&contents);
            let sections = self.parser.sections_from_str(path, &contents);

            let schema = schema_override
                .as_ref()
                .cloned()
                .unwrap_or_else(|| self.schema_engine.schema_for_path(path));

            let check = self
                .schema_engine
                .check(schema.clone(), &sections, &line_offsets);

            for violation in check.violations {
                findings.push(ValidateFinding {
                    path: path.clone(),
                    line: violation.line,
                    message: violation.message,
                    schema: schema.name().to_string(),
                });
            }
        }

        let error_count = findings.len();
        let report = ValidateRenderData {
            files_scanned: targets.len(),
            error_count,
            findings,
        };

        let rendered = if options.quiet && error_count == 0 {
            String::new()
        } else {
            match options.format {
                ValidateFormat::Plain => self.renderer.render_validate_plain(&report),
                ValidateFormat::Json => self.renderer.render_validate_json(&report)?,
            }
        };

        let exit_code = if error_count > 0 { 1 } else { 0 };

        Ok(ValidateOutcome {
            rendered,
            report,
            exit_code,
        })
    }

    /// Synchronise TOC blocks against live heading structure.
    pub fn toc(&self, options: TocOptions) -> Result<TocOutcome, OperationError> {
        let targets = self.collect_targets(&options.scan)?;

        let mut changes = Vec::new();
        let mut messages = Vec::new();
        let mut requires_update = false;
        let mut encountered_error = false;

        for path in targets {
            let absolute = self.config.project.root.join(&path);
            let contents =
                std::fs::read_to_string(&absolute).map_err(|source| OperationError::Io {
                    path: absolute.clone(),
                    source,
                })?;

            let sections = self.parser.sections_from_str(&path, &contents);
            let block = match toc::locate_block(&contents, &self.config.lint.toc) {
                Some(block) => block,
                None => {
                    encountered_error = true;
                    changes.push(TocChange {
                        path: path.clone(),
                        status: TocStatus::MissingMarkers,
                        diff: None,
                    });
                    messages.push(format!("❌ {} missing TOC markers", path.display()));
                    continue;
                }
            };

            let generated_items = toc::generate_items(&sections);
            let existing_body = contents[block.start_offset..block.end_offset].to_string();
            let line_sep = if existing_body.contains("\r\n") || contents.contains("\r\n") {
                "\r\n"
            } else {
                "\n"
            };
            let rendered_body = toc::render_items_with_separator(&generated_items, line_sep);

            if existing_body == rendered_body {
                changes.push(TocChange {
                    path: path.clone(),
                    status: TocStatus::UpToDate,
                    diff: None,
                });
                continue;
            }

            match options.mode {
                TocMode::Check => {
                    requires_update = true;
                    changes.push(TocChange {
                        path: path.clone(),
                        status: TocStatus::NeedsUpdate,
                        diff: None,
                    });
                    messages.push(format!("❌ {} requires TOC update", path.display()));
                }
                TocMode::Diff => {
                    requires_update = true;
                    let diff = build_diff(&path, &existing_body, &rendered_body);
                    changes.push(TocChange {
                        path: path.clone(),
                        status: TocStatus::NeedsUpdate,
                        diff: Some(diff.clone()),
                    });
                    messages.push(diff);
                }
                TocMode::Update => {
                    let mut updated = String::new();
                    updated.push_str(&contents[..block.start_offset]);
                    updated.push_str(&rendered_body);
                    updated.push_str(&contents[block.end_offset..]);
                    atomic_write(&absolute, &updated)?;
                    changes.push(TocChange {
                        path: path.clone(),
                        status: TocStatus::Updated,
                        diff: None,
                    });
                    messages.push(format!("✏️  updated {}", path.display()));
                }
            }
        }

        let exit_code = match options.mode {
            TocMode::Update => {
                if encountered_error {
                    1
                } else {
                    0
                }
            }
            TocMode::Check | TocMode::Diff => {
                if requires_update || encountered_error {
                    1
                } else {
                    0
                }
            }
        };

        let rendered = if options.quiet && messages.is_empty() {
            String::new()
        } else {
            messages.join("\n")
        };

        Ok(TocOutcome {
            rendered,
            changes,
            exit_code,
        })
    }

    /// Build a link graph covering the selected targets.
    pub fn link_graph(&self, options: ScanOptions) -> Result<LinkGraph, OperationError> {
        let targets = self.collect_targets(&options)?;
        LinkGraph::build(&self.parser, &self.config.project.root, &targets)
    }

    /// Locate references to a given Markdown path or anchor.
    pub fn refs(&self, options: RefsOptions) -> Result<RefsOutcome, OperationError> {
        let root = &self.config.project.root;
        let query = ReferenceQuery::new(root, &options.pattern, options.anchor_only)?;

        let targets = self.collect_targets(&options.scan)?;
        let graph = LinkGraph::build(&self.parser, root, &targets)?;

        let mut matches = Vec::new();
        for entry in graph.files() {
            for link in entry.links() {
                if let Some(target) = &link.target {
                    if query.matches(target) {
                        matches.push(RefsMatch {
                            source: entry.path().to_path_buf(),
                            line: link.line,
                            display: entry.line_text(link.line).unwrap_or_default(),
                            target_path: target.path.clone(),
                            target_anchor: target.anchor.clone(),
                        });
                    }
                }
            }

            for definition in entry.definitions() {
                if let Some(target) = &definition.target {
                    if query.matches(target) {
                        matches.push(RefsMatch {
                            source: entry.path().to_path_buf(),
                            line: definition.line,
                            display: entry.line_text(definition.line).unwrap_or_default(),
                            target_path: target.path.clone(),
                            target_anchor: target.anchor.clone(),
                        });
                    }
                }
            }
        }

        matches.sort_by(|a, b| match a.source.cmp(&b.source) {
            std::cmp::Ordering::Equal => a.line.cmp(&b.line),
            other => other,
        });

        let exit_code = if matches.is_empty() { 1 } else { 0 };

        Ok(RefsOutcome {
            query: options.pattern,
            matches,
            exit_code,
        })
    }

    /// Move/rename a Markdown file while updating inbound/outbound references.
    pub fn mv(&self, options: MvOptions) -> Result<MvOutcome, OperationError> {
        let root = &self.config.project.root;
        let (source_rel, source_abs) = resolve_input_path(root, &options.source, true, "source")?;
        if !is_markdown_path(&source_rel.to_string_lossy()) {
            return Err(OperationError::InvalidInput(format!(
                "source '{}' is not a Markdown file",
                source_rel.display()
            )));
        }
        let source_meta = fs::metadata(&source_abs).map_err(|source| OperationError::Io {
            path: source_abs.clone(),
            source,
        })?;
        if !source_meta.is_file() {
            return Err(OperationError::InvalidInput(format!(
                "source '{}' must be a file",
                source_rel.display()
            )));
        }

        let (dest_rel, dest_abs) =
            resolve_input_path(root, &options.destination, false, "destination")?;
        if !is_markdown_path(&dest_rel.to_string_lossy()) {
            return Err(OperationError::InvalidInput(format!(
                "destination '{}' must end with .md or .markdown",
                dest_rel.display()
            )));
        }
        if source_rel == dest_rel {
            return Err(OperationError::InvalidInput(
                "source and destination refer to the same path".into(),
            ));
        }
        if dest_abs.exists() && !options.force {
            return Err(OperationError::InvalidInput(format!(
                "destination '{}' already exists (use --force to overwrite)",
                dest_rel.display()
            )));
        }

        let mut targets = self.collect_targets(&options.scan)?;
        if !targets.contains(&source_rel) {
            targets.push(source_rel.clone());
            targets.sort();
            targets.dedup();
        }
        let graph = LinkGraph::build(&self.parser, root, &targets)?;

        let plan = plan_file_moves(
            &graph,
            root,
            &[FileMove {
                from: source_rel.clone(),
                to: dest_rel.clone(),
            }],
        )?;

        #[derive(Clone)]
        struct ProcessedEdit {
            edit: crate::refactor::rewrite::FileEdit,
            original_contents: String,
            diff: Option<String>,
            status: MvFileStatus,
        }

        let mut processed = Vec::new();
        for edit in &plan.file_edits {
            let entry = graph.file(&edit.original_path).ok_or_else(|| {
                OperationError::Rewrite(RewriteError::MissingFile {
                    path: edit.original_path.clone(),
                })
            })?;
            let original_contents = entry.contents().to_string();
            let diff = if original_contents == edit.updated_contents {
                None
            } else {
                Some(build_diff(
                    &edit.original_path,
                    &original_contents,
                    &edit.updated_contents,
                ))
            };
            let status = if edit.original_path != edit.output_path {
                MvFileStatus::Relocated
            } else if diff.is_some() {
                MvFileStatus::Updated
            } else {
                MvFileStatus::Unchanged
            };
            processed.push(ProcessedEdit {
                edit: edit.clone(),
                original_contents,
                diff,
                status,
            });
        }

        let changes: Vec<MvFileChange> = processed
            .iter()
            .map(|edit| MvFileChange {
                original_path: edit.edit.original_path.clone(),
                output_path: edit.edit.output_path.clone(),
                status: edit.status,
                diff: if options.dry_run {
                    edit.diff.clone()
                } else {
                    None
                },
            })
            .collect();

        if options.dry_run {
            return Ok(MvOutcome {
                changes,
                exit_code: 0,
                dry_run: true,
            });
        }

        struct AppliedOperation {
            original_path: PathBuf,
            output_path: PathBuf,
            original_contents: String,
            dest_original_contents: Option<String>,
            rename: bool,
        }

        let mut applied: Vec<AppliedOperation> = Vec::new();

        let apply_result: Result<(), OperationError> = (|| {
            // Apply relocations first so references point to valid destination.
            for edit in processed
                .iter()
                .filter(|edit| edit.edit.original_path != edit.edit.output_path)
            {
                let source_abs = root.join(&edit.edit.original_path);
                let dest_abs = root.join(&edit.edit.output_path);

                fs::create_dir_all(dest_abs.parent().ok_or_else(|| {
                    OperationError::InvalidInput(format!(
                        "destination '{}' does not have a parent directory",
                        edit.edit.output_path.display()
                    ))
                })?)
                .map_err(|source| OperationError::Io {
                    path: dest_abs.clone(),
                    source,
                })?;

                let mut dest_original_contents = None;
                if dest_abs.exists() {
                    if !options.force {
                        return Err(OperationError::InvalidInput(format!(
                            "destination '{}' already exists",
                            edit.edit.output_path.display()
                        )));
                    }
                    dest_original_contents =
                        Some(fs::read_to_string(&dest_abs).map_err(|source| {
                            OperationError::Io {
                                path: dest_abs.clone(),
                                source,
                            }
                        })?);
                    if options.create_backup {
                        maybe_create_backup(&dest_abs).map_err(|source| OperationError::Io {
                            path: dest_abs.clone(),
                            source,
                        })?;
                    }
                    fs::remove_file(&dest_abs).map_err(|source| OperationError::Io {
                        path: dest_abs.clone(),
                        source,
                    })?;
                }

                if options.create_backup && source_abs.exists() {
                    maybe_create_backup(&source_abs).map_err(|source| OperationError::Io {
                        path: source_abs.clone(),
                        source,
                    })?;
                }

                fs::rename(&source_abs, &dest_abs).map_err(|source| OperationError::Io {
                    path: source_abs.clone(),
                    source,
                })?;

                if edit.status != MvFileStatus::Unchanged {
                    atomic_write(&dest_abs, &edit.edit.updated_contents).map_err(|source| {
                        OperationError::Io {
                            path: dest_abs.clone(),
                            source,
                        }
                    })?;
                }

                applied.push(AppliedOperation {
                    original_path: edit.edit.original_path.clone(),
                    output_path: edit.edit.output_path.clone(),
                    original_contents: edit.original_contents.clone(),
                    dest_original_contents,
                    rename: true,
                });
            }

            // Update files that remain in place.
            for edit in processed
                .iter()
                .filter(|edit| edit.edit.original_path == edit.edit.output_path)
            {
                if edit.status == MvFileStatus::Unchanged {
                    continue;
                }

                let path_abs = root.join(&edit.edit.original_path);
                if options.create_backup && path_abs.exists() {
                    maybe_create_backup(&path_abs).map_err(|source| OperationError::Io {
                        path: path_abs.clone(),
                        source,
                    })?;
                }

                atomic_write(&path_abs, &edit.edit.updated_contents).map_err(|source| {
                    OperationError::Io {
                        path: path_abs.clone(),
                        source,
                    }
                })?;

                applied.push(AppliedOperation {
                    original_path: edit.edit.original_path.clone(),
                    output_path: edit.edit.output_path.clone(),
                    original_contents: edit.original_contents.clone(),
                    dest_original_contents: None,
                    rename: false,
                });
            }

            Ok(())
        })();

        if let Err(err) = apply_result {
            // Attempt rollback best-effort.
            for op in applied.iter().rev() {
                if op.rename {
                    let dest_abs = root.join(&op.output_path);
                    let original_abs = root.join(&op.original_path);
                    if dest_abs.exists() {
                        if let Err(rename_err) = fs::rename(&dest_abs, &original_abs) {
                            let _ = atomic_write(&original_abs, &op.original_contents);
                            if let Some(dest_contents) = &op.dest_original_contents {
                                let _ = atomic_write(&dest_abs, dest_contents);
                            }
                            let _ = rename_err;
                        } else if let Some(dest_contents) = &op.dest_original_contents {
                            let _ = atomic_write(&dest_abs, dest_contents);
                        }
                    } else {
                        let _ = atomic_write(&original_abs, &op.original_contents);
                        if let Some(dest_contents) = &op.dest_original_contents {
                            let _ = atomic_write(&dest_abs, dest_contents);
                        }
                    }
                } else {
                    let path_abs = root.join(&op.original_path);
                    let _ = atomic_write(&path_abs, &op.original_contents);
                }
            }
            return Err(err);
        }

        Ok(MvOutcome {
            changes,
            exit_code: 0,
            dry_run: false,
        })
    }

    fn render_lint(
        &self,
        report: &LintRenderData,
        format: LintFormat,
    ) -> Result<String, OperationError> {
        let rendered = match format {
            LintFormat::Plain => self.renderer.render_lint_plain(report),
            LintFormat::Json => self.renderer.render_lint_json(report)?,
            LintFormat::Sarif => self.renderer.render_lint_sarif(report)?,
        };
        Ok(rendered)
    }

    fn collect_targets(&self, options: &ScanOptions) -> Result<Vec<PathBuf>, OperationError> {
        let root = self.config.project.root.clone();
        let mut candidates = if options.staged {
            let staged = self.git_staged_files()?;
            if options.paths.is_empty() {
                staged
            } else {
                filter_paths(staged, &options.paths, &root)
            }
        } else if options.paths.is_empty() {
            walk_markdown_files(&root, self.ignore_filter.as_ref())?
        } else {
            collect_from_paths(&root, &options.paths, self.ignore_filter.as_ref())?
        };

        candidates.retain(|path| self.parser.is_path_in_scope(path));

        if options.respect_ignore {
            if let Some(filter) = &self.ignore_filter {
                let root = &self.config.project.root;
                candidates.retain(|path| {
                    let absolute = root.join(path);
                    !filter
                        .matched_path_or_any_parents(&absolute, false)
                        .is_ignore()
                });
            }
        }
        candidates.sort();
        candidates.dedup();
        Ok(candidates)
    }

    fn git_staged_files(&self) -> Result<Vec<PathBuf>, OperationError> {
        let output = Command::new("git")
            .arg("-C")
            .arg(&self.config.project.root)
            .arg("diff")
            .arg("--name-only")
            .arg("--cached")
            .output()
            .map_err(|err| OperationError::Git {
                source: err,
                message: "failed to invoke git".into(),
            })?;

        if !output.status.success() {
            return Err(OperationError::Git {
                source: io::Error::other(String::from_utf8_lossy(&output.stderr).to_string()),
                message: "git diff --name-only --cached failed".into(),
            });
        }

        let mut files = Vec::new();
        for line in String::from_utf8_lossy(&output.stdout).lines() {
            let path = PathBuf::from(line);
            if is_markdown_path(line) {
                files.push(path);
            }
        }

        Ok(files)
    }

    fn resolve_output_path(&self, path: &Path) -> PathBuf {
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.config.project.root.join(path)
        }
    }
}

fn load_ignore_filter(root: &Path) -> Option<Gitignore> {
    let ignore_path = root.join(".markdown-doc-ignore");
    if !ignore_path.exists() {
        return None;
    }
    let (filter, error) = Gitignore::new(ignore_path);
    if error.is_some() {
        None
    } else {
        Some(filter)
    }
}

fn walk_markdown_files(
    root: &Path,
    ignore_filter: Option<&Gitignore>,
) -> Result<Vec<PathBuf>, OperationError> {
    let mut files = Vec::new();
    let mut walker = WalkDir::new(root).into_iter();
    while let Some(entry) = walker.next() {
        match entry {
            Ok(entry) => {
                let path = entry.path();

                if entry.file_type().is_dir() {
                    if is_ignored(path, ignore_filter, root) {
                        walker.skip_current_dir();
                    }
                    continue;
                }

                if let Some(name) = path.file_name().and_then(OsStr::to_str) {
                    if is_markdown_path(name) {
                        let relative = path.strip_prefix(root).unwrap_or(path);
                        files.push(relative.to_path_buf());
                    }
                }
            }
            Err(err) => {
                let path_buf = err
                    .path()
                    .map(|p| p.to_path_buf())
                    .unwrap_or_else(|| root.to_path_buf());
                if is_ignored(&path_buf, ignore_filter, root) {
                    walker.skip_current_dir();
                    continue;
                }

                let message = err.to_string();
                let source = err
                    .into_io_error()
                    .unwrap_or_else(|| io::Error::other(message));
                return Err(OperationError::Io {
                    path: path_buf,
                    source,
                });
            }
        }
    }
    Ok(files)
}

fn collect_from_paths(
    root: &Path,
    paths: &[PathBuf],
    ignore_filter: Option<&Gitignore>,
) -> Result<Vec<PathBuf>, OperationError> {
    let mut results = Vec::new();
    for provided in paths {
        let absolute = if provided.is_absolute() {
            provided.clone()
        } else {
            root.join(provided)
        };

        if absolute.is_dir() {
            let mut nested = walk_markdown_files(&absolute, ignore_filter)?;
            if let Ok(stripped) = absolute.strip_prefix(root) {
                nested.iter_mut().for_each(|path| {
                    let full = stripped.join(&*path);
                    *path = full;
                });
            }
            results.extend(nested);
        } else if absolute.is_file()
            && is_markdown_path(
                absolute
                    .file_name()
                    .and_then(OsStr::to_str)
                    .unwrap_or_default(),
            )
        {
            let rel = absolute.strip_prefix(root).unwrap_or(&absolute);
            results.push(rel.to_path_buf());
        }
    }

    Ok(results)
}

fn build_diff(path: &Path, before: &str, after: &str) -> String {
    let old_header = format!("a/{}", path.display());
    let new_header = format!("b/{}", path.display());
    let diff = TextDiff::from_lines(before, after)
        .unified_diff()
        .context_radius(3)
        .header(&old_header, &new_header)
        .to_string();
    if diff.ends_with('\n') {
        diff
    } else {
        format!("{}\n", diff)
    }
}

fn filter_paths(files: Vec<PathBuf>, filters: &[PathBuf], root: &Path) -> Vec<PathBuf> {
    if filters.is_empty() {
        files
    } else {
        let filter_set: Vec<PathBuf> = filters
            .iter()
            .map(|filter| {
                if filter.is_absolute() {
                    filter.clone()
                } else {
                    root.join(filter)
                }
            })
            .collect();

        files
            .into_iter()
            .filter(|file| {
                let absolute = root.join(file);
                filter_set.iter().any(|filter| {
                    if filter.is_dir() {
                        absolute.starts_with(filter)
                    } else {
                        &absolute == filter
                    }
                })
            })
            .collect()
    }
}

fn is_ignored(path: &Path, filter: Option<&Gitignore>, root: &Path) -> bool {
    filter
        .map(|ignore| {
            let absolute = if path.is_absolute() {
                path.to_path_buf()
            } else {
                root.join(path)
            };
            ignore
                .matched_path_or_any_parents(&absolute, path.is_dir())
                .is_ignore()
        })
        .unwrap_or(false)
}

fn resolve_input_path(
    root: &Path,
    provided: &Path,
    must_exist: bool,
    label: &str,
) -> Result<(PathBuf, PathBuf), OperationError> {
    let candidate = if provided.is_absolute() {
        provided.to_path_buf()
    } else {
        root.join(provided)
    };
    let normalized = normalize_path(candidate);

    if !normalized.starts_with(root) {
        return Err(OperationError::InvalidInput(format!(
            "{label} '{}' resolves outside the project root",
            provided.display()
        )));
    }

    if must_exist && !normalized.exists() {
        return Err(OperationError::InvalidInput(format!(
            "{label} '{}' does not exist",
            provided.display()
        )));
    }

    let relative = normalized
        .strip_prefix(root)
        .map_err(|_| {
            OperationError::InvalidInput(format!(
                "{label} '{}' could not be resolved relative to project root",
                provided.display()
            ))
        })?
        .to_path_buf();

    Ok((relative, normalized))
}

fn maybe_create_backup(path: &Path) -> io::Result<()> {
    if !path.exists() {
        return Ok(());
    }
    let backup = derive_backup_path(path)
        .ok_or_else(|| io::Error::other("cannot derive backup filename"))?;
    if let Some(parent) = backup.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::copy(path, &backup)?;
    Ok(())
}

fn derive_backup_path(path: &Path) -> Option<PathBuf> {
    let mut name = path.file_name()?.to_os_string();
    name.push(".bak");
    Some(path.with_file_name(name))
}

struct ReferenceQuery {
    path_matcher: Option<PathMatcher>,
    anchor: Option<String>,
}

impl ReferenceQuery {
    fn new(root: &Path, pattern: &str, anchor_only: bool) -> Result<Self, OperationError> {
        let trimmed = pattern.trim();
        if trimmed.is_empty() {
            return Err(OperationError::InvalidInput(
                "refs pattern cannot be empty".into(),
            ));
        }

        let (path_part, anchor_part) = if anchor_only {
            (None, Some(trimmed))
        } else if let Some(idx) = trimmed.find('#') {
            let (path_segment, anchor_segment) = trimmed.split_at(idx);
            let anchor_segment = &anchor_segment[1..];
            let anchor_segment = anchor_segment.trim();
            let path_segment = path_segment.trim();
            let path_opt = if path_segment.is_empty() {
                None
            } else {
                Some(path_segment)
            };
            let anchor_opt = if anchor_segment.is_empty() {
                None
            } else {
                Some(anchor_segment)
            };
            (path_opt, anchor_opt)
        } else if let Some(stripped) = trimmed.strip_prefix('#') {
            let anchor_segment = stripped.trim();
            if anchor_segment.is_empty() {
                return Err(OperationError::InvalidInput(
                    "anchor query must not be empty".into(),
                ));
            }
            (None, Some(anchor_segment))
        } else {
            (Some(trimmed), None)
        };

        let anchor = anchor_part
            .map(normalize_anchor_fragment)
            .filter(|value| !value.is_empty());

        if anchor_only && anchor.is_none() {
            return Err(OperationError::InvalidInput(
                "--anchor-only requires a non-empty anchor".into(),
            ));
        }

        let path_matcher = if let Some(path_segment) = path_part {
            Some(PathMatcher::new(root, path_segment)?)
        } else {
            None
        };

        Ok(ReferenceQuery {
            path_matcher,
            anchor,
        })
    }

    fn matches(&self, target: &crate::refactor::graph::LinkTarget) -> bool {
        if let Some(expected_anchor) = &self.anchor {
            match target.anchor.as_ref() {
                Some(anchor) if anchor == expected_anchor => {}
                _ => return false,
            }
        }

        match (&self.path_matcher, target.path.as_ref()) {
            (Some(matcher), Some(path)) => matcher.matches(path),
            (Some(_), None) => false,
            (None, _) => true,
        }
    }
}

enum PathMatcher {
    Exact(String),
    Glob(GlobSet),
}

impl PathMatcher {
    fn new(root: &Path, input: &str) -> Result<Self, OperationError> {
        let cleaned = input.trim();
        if cleaned.is_empty() {
            return Err(OperationError::InvalidInput(
                "path pattern must not be empty".into(),
            ));
        }

        if contains_glob_characters(cleaned) {
            let pattern = cleaned.replace('\\', "/");
            let mut builder = GlobSetBuilder::new();
            builder.add(Glob::new(&pattern).map_err(|err| {
                OperationError::InvalidInput(format!("invalid glob pattern '{pattern}': {err}"))
            })?);
            let glob = builder
                .build()
                .map_err(|err| OperationError::InvalidInput(err.to_string()))?;
            Ok(PathMatcher::Glob(glob))
        } else {
            let candidate = normalize_path(root.join(cleaned));
            let relative = candidate.strip_prefix(root).map_err(|_| {
                OperationError::InvalidInput(format!(
                    "pattern '{}' resolves outside project root",
                    input
                ))
            })?;
            Ok(PathMatcher::Exact(path_to_slash(relative)))
        }
    }

    fn matches(&self, path: &Path) -> bool {
        let value = path_to_slash(path);
        match self {
            PathMatcher::Exact(expected) => &value == expected,
            PathMatcher::Glob(glob) => glob.is_match(&value),
        }
    }
}

fn contains_glob_characters(value: &str) -> bool {
    value.contains('*') || value.contains('?') || value.contains('[')
}

fn path_to_slash(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

fn build_catalog_entry(path: &Path, sections: Vec<DocumentSection>) -> CatalogEntry {
    let headings = sections
        .into_iter()
        .map(|section| HeadingSummary {
            level: section.heading.depth,
            text: section.heading.normalized.clone(),
            anchor: section.heading.anchor.clone(),
        })
        .collect();

    CatalogEntry {
        path: normalize_relative_path(path),
        headings,
    }
}

fn normalize_relative_path(path: &Path) -> PathBuf {
    if path.components().next().is_none() {
        PathBuf::from(".")
    } else {
        path.to_path_buf()
    }
}

fn is_markdown_path(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    lower.ends_with(".md") || lower.ends_with(".markdown")
}

/// Catalog execution options.
pub struct CatalogOptions {
    pub scan: ScanOptions,
    pub format: CatalogFormat,
    pub output_path: Option<PathBuf>,
    pub write_to_disk: bool,
}

/// Lint execution options.
pub struct LintOptions {
    pub scan: ScanOptions,
    pub format: LintFormat,
}

/// Validate execution options.
pub struct ValidateOptions {
    pub scan: ScanOptions,
    pub format: ValidateFormat,
    pub schema: Option<String>,
    pub quiet: bool,
}

/// File scanning configuration shared by catalog and lint.
pub struct ScanOptions {
    pub paths: Vec<PathBuf>,
    pub staged: bool,
    pub respect_ignore: bool,
}

impl Default for ScanOptions {
    fn default() -> Self {
        ScanOptions {
            paths: Vec::new(),
            staged: false,
            respect_ignore: true,
        }
    }
}

/// Catalog execution result.
pub struct CatalogOutcome {
    pub rendered: String,
    pub format: CatalogFormat,
    pub entries: Vec<CatalogEntry>,
}

/// Lint execution result containing rendered output and exit code.
pub struct LintOutcome {
    pub rendered: String,
    pub report: LintRenderData,
    pub exit_code: i32,
}

/// Validate execution result containing rendered output and exit code.
pub struct ValidateOutcome {
    pub rendered: String,
    pub report: ValidateRenderData,
    pub exit_code: i32,
}

/// Options for the refs command.
pub struct RefsOptions {
    pub scan: ScanOptions,
    pub pattern: String,
    pub anchor_only: bool,
}

/// Reference lookup result set.
pub struct RefsOutcome {
    pub query: String,
    pub matches: Vec<RefsMatch>,
    pub exit_code: i32,
}

/// Individual reference match.
pub struct RefsMatch {
    pub source: PathBuf,
    pub line: usize,
    pub display: String,
    pub target_path: Option<PathBuf>,
    pub target_anchor: Option<String>,
}

/// Options for the mv command.
pub struct MvOptions {
    pub scan: ScanOptions,
    pub source: PathBuf,
    pub destination: PathBuf,
    pub dry_run: bool,
    pub force: bool,
    pub create_backup: bool,
    pub quiet: bool,
    pub json: bool,
}

/// Result describing the effects of a move/rename operation.
pub struct MvOutcome {
    pub changes: Vec<MvFileChange>,
    pub exit_code: i32,
    pub dry_run: bool,
}

/// Individual file update surfaced by the mv command.
pub struct MvFileChange {
    pub original_path: PathBuf,
    pub output_path: PathBuf,
    pub status: MvFileStatus,
    pub diff: Option<String>,
}

/// Status classification for mv outcomes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MvFileStatus {
    Updated,
    Relocated,
    Unchanged,
}

/// Options for the TOC command.
pub struct TocOptions {
    pub scan: ScanOptions,
    pub mode: TocMode,
    pub quiet: bool,
}

/// Execution result for TOC synchronisation.
pub struct TocOutcome {
    pub rendered: String,
    pub changes: Vec<TocChange>,
    pub exit_code: i32,
}

/// Individual file change surfaced by the TOC command.
pub struct TocChange {
    pub path: PathBuf,
    pub status: TocStatus,
    pub diff: Option<String>,
}

/// Mode selection for TOC execution.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TocMode {
    Check,
    Update,
    Diff,
}

/// Status classification for TOC outcomes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TocStatus {
    UpToDate,
    NeedsUpdate,
    Updated,
    MissingMarkers,
}

/// Errors surfaced while running operations.
#[derive(Debug, Error)]
pub enum OperationError {
    #[error("i/o error accessing {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("git command failed: {message}")]
    Git {
        #[source]
        source: io::Error,
        message: String,
    },
    #[error("schema '{name}' not found")]
    SchemaNotFound { name: String },
    #[error("{0}")]
    InvalidInput(String),
    #[error("rewrite failure: {0}")]
    Rewrite(#[from] RewriteError),
    #[error("{0}")]
    Other(String),
}

impl From<io::Error> for OperationError {
    fn from(source: io::Error) -> Self {
        OperationError::Io {
            path: PathBuf::new(),
            source,
        }
    }
}

impl From<serde_json::Error> for OperationError {
    fn from(source: serde_json::Error) -> Self {
        OperationError::Other(source.to_string())
    }
}
