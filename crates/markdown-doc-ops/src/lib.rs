//! High-level operations shared by markdown-doc commands.

mod lint;

use std::ffi::OsStr;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::SystemTime;

use markdown_doc_config::Config;
use markdown_doc_format::{
    CatalogEntry, CatalogFormat, CatalogRenderData, HeadingSummary, LintFormat, LintRenderData,
    Renderer,
};
use markdown_doc_parser::{DocumentSection, ParserContext};
use markdown_doc_utils::atomic_write;
use rayon::prelude::*;
use thiserror::Error;
use walkdir::WalkDir;

/// Primary entry point for catalog and lint operations.
pub struct Operations {
    config: Config,
    parser: ParserContext,
    renderer: Renderer,
}

impl Operations {
    /// Assemble the operation layer from config by wiring parser + renderer.
    pub fn new(config: Config) -> Self {
        let parser = ParserContext::new(config.clone());
        let renderer = Renderer::from_config(config.clone());
        Self {
            config,
            parser,
            renderer,
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
        let schema_provider = NoopSchemaProvider;

        let result = lint::run(lint::LintRunInput {
            config: &self.config,
            parser: &self.parser,
            targets: &targets,
            root: &self.config.project.root,
            schema_provider: &schema_provider,
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
            walk_markdown_files(&root)?
        } else {
            collect_from_paths(&root, &options.paths)?
        };

        candidates.retain(|path| self.parser.is_path_in_scope(path));
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

fn walk_markdown_files(root: &Path) -> Result<Vec<PathBuf>, OperationError> {
    let mut files = Vec::new();
    for entry in WalkDir::new(root) {
        let entry = match entry {
            Ok(entry) => entry,
            Err(err) => {
                let path_hint = err.path().unwrap_or(root).to_path_buf();
                let message = err.to_string();
                let source = err
                    .into_io_error()
                    .unwrap_or_else(|| io::Error::other(message));
                return Err(OperationError::Io {
                    path: path_hint,
                    source,
                });
            }
        };

        if entry.file_type().is_file() {
            let path = entry.path();
            if let Some(name) = path.file_name().and_then(OsStr::to_str) {
                if is_markdown_path(name) {
                    let relative = path.strip_prefix(root).unwrap_or(path);
                    files.push(relative.to_path_buf());
                }
            }
        }
    }
    Ok(files)
}

fn collect_from_paths(root: &Path, paths: &[PathBuf]) -> Result<Vec<PathBuf>, OperationError> {
    let mut results = Vec::new();
    for provided in paths {
        let absolute = if provided.is_absolute() {
            provided.clone()
        } else {
            root.join(provided)
        };

        if absolute.is_dir() {
            let mut nested = walk_markdown_files(&absolute)?;
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

struct NoopSchemaProvider;

impl lint::SchemaProvider for NoopSchemaProvider {}

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

/// File scanning configuration shared by catalog and lint.
#[derive(Default)]
pub struct ScanOptions {
    pub paths: Vec<PathBuf>,
    pub staged: bool,
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
