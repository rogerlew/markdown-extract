//! Rewrite helpers for markdown-doc refactoring operations.

use std::collections::{HashMap, HashSet};
use std::ops::Range;
use std::path::{Path, PathBuf};

use crate::paths::{normalize_path as canonicalize_path, relative_path, split_link_target};
use crate::refactor::graph::{LinkGraph, LinkOccurrence, LinkRecord, ReferenceDefinition};

/// Declaration of a file move (relative to the project root).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FileMove {
    pub from: PathBuf,
    pub to: PathBuf,
}

/// Planned rewrite comprising updated file contents and rename operations.
#[derive(Debug, Clone)]
pub struct RewritePlan {
    pub moves: Vec<FileMove>,
    pub file_edits: Vec<FileEdit>,
}

/// Per-file edit payload capturing the updated contents and text edits applied.
#[derive(Debug, Clone)]
pub struct FileEdit {
    pub original_path: PathBuf,
    pub output_path: PathBuf,
    pub edits: Vec<TextEdit>,
    pub updated_contents: String,
}

/// Atomic text edit applied to a single file.
#[derive(Debug, Clone)]
pub struct TextEdit {
    pub range: Range<usize>,
    pub replacement: String,
}

/// Errors raised while planning rewrite operations.
#[derive(Debug, thiserror::Error)]
pub enum RewriteError {
    #[error("duplicate move declared for {path}")]
    DuplicateMove { path: PathBuf },
    #[error("file '{path}' is not represented in the link graph")]
    MissingFile { path: PathBuf },
    #[error("unable to compute relative path from '{from}' to '{to}'")]
    RelativePathFailure { from: PathBuf, to: PathBuf },
    #[error("conflicting edits for {path} at bytes {range_start}..{range_end}")]
    ConflictingEdit {
        path: PathBuf,
        range_start: usize,
        range_end: usize,
    },
    #[error("invalid edit range for {path} at bytes {range_start}..{range_end}")]
    InvalidEditRange {
        path: PathBuf,
        range_start: usize,
        range_end: usize,
    },
}

/// Plan file move rewrites by analysing the link graph and computing the edits
/// required to keep relative paths accurate.
pub fn plan_file_moves(
    graph: &LinkGraph,
    root: &Path,
    moves: &[FileMove],
) -> Result<RewritePlan, RewriteError> {
    let move_map = build_move_map(moves)?;
    let mut edits: HashMap<PathBuf, Vec<TextEdit>> = HashMap::new();

    for mv in moves {
        let from = canonicalize_path(mv.from.clone());
        // Ensure the moved file is represented in the graph so we can update outbound links.
        let file_entry = graph
            .file(&from)
            .ok_or_else(|| RewriteError::MissingFile { path: from.clone() })?;

        let target_new = resolve_future_path(&from, &move_map);

        // Update inbound references pointing at the moved file.
        for occurrence in graph.links_to(&from, None) {
            plan_inbound_edit(occurrence, &move_map, root, &target_new, &mut edits)?;
        }

        // Update outbound references within the moved file itself.
        plan_outbound_edits(file_entry, &from, &move_map, root, &mut edits)?;

        // Also update reference definitions housed within the moved file.
        plan_definition_edits(file_entry, &from, &move_map, root, &mut edits)?;

        // Ensure the moved file itself is staged for write-out even if no in-file edits were required.
        edits.entry(from.clone()).or_default();
    }

    let mut file_edits = Vec::new();
    for (path, mut file_edits_list) in edits {
        let entry = graph
            .file(&path)
            .ok_or_else(|| RewriteError::MissingFile { path: path.clone() })?;

        if file_edits_list.is_empty() {
            // No content changes; reuse original contents.
            let output_path = resolve_future_path(&path, &move_map);
            file_edits.push(FileEdit {
                original_path: path,
                output_path,
                edits: Vec::new(),
                updated_contents: entry.contents().to_string(),
            });
            continue;
        }

        file_edits_list.sort_by(|a, b| b.range.start.cmp(&a.range.start));
        let updated_contents = apply_edits(entry.contents(), &file_edits_list, &path)?;
        let output_path = resolve_future_path(&path, &move_map);

        file_edits.push(FileEdit {
            original_path: path,
            output_path,
            edits: file_edits_list,
            updated_contents,
        });
    }

    Ok(RewritePlan {
        moves: moves.to_vec(),
        file_edits,
    })
}

fn plan_inbound_edit(
    occurrence: LinkOccurrence,
    move_map: &HashMap<PathBuf, PathBuf>,
    root: &Path,
    target_new: &Path,
    edits: &mut HashMap<PathBuf, Vec<TextEdit>>,
) -> Result<(), RewriteError> {
    match occurrence {
        LinkOccurrence::Inline { source, link } => {
            if let Some(span) = link.destination_span.clone() {
                let replacement =
                    compute_replacement(&source, &link.raw_target, target_new, move_map, root)?;
                record_edit(edits, source, span, replacement)?;
            }
        }
        LinkOccurrence::Definition { source, definition } => {
            if let Some(span) = definition.destination_span.clone() {
                let replacement = compute_replacement(
                    &source,
                    &definition.raw_target,
                    target_new,
                    move_map,
                    root,
                )?;
                record_edit(edits, source, span, replacement)?;
            }
        }
    }

    Ok(())
}

fn plan_outbound_edits(
    entry: &crate::refactor::graph::FileGraphEntry,
    original_path: &Path,
    move_map: &HashMap<PathBuf, PathBuf>,
    root: &Path,
    edits: &mut HashMap<PathBuf, Vec<TextEdit>>,
) -> Result<(), RewriteError> {
    for link in entry.links() {
        let span = match link.destination_span.clone() {
            Some(span) => span,
            None => continue,
        };

        if link.raw_target.starts_with('#') {
            continue;
        }

        let replacement = compute_outbound_replacement(original_path, link, move_map, root)?;

        record_edit(edits, original_path.to_path_buf(), span, replacement)?;
    }

    Ok(())
}

fn plan_definition_edits(
    entry: &crate::refactor::graph::FileGraphEntry,
    original_path: &Path,
    move_map: &HashMap<PathBuf, PathBuf>,
    root: &Path,
    edits: &mut HashMap<PathBuf, Vec<TextEdit>>,
) -> Result<(), RewriteError> {
    for definition in entry.definitions() {
        let span = match definition.destination_span.clone() {
            Some(span) => span,
            None => continue,
        };

        if definition.raw_target.starts_with('#') {
            continue;
        }

        let replacement =
            compute_outbound_definition_replacement(original_path, definition, move_map, root)?;

        record_edit(edits, original_path.to_path_buf(), span, replacement)?;
    }

    Ok(())
}

fn compute_outbound_replacement(
    original_path: &Path,
    link: &LinkRecord,
    move_map: &HashMap<PathBuf, PathBuf>,
    root: &Path,
) -> Result<String, RewriteError> {
    compute_replacement(
        original_path,
        &link.raw_target,
        link.target
            .as_ref()
            .and_then(|target| target.path.as_ref())
            .map(|path| resolve_future_path(path, move_map))
            .as_ref()
            .unwrap_or(&resolve_future_path(original_path, move_map)),
        move_map,
        root,
    )
}

fn compute_outbound_definition_replacement(
    original_path: &Path,
    definition: &ReferenceDefinition,
    move_map: &HashMap<PathBuf, PathBuf>,
    root: &Path,
) -> Result<String, RewriteError> {
    compute_replacement(
        original_path,
        &definition.raw_target,
        definition
            .target
            .as_ref()
            .and_then(|target| target.path.as_ref())
            .map(|path| resolve_future_path(path, move_map))
            .as_ref()
            .unwrap_or(&resolve_future_path(original_path, move_map)),
        move_map,
        root,
    )
}

fn compute_replacement(
    source_path: &Path,
    raw_target: &str,
    future_target: &Path,
    move_map: &HashMap<PathBuf, PathBuf>,
    root: &Path,
) -> Result<String, RewriteError> {
    let (path_part, anchor_part) = split_link_target(raw_target);
    if path_part.is_empty() {
        // Anchor-only links remain valid after renames.
        return Ok(raw_target.to_string());
    }

    let future_source = resolve_future_path(source_path, move_map);
    let source_abs = root.join(&future_source);
    let source_dir = source_abs.parent().unwrap_or(root);
    let target_abs = root.join(future_target);

    let mut path_string = if path_part.starts_with('/') {
        format!("/{}", to_markdown_path(future_target))
    } else {
        match relative_path(source_dir, &target_abs) {
            Some(relative) => to_markdown_path(&relative),
            None => {
                return Err(RewriteError::RelativePathFailure {
                    from: future_source,
                    to: future_target.to_path_buf(),
                })
            }
        }
    };

    if let Some(anchor) = anchor_part {
        path_string.push('#');
        path_string.push_str(anchor);
    }

    Ok(path_string)
}

fn record_edit(
    edits: &mut HashMap<PathBuf, Vec<TextEdit>>,
    path: PathBuf,
    range: Range<usize>,
    replacement: String,
) -> Result<(), RewriteError> {
    let entry = edits.entry(path.clone()).or_default();
    if let Some(existing) = entry.iter_mut().find(|edit| edit.range == range) {
        if existing.replacement != replacement {
            return Err(RewriteError::ConflictingEdit {
                path,
                range_start: range.start,
                range_end: range.end,
            });
        }
        return Ok(());
    }

    entry.push(TextEdit { range, replacement });
    Ok(())
}

fn apply_edits(original: &str, edits: &[TextEdit], path: &Path) -> Result<String, RewriteError> {
    let mut updated = original.to_string();
    for edit in edits {
        if edit.range.end > updated.len() || edit.range.start > edit.range.end {
            return Err(RewriteError::InvalidEditRange {
                path: path.to_path_buf(),
                range_start: edit.range.start,
                range_end: edit.range.end,
            });
        }
        updated.replace_range(edit.range.clone(), &edit.replacement);
    }
    Ok(updated)
}

fn build_move_map(moves: &[FileMove]) -> Result<HashMap<PathBuf, PathBuf>, RewriteError> {
    let mut map = HashMap::new();
    let mut seen = HashSet::new();
    for mv in moves {
        let from = canonicalize_path(mv.from.clone());
        if !seen.insert(from.clone()) {
            return Err(RewriteError::DuplicateMove { path: from });
        }
        map.insert(from, canonicalize_path(mv.to.clone()));
    }
    Ok(map)
}

fn resolve_future_path(path: &Path, moves: &HashMap<PathBuf, PathBuf>) -> PathBuf {
    moves
        .get(path)
        .cloned()
        .unwrap_or_else(|| canonicalize_path(path.to_path_buf()))
}

fn to_markdown_path(path: &Path) -> String {
    let mut components = Vec::new();
    for component in path.components() {
        match component {
            std::path::Component::CurDir => continue,
            other => components.push(other.as_os_str().to_string_lossy().into_owned()),
        }
    }

    if components.is_empty() {
        ".".into()
    } else {
        components.join("/")
    }
}
