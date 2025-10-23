use std::io::BufReader;
use std::path::PathBuf;

use regex::Regex;

use crate::diff::build_unified_diff;
use crate::error::{EditError, EditResult, ExitCode};
use crate::fs::write_atomic;
use crate::payload::{load_payload, PayloadSource};
use crate::section::{
    section_slice, split_section_header, MatchedSection, SectionEdit, SectionTree,
};
use crate::MarkdownHeading;
use markdown_extract::collect_headings_from_reader;

#[derive(Debug, Clone)]
pub struct EditOptions {
    pub allow_duplicate: bool,
    pub apply_to_all: bool,
    pub max_matches: Option<usize>,
    pub dry_run: bool,
    pub backup: bool,
}

impl Default for EditOptions {
    fn default() -> Self {
        Self {
            allow_duplicate: false,
            apply_to_all: false,
            max_matches: Some(1),
            dry_run: false,
            backup: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct EditRequest {
    pub path: PathBuf,
    pub pattern: Regex,
    pub options: EditOptions,
    pub operation: Operation,
}

#[derive(Debug, Clone)]
pub enum Operation {
    Replace(ReplaceOptions),
    Delete,
    AppendTo(PayloadSource),
    PrependTo(PayloadSource),
    InsertAfter(InsertOptions),
    InsertBefore(InsertOptions),
}

#[derive(Debug, Clone)]
pub struct ReplaceOptions {
    pub payload: PayloadSource,
    pub keep_heading: bool,
}

#[derive(Debug, Clone)]
pub struct InsertOptions {
    pub payload: PayloadSource,
}

#[derive(Debug)]
pub struct EditOutcome {
    pub exit_code: ExitCode,
    pub changed: bool,
    pub diff: Option<String>,
    pub edits: Vec<SectionEdit>,
    pub result: String,
}

pub fn apply_edit(request: EditRequest) -> EditResult<EditOutcome> {
    let content = std::fs::read_to_string(&request.path)?;
    let tree = SectionTree::build(&content, &request.pattern);

    let matches = tree.matched(&request.pattern);

    if matches.is_empty() {
        return Err(EditError::NotFound);
    }

    if !request.options.apply_to_all && matches.len() > 1 {
        return Err(EditError::TooManyMatches {
            max: 1,
            actual: matches.len(),
        });
    }

    if let Some(max) = request.options.max_matches {
        if matches.len() > max {
            return Err(EditError::TooManyMatches {
                max,
                actual: matches.len(),
            });
        }
    }

    let edits = match &request.operation {
        Operation::Replace(opts) => {
            handle_replace(&content, &tree, &matches, opts, &request.options)?
        }
        Operation::Delete => handle_delete(&content, &matches),
        Operation::AppendTo(source) => handle_append_prepend(
            &content,
            &tree,
            &matches,
            source,
            AppendPrepend::Append,
            &request.options,
        )?,
        Operation::PrependTo(source) => handle_append_prepend(
            &content,
            &tree,
            &matches,
            source,
            AppendPrepend::Prepend,
            &request.options,
        )?,
        Operation::InsertAfter(opts) => handle_insert(
            &content,
            &tree,
            &matches,
            opts,
            InsertPosition::After,
            &request.options,
        )?,
        Operation::InsertBefore(opts) => handle_insert(
            &content,
            &tree,
            &matches,
            opts,
            InsertPosition::Before,
            &request.options,
        )?,
    };

    if edits.is_empty() {
        return Ok(EditOutcome {
            exit_code: ExitCode::Success,
            changed: false,
            diff: None,
            edits,
            result: content,
        });
    }

    let result = apply_edits(&content, &edits)?;
    let diff = build_unified_diff(&content, &result, request.path.to_string_lossy().as_ref());

    if !request.options.dry_run {
        write_atomic(&request.path, &result, request.options.backup)?;
    }

    Ok(EditOutcome {
        exit_code: ExitCode::Success,
        changed: true,
        diff,
        edits,
        result,
    })
}

fn apply_edits(content: &str, edits: &[SectionEdit]) -> EditResult<String> {
    let mut ordered = edits.to_vec();
    ordered.sort_by_key(|edit| edit.range.start);

    let mut rebuilt = String::with_capacity(content.len());
    let mut cursor = 0;

    for edit in ordered {
        if edit.range.start < cursor {
            return Err(EditError::Validation("overlapping edits".to_string()));
        }

        rebuilt.push_str(&content[cursor..edit.range.start]);
        rebuilt.push_str(&edit.replacement);
        cursor = edit.range.end;
    }

    rebuilt.push_str(&content[cursor..]);

    Ok(rebuilt)
}

fn handle_delete(content: &str, matches: &[MatchedSection]) -> Vec<SectionEdit> {
    matches
        .iter()
        .map(|matched| {
            let section = matched.section();
            let original = section_slice(content, section).to_string();
            SectionEdit {
                range: section.start..section.end,
                original,
                replacement: String::new(),
                heading: section.heading.clone(),
            }
        })
        .collect()
}

enum AppendPrepend {
    Append,
    Prepend,
}

fn handle_append_prepend(
    content: &str,
    tree: &SectionTree,
    matches: &[MatchedSection],
    source: &PayloadSource,
    options: AppendPrepend,
    edit_options: &EditOptions,
) -> EditResult<Vec<SectionEdit>> {
    let payload = load_payload(source.clone())?;
    let normalized_payload = normalize_body_block(&payload);

    let mut edits = Vec::new();

    for matched in matches {
        let section = matched.section();
        let slice = section_slice(content, section);
        let (header, body) = split_section_header(content, section);

        let new_body = match options {
            AppendPrepend::Append => {
                if !edit_options.allow_duplicate && body_contains_suffix(body, &normalized_payload)
                {
                    continue;
                }
                append_body(body, &normalized_payload)
            }
            AppendPrepend::Prepend => {
                if !edit_options.allow_duplicate && body_contains_prefix(body, &normalized_payload)
                {
                    continue;
                }
                prepend_body(body, &normalized_payload)
            }
        };

        let ensure_blank_line = tree.next_section(matched.index()).is_some();
        let replacement = build_section_string(header, &new_body, ensure_blank_line);

        edits.push(SectionEdit {
            range: section.start..section.end,
            original: slice.to_string(),
            replacement,
            heading: section.heading.clone(),
        });
    }

    Ok(edits)
}

fn build_section_string(header: &str, body: &str, ensure_blank_line: bool) -> String {
    let mut section = String::new();
    section.push_str(header);
    section.push_str(body);
    normalize_section_end(section, ensure_blank_line)
}

fn normalize_body_block(payload: &str) -> String {
    let mut content = payload.trim_end_matches('\n').to_string();
    content.push('\n');
    content
}

fn append_body(existing: &str, payload: &str) -> String {
    let mut output = existing.trim_end_matches('\n').to_string();
    if !output.is_empty() {
        output.push('\n');
    }
    output.push_str(payload);
    output
}

fn prepend_body(existing: &str, payload: &str) -> String {
    let mut output = String::new();
    output.push_str(payload);
    let trimmed_existing = existing.trim_start_matches('\n');
    if !trimmed_existing.is_empty() {
        if !output.ends_with('\n') {
            output.push('\n');
        }
        output.push_str(trimmed_existing);
    }
    output
}

fn body_contains_suffix(body: &str, payload: &str) -> bool {
    let trimmed_body = body.trim_end_matches('\n');
    let trimmed_payload = payload.trim_end_matches('\n');
    trimmed_body.ends_with(trimmed_payload)
}

fn body_contains_prefix(body: &str, payload: &str) -> bool {
    let trimmed_body = body.trim_start_matches('\n');
    let trimmed_payload = payload.trim_start_matches('\n');
    trimmed_body.starts_with(trimmed_payload)
}

fn normalize_section_end(mut section: String, ensure_blank_line: bool) -> String {
    while section.ends_with('\n') {
        section.pop();
    }

    section.push('\n');
    if ensure_blank_line {
        section.push('\n');
    }

    section
}

fn handle_replace(
    content: &str,
    tree: &SectionTree,
    matches: &[MatchedSection],
    options: &ReplaceOptions,
    edit_options: &EditOptions,
) -> EditResult<Vec<SectionEdit>> {
    let payload_raw = load_payload(options.payload.clone())?;

    let mut edits = Vec::new();

    if options.keep_heading {
        let normalized_body = normalize_body_block(&payload_raw);

        for matched in matches {
            let section = matched.section();
            let (header, body) = split_section_header(content, section);

            let current_body = normalize_body_block(body);
            if !edit_options.allow_duplicate && current_body == normalized_body {
                continue;
            }

            let ensure_blank_line = tree.next_section(matched.index()).is_some();
            let replacement = build_section_string(header, &normalized_body, ensure_blank_line);

            edits.push(SectionEdit {
                range: section.start..section.end,
                original: section_slice(content, section).to_string(),
                replacement,
                heading: section.heading.clone(),
            });
        }

        return Ok(edits);
    }

    let mut payload_reader = BufReader::new(std::io::Cursor::new(payload_raw.as_bytes()));
    let parsed_headings = collect_headings_from_reader(&mut payload_reader);
    if parsed_headings.is_empty() {
        return Err(EditError::Validation(
            "replacement payload must contain a heading".to_string(),
        ));
    }

    let first_heading = &parsed_headings[0].heading;
    if first_heading.start != 0 {
        return Err(EditError::Validation(
            "replacement payload must begin with a heading".to_string(),
        ));
    }

    for extra in parsed_headings.iter().skip(1) {
        if extra.heading.depth <= first_heading.depth {
            return Err(EditError::Validation(
                "payload contains multiple top-level headings".to_string(),
            ));
        }
    }

    let new_heading = first_heading.clone();
    let payload_base = payload_raw.trim_end_matches('\n').to_string();

    for matched in matches {
        if matched.heading().depth != new_heading.depth {
            return Err(EditError::Validation(format!(
                "replacement heading depth {} does not match target depth {}",
                new_heading.depth,
                matched.heading().depth
            )));
        }

        ensure_unique_heading(tree, matched.index(), &new_heading, Some(matched.index()))?;

        let ensure_blank_line = tree.next_section(matched.index()).is_some();
        let replacement = normalize_section_end(payload_base.clone(), ensure_blank_line);

        edits.push(SectionEdit {
            range: matched.section().start..matched.section().end,
            original: section_slice(content, matched.section()).to_string(),
            replacement,
            heading: new_heading.clone(),
        });
    }

    Ok(edits)
}

fn ensure_unique_heading(
    tree: &SectionTree,
    index: usize,
    heading: &MarkdownHeading,
    exclude: Option<usize>,
) -> EditResult<()> {
    let parent_doc_idx = tree
        .document_heading_index(index)
        .and_then(|idx| tree.document_headings()[idx].parent);
    let exclude_doc_idx = exclude.and_then(|idx| tree.document_heading_index(idx));
    let normalized = heading.normalized.to_lowercase();

    if has_heading_collision(
        tree,
        parent_doc_idx,
        heading.depth,
        &normalized,
        exclude_doc_idx,
    ) {
        return Err(EditError::Validation(format!(
            "heading '{}' already exists at this level",
            heading.normalized
        )));
    }

    Ok(())
}

#[derive(Clone, Copy, Debug)]
enum InsertPosition {
    Before,
    After,
}

fn handle_insert(
    content: &str,
    tree: &SectionTree,
    matches: &[MatchedSection],
    options: &InsertOptions,
    position: InsertPosition,
    edit_options: &EditOptions,
) -> EditResult<Vec<SectionEdit>> {
    let payload_raw = load_payload(options.payload.clone())?;
    let mut payload_reader = BufReader::new(std::io::Cursor::new(payload_raw.as_bytes()));
    let parsed_headings = collect_headings_from_reader(&mut payload_reader);
    if parsed_headings.is_empty() {
        return Err(EditError::Validation(
            "insert payload must begin with a heading".to_string(),
        ));
    }
    let first_heading = &parsed_headings[0].heading;
    if first_heading.start != 0 {
        return Err(EditError::Validation(
            "insert payload must begin with a heading".to_string(),
        ));
    }
    for extra in parsed_headings.iter().skip(1) {
        if extra.heading.depth <= first_heading.depth {
            return Err(EditError::Validation(
                "insert payload contains multiple top-level headings".to_string(),
            ));
        }
    }

    let new_heading = first_heading.clone();
    let payload_base = payload_raw.trim_end_matches('\n').to_string();

    let mut edits = Vec::new();

    for matched in matches {
        let target_heading = matched.heading();

        match position {
            InsertPosition::Before => {
                if new_heading.depth != target_heading.depth {
                    return Err(EditError::Validation(format!(
                        "insert-before payload heading depth {} must match target depth {}",
                        new_heading.depth, target_heading.depth
                    )));
                }
                ensure_heading_collision_for_insert(tree, matched.index(), &new_heading, position)?;

                let ensure_blank_line = true;
                let normalized_payload =
                    normalize_section_end(payload_base.clone(), ensure_blank_line);

                if is_duplicate_insert(
                    content,
                    matched.section().start,
                    &normalized_payload,
                    edit_options.allow_duplicate,
                    InsertPosition::Before,
                ) {
                    continue;
                }

                edits.push(SectionEdit {
                    range: matched.section().start..matched.section().start,
                    original: String::new(),
                    replacement: normalized_payload,
                    heading: new_heading.clone(),
                });
            }
            InsertPosition::After => {
                if new_heading.depth < target_heading.depth {
                    return Err(EditError::Validation(format!(
                        "insert-after payload heading depth {} cannot be higher-level than target depth {}",
                        new_heading.depth, target_heading.depth
                    )));
                }
                ensure_heading_collision_for_insert(tree, matched.index(), &new_heading, position)?;

                let ensure_blank_line = tree.next_section(matched.index()).is_some();
                let normalized_payload =
                    normalize_section_end(payload_base.clone(), ensure_blank_line);

                if is_duplicate_insert(
                    content,
                    matched.section().end,
                    &normalized_payload,
                    edit_options.allow_duplicate,
                    InsertPosition::After,
                ) {
                    continue;
                }

                edits.push(SectionEdit {
                    range: matched.section().end..matched.section().end,
                    original: String::new(),
                    replacement: normalized_payload,
                    heading: new_heading.clone(),
                });
            }
        }
    }

    Ok(edits)
}

fn ensure_heading_collision_for_insert(
    tree: &SectionTree,
    index: usize,
    heading: &MarkdownHeading,
    position: InsertPosition,
) -> EditResult<()> {
    let target_doc_idx = tree
        .document_heading_index(index)
        .ok_or_else(|| EditError::Validation("unable to locate target heading".to_string()))?;
    let target = &tree.document_headings()[target_doc_idx];

    let parent_doc_idx = match position {
        InsertPosition::Before => target.parent,
        InsertPosition::After => {
            if heading.depth == tree.sections()[index].heading.depth {
                target.parent
            } else {
                Some(target_doc_idx)
            }
        }
    };

    let normalized = heading.normalized.to_lowercase();

    if has_heading_collision(tree, parent_doc_idx, heading.depth, &normalized, None) {
        return Err(EditError::Validation(format!(
            "heading '{}' already exists at this level",
            heading.normalized
        )));
    }

    Ok(())
}

fn is_duplicate_insert(
    content: &str,
    offset: usize,
    payload: &str,
    allow_duplicate: bool,
    position: InsertPosition,
) -> bool {
    if allow_duplicate {
        return false;
    }

    let payload_trimmed = payload.trim_matches('\n');
    match position {
        InsertPosition::Before => {
            let prefix = &content[..offset];
            prefix.trim_end().ends_with(payload_trimmed)
        }
        InsertPosition::After => {
            let suffix = &content[offset..];
            suffix.trim_start().starts_with(payload_trimmed)
        }
    }
}

fn has_heading_collision(
    tree: &SectionTree,
    parent_doc_idx: Option<usize>,
    depth: usize,
    normalized: &str,
    exclude_doc_idx: Option<usize>,
) -> bool {
    tree.document_headings()
        .iter()
        .enumerate()
        .any(|(idx, node)| {
            if Some(idx) == exclude_doc_idx {
                return false;
            }

            node.parent == parent_doc_idx
                && node.heading.depth == depth
                && node.heading.normalized.to_lowercase() == normalized
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::payload::PayloadSource;
    use regex::RegexBuilder;
    use tempfile::tempdir;

    fn mk_regex(pattern: &str) -> Regex {
        RegexBuilder::new(pattern)
            .case_insensitive(true)
            .size_limit(1024 * 100)
            .build()
            .unwrap()
    }

    fn write_fixture(contents: &str) -> (tempfile::TempDir, PathBuf) {
        let dir = tempdir().unwrap();
        let path = dir.path().join("doc.md");
        std::fs::write(&path, contents).unwrap();
        (dir, path)
    }

    #[test]
    fn appends_to_section() {
        let (dir, path) = write_fixture("# Heading\n\nBody\n");
        let request = EditRequest {
            path: path.clone(),
            pattern: mk_regex("heading"),
            options: EditOptions {
                dry_run: true,
                ..Default::default()
            },
            operation: Operation::AppendTo(PayloadSource::Inline("New line\n".into())),
        };

        let outcome = apply_edit(request).unwrap();
        assert!(outcome.changed);
        assert!(outcome.result.contains("Body\nNew line\n"));
        drop(dir);
    }
}
