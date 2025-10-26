//! Link graph construction utilities for markdown-doc refactoring workflows.

use std::collections::HashMap;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use markdown_doc_parser::ParserContext;
use pulldown_cmark::{Event, LinkType, Options, Parser, Tag};

use crate::anchors::normalize_anchor_fragment;
use crate::lines::{byte_to_line, compute_line_offsets};
use crate::paths::{is_external, resolve_relative_path, split_link_target, ResolvedPath};
use crate::OperationError;

/// Graph describing outbound links and anchors for a collection of Markdown files.
pub struct LinkGraph {
    files: HashMap<PathBuf, FileGraphEntry>,
    backrefs: HashMap<TargetKey, Vec<LinkLocation>>,
}

impl LinkGraph {
    /// Build a link graph for the provided Markdown files relative to `root`.
    pub fn build(
        parser: &ParserContext,
        root: &Path,
        files: &[PathBuf],
    ) -> Result<LinkGraph, OperationError> {
        let mut file_map = HashMap::new();
        let mut backrefs: HashMap<TargetKey, Vec<LinkLocation>> = HashMap::new();

        for relative in files {
            let absolute = root.join(relative);
            let contents =
                std::fs::read_to_string(&absolute).map_err(|source| OperationError::Io {
                    path: absolute.clone(),
                    source,
                })?;

            let contents_arc = Arc::new(contents);
            let line_offsets = Arc::new(compute_line_offsets(&contents_arc));
            let sections = parser.sections_from_str(relative, &contents_arc);

            let anchors = sections
                .iter()
                .map(|section| AnchorRecord {
                    slug: section.heading.anchor.clone(),
                    normalized: section.heading.normalized.clone(),
                    line: byte_to_line(section.heading.byte_range.start, &line_offsets),
                    depth: section.heading.depth,
                    byte_range: section.heading.byte_range.clone(),
                })
                .collect::<Vec<_>>();

            let inline_links =
                collect_inline_links(&contents_arc, &line_offsets, relative, root, &mut backrefs);

            let reference_definitions =
                collect_reference_definitions(&contents_arc, relative, root, &mut backrefs);

            let entry = FileGraphEntry {
                path: relative.clone(),
                contents: contents_arc,
                line_offsets,
                anchors,
                links: inline_links,
                definitions: reference_definitions,
            };
            file_map.insert(relative.clone(), entry);
        }

        Ok(LinkGraph {
            files: file_map,
            backrefs,
        })
    }

    /// Return the graph entry for `path`, if tracked.
    pub fn file(&self, path: &Path) -> Option<&FileGraphEntry> {
        self.files.get(path)
    }

    /// Iterate over all files tracked by the graph.
    pub fn files(&self) -> impl Iterator<Item = &FileGraphEntry> {
        self.files.values()
    }

    /// Collect links originating from `path`.
    pub fn links_from(&self, path: &Path) -> Vec<LinkRecord> {
        self.files
            .get(path)
            .map(|entry| entry.links.clone())
            .unwrap_or_default()
    }

    /// Collect reference definitions originating from `path`.
    pub fn definitions_from(&self, path: &Path) -> Vec<ReferenceDefinition> {
        self.files
            .get(path)
            .map(|entry| entry.definitions.clone())
            .unwrap_or_default()
    }

    /// Retrieve anchors declared within `path`.
    pub fn anchors_in(&self, path: &Path) -> Vec<AnchorRecord> {
        self.files
            .get(path)
            .map(|entry| entry.anchors.clone())
            .unwrap_or_default()
    }

    /// Locate all occurrences that point to `target_path`, optionally filtered by `anchor`.
    pub fn links_to(&self, target_path: &Path, anchor: Option<&str>) -> Vec<LinkOccurrence> {
        let mut results = Vec::new();

        match anchor {
            Some(anchor) => {
                let key = TargetKey::new(target_path, Some(anchor));
                if let Some(locations) = self.backrefs.get(&key) {
                    self.collect_locations(locations, &mut results);
                }
            }
            None => {
                for (key, locations) in &self.backrefs {
                    if key.path == target_path {
                        self.collect_locations(locations, &mut results);
                    }
                }
            }
        }

        results
    }

    fn collect_locations(&self, locations: &[LinkLocation], results: &mut Vec<LinkOccurrence>) {
        for location in locations {
            if let Some(entry) = self.files.get(&location.source) {
                match location.kind {
                    LocationKind::Inline(index) => {
                        if let Some(link) = entry.links.get(index) {
                            results.push(LinkOccurrence::Inline {
                                source: entry.path().to_path_buf(),
                                link: link.clone(),
                            });
                        }
                    }
                    LocationKind::Definition(index) => {
                        if let Some(definition) = entry.definitions.get(index) {
                            results.push(LinkOccurrence::Definition {
                                source: entry.path().to_path_buf(),
                                definition: definition.clone(),
                            });
                        }
                    }
                }
            }
        }
    }
}

/// Per-file graph entry containing anchors and links.
pub struct FileGraphEntry {
    path: PathBuf,
    contents: Arc<String>,
    line_offsets: Arc<Vec<usize>>,
    anchors: Vec<AnchorRecord>,
    links: Vec<LinkRecord>,
    definitions: Vec<ReferenceDefinition>,
}

impl FileGraphEntry {
    /// File path relative to the project root.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Raw Markdown contents.
    pub fn contents(&self) -> &str {
        &self.contents
    }

    /// Cached line offsets for byte → line conversions.
    pub fn line_offsets(&self) -> &[usize] {
        &self.line_offsets
    }

    /// Anchors defined within the file.
    pub fn anchors(&self) -> &[AnchorRecord] {
        &self.anchors
    }

    /// Outbound inline links within the file.
    pub fn links(&self) -> &[LinkRecord] {
        &self.links
    }

    /// Reference-style link definitions within the file.
    pub fn definitions(&self) -> &[ReferenceDefinition] {
        &self.definitions
    }

    /// Retrieve the raw line text for a 1-based line number.
    pub fn line_text(&self, line: usize) -> Option<String> {
        if line == 0 {
            return None;
        }
        let start = *self.line_offsets.get(line.saturating_sub(1))?;
        let end = if line < self.line_offsets.len() {
            *self.line_offsets.get(line)?
        } else {
            self.contents.len()
        };
        if start > self.contents.len() || end > self.contents.len() || start > end {
            return None;
        }
        let slice = &self.contents[start..end];
        Some(slice.trim_end_matches(&['\r', '\n'][..]).to_string())
    }
}

/// Representation of a heading anchor discovered within a file.
#[derive(Clone, Debug)]
pub struct AnchorRecord {
    pub slug: String,
    pub normalized: String,
    pub line: usize,
    pub depth: usize,
    pub byte_range: Range<usize>,
}

/// Normalised link target (path + optional anchor fragment).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct LinkTarget {
    pub path: Option<PathBuf>,
    pub anchor: Option<String>,
}

impl LinkTarget {
    fn key(&self) -> Option<TargetKey> {
        self.path.as_ref().cloned().map(|path| TargetKey {
            path,
            anchor: self.anchor.clone(),
        })
    }
}

/// Inline link record capturing target metadata and destination span.
#[derive(Clone, Debug)]
pub struct LinkRecord {
    pub kind: LinkKind,
    pub line: usize,
    pub raw_target: String,
    pub target: Option<LinkTarget>,
    pub destination_span: Option<Range<usize>>,
    pub needs_angle_brackets: bool,
}

/// Reference-style link definition.
#[derive(Clone, Debug)]
pub struct ReferenceDefinition {
    pub label: String,
    pub line: usize,
    pub raw_target: String,
    pub target: Option<LinkTarget>,
    pub destination_span: Option<Range<usize>>,
    pub needs_angle_brackets: bool,
}

/// Link variants represented in the graph.
#[derive(Clone, Debug)]
pub enum LinkKind {
    Inline,
    Reference,
    Image,
}

/// Graph occurrence referencing either an inline link or a definition.
#[derive(Clone, Debug)]
pub enum LinkOccurrence {
    Inline {
        source: PathBuf,
        link: LinkRecord,
    },
    Definition {
        source: PathBuf,
        definition: ReferenceDefinition,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct TargetKey {
    path: PathBuf,
    anchor: Option<String>,
}

impl TargetKey {
    fn new(path: &Path, anchor: Option<&str>) -> Self {
        let normalized_anchor = anchor.map(normalize_anchor_fragment);
        TargetKey {
            path: path.to_path_buf(),
            anchor: normalized_anchor,
        }
    }
}

#[derive(Clone, Debug)]
struct LinkLocation {
    source: PathBuf,
    kind: LocationKind,
}

#[derive(Clone, Debug)]
enum LocationKind {
    Inline(usize),
    Definition(usize),
}

fn collect_inline_links(
    contents: &Arc<String>,
    line_offsets: &Arc<Vec<usize>>,
    relative: &Path,
    root: &Path,
    backrefs: &mut HashMap<TargetKey, Vec<LinkLocation>>,
) -> Vec<LinkRecord> {
    let mut links = Vec::new();
    let parser = Parser::new_ext(contents, Options::all());

    for (event, range) in parser.into_offset_iter() {
        let (link_type, dest, is_image) = match event {
            Event::Start(Tag::Link(link_type, dest, _)) => (link_type, dest.to_string(), false),
            Event::Start(Tag::Image(link_type, dest, _)) => (link_type, dest.to_string(), true),
            _ => continue,
        };

        if dest.is_empty() || is_external(&dest) {
            continue;
        }

        let (target, needs_angle, dest_span) =
            normalise_target(&dest, link_type, contents, &range, relative, root);

        let kind = if is_image {
            LinkKind::Image
        } else if matches!(
            link_type,
            LinkType::Reference | LinkType::Collapsed | LinkType::Shortcut
        ) {
            LinkKind::Reference
        } else {
            LinkKind::Inline
        };

        if let Some(ref link_target) = target {
            if let Some(key) = link_target.key() {
                backrefs.entry(key).or_default().push(LinkLocation {
                    source: relative.to_path_buf(),
                    kind: LocationKind::Inline(links.len()),
                });
            }
        }

        let line = byte_to_line(range.start, line_offsets);
        links.push(LinkRecord {
            kind,
            line,
            raw_target: dest,
            target,
            destination_span: dest_span,
            needs_angle_brackets: needs_angle,
        });
    }

    links
}

fn collect_reference_definitions(
    contents: &Arc<String>,
    relative: &Path,
    root: &Path,
    backrefs: &mut HashMap<TargetKey, Vec<LinkLocation>>,
) -> Vec<ReferenceDefinition> {
    let mut definitions = Vec::new();
    let mut offset = 0usize;

    for (idx, line) in contents.split_inclusive('\n').enumerate() {
        let length = line.len();
        if let Some((label, dest_info)) = parse_reference_definition(line) {
            let absolute_start = offset + dest_info.offset + dest_info.inner_offset;
            let absolute_end = absolute_start + dest_info.inner_len;
            let cleaned_dest = dest_info.raw;

            if cleaned_dest.is_empty() || is_external(cleaned_dest) {
                offset += length;
                continue;
            }

            let (target, _, _) = normalise_target(
                cleaned_dest,
                LinkType::Inline,
                contents,
                &(absolute_start..absolute_end),
                relative,
                root,
            );

            if let Some(ref link_target) = target {
                if let Some(key) = link_target.key() {
                    backrefs.entry(key).or_default().push(LinkLocation {
                        source: relative.to_path_buf(),
                        kind: LocationKind::Definition(definitions.len()),
                    });
                }
            }

            definitions.push(ReferenceDefinition {
                label: label.to_string(),
                line: idx + 1,
                raw_target: cleaned_dest.to_string(),
                target,
                destination_span: Some(absolute_start..absolute_end),
                needs_angle_brackets: dest_info.needs_angle,
            });
        }
        offset += length;
    }

    definitions
}

fn normalise_target(
    dest: &str,
    link_type: LinkType,
    contents: &str,
    range: &Range<usize>,
    relative: &Path,
    root: &Path,
) -> (Option<LinkTarget>, bool, Option<Range<usize>>) {
    let (path_part, anchor_part) = split_link_target(dest);

    let anchor_normalized = anchor_part.map(normalize_anchor_fragment);
    let mut resolved_path: Option<PathBuf> = None;
    if !path_part.is_empty() {
        let ResolvedPath { relative: rel, .. } = resolve_relative_path(relative, path_part, root);
        resolved_path = Some(rel);
    } else if anchor_normalized.is_some() {
        resolved_path = Some(relative.to_path_buf());
    }

    if resolved_path.is_none() && anchor_normalized.is_none() {
        return (None, false, None);
    }

    let dest_span = match link_type {
        LinkType::Reference | LinkType::Collapsed | LinkType::Shortcut => None,
        _ => destination_span(contents, range),
    };

    let needs_angle = dest_contains_angle(contents, range);

    (
        Some(LinkTarget {
            path: resolved_path,
            anchor: anchor_normalized,
        }),
        needs_angle,
        dest_span,
    )
}

fn destination_span(contents: &str, range: &Range<usize>) -> Option<Range<usize>> {
    let snippet = &contents[range.clone()];
    let bytes = snippet.as_bytes();
    let mut index = 0usize;

    while index < bytes.len() {
        match bytes[index] {
            b'\\' => index += 2,
            b'(' => {
                let mut depth = 1usize;
                let mut cursor = index + 1;
                while cursor < bytes.len() {
                    match bytes[cursor] {
                        b'\\' => cursor += 2,
                        b'(' => {
                            depth += 1;
                            cursor += 1;
                        }
                        b')' => {
                            depth -= 1;
                            if depth == 0 {
                                let inside = &snippet[index + 1..cursor];
                                if let Some((start, len)) = extract_url_bounds(inside) {
                                    let absolute_start = range.start + index + 1 + start;
                                    let absolute_end = absolute_start + len;
                                    return Some(absolute_start..absolute_end);
                                } else {
                                    return None;
                                }
                            }
                            cursor += 1;
                        }
                        _ => cursor += 1,
                    }
                }
                return None;
            }
            _ => index += 1,
        }
    }
    None
}

fn extract_url_bounds(segment: &str) -> Option<(usize, usize)> {
    let trimmed_start = segment.len() - segment.trim_start().len();
    let trimmed = &segment[trimmed_start..];
    if trimmed.is_empty() {
        return None;
    }

    if trimmed.starts_with('<') {
        let close = trimmed.find('>')?;
        if close <= 1 {
            return None;
        }
        let start = trimmed_start + 1;
        let len = close - 1;
        Some((start, len))
    } else {
        let mut end = trimmed.len();
        for (idx, ch) in trimmed.char_indices() {
            if ch.is_whitespace() {
                end = idx;
                break;
            }
        }
        if end == 0 {
            None
        } else {
            Some((trimmed_start, end))
        }
    }
}

fn dest_contains_angle(contents: &str, range: &Range<usize>) -> bool {
    let snippet = &contents[range.clone()];
    if let Some(start) = snippet.find('(') {
        let inside = &snippet[start + 1..];
        let trimmed_start = inside.len() - inside.trim_start().len();
        inside[trimmed_start..].starts_with('<')
    } else {
        false
    }
}

struct DefinitionTarget<'a> {
    raw: &'a str,
    offset: usize,
    inner_offset: usize,
    inner_len: usize,
    needs_angle: bool,
}

fn parse_reference_definition(line: &str) -> Option<(&str, DefinitionTarget<'_>)> {
    let trimmed = line.trim_start();
    if !trimmed.starts_with('[') {
        return None;
    }

    let offset = line.len() - trimmed.len();
    let closing = trimmed.find("]:")?;
    let label = &trimmed[1..closing];
    let remainder = &trimmed[closing + 2..];
    let remainder_trimmed = remainder.trim_start();
    let dest_start_offset = offset + closing + 2 + (remainder.len() - remainder_trimmed.len());

    if remainder_trimmed.is_empty() {
        return None;
    }

    let (inner_offset, inner_len, needs_angle) = extract_definition_bounds(remainder_trimmed)?;

    let raw = &remainder_trimmed[inner_offset..inner_offset + inner_len];
    Some((
        label,
        DefinitionTarget {
            raw,
            offset: dest_start_offset,
            inner_offset,
            inner_len,
            needs_angle,
        },
    ))
}

fn extract_definition_bounds(segment: &str) -> Option<(usize, usize, bool)> {
    if segment.is_empty() {
        return None;
    }

    let trimmed_start = segment.len() - segment.trim_start().len();
    let trimmed = &segment[trimmed_start..];

    if trimmed.is_empty() {
        return None;
    }

    if trimmed.starts_with('<') {
        let close = trimmed.find('>')?;
        if close <= 1 {
            return None;
        }
        let url_offset = trimmed_start + 1;
        let url_len = close - 1;
        Some((url_offset, url_len, true))
    } else {
        let mut end = trimmed.len();
        for (idx, ch) in trimmed.char_indices() {
            if ch.is_whitespace() {
                end = idx;
                break;
            }
        }
        if end == 0 {
            None
        } else {
            Some((trimmed_start, end, false))
        }
    }
}
