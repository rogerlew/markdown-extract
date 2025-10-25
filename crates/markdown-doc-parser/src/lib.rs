//! Markdown parsing helpers that expose enriched section spans for the toolkit.
//!
//! The parser bridges the span extraction logic in `markdown-extract` with the
//! higher-level requirements of `markdown-doc`. It resolves configuration-based
//! path filters, normalises headings, generates anchors, and captures byte ranges
//! for every section that begins with a Markdown heading.

use std::io::Cursor;
use std::io::{self, BufReader, Read};
use std::ops::Range;
use std::path::{Path, PathBuf};

use globset::GlobMatcher;
use markdown_doc_config::{Config, PatternList};
use markdown_extract::{collect_headings_from_reader, HeadingKind, MarkdownHeading, ParsedHeading};

pub use markdown_extract::normalize_heading_text;

/// High-level parser context configured with resolved settings.
#[derive(Clone)]
pub struct ParserContext {
    config: Config,
}

impl ParserContext {
    /// Construct a new parser context from the provided configuration.
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    /// Return a reference to the underlying configuration.
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Determine whether the given path should be considered for parsing based on
    /// project and catalog include/exclude patterns.
    pub fn is_path_in_scope(&self, path: &Path) -> bool {
        let absolute = self.absolute_path(path);
        let relative = self.relative_path(&absolute);

        if matches_patterns(&self.config.project.exclude, &relative) {
            return false;
        }

        if matches_patterns(&self.config.catalog.exclude, &relative) {
            return false;
        }

        if !self.config.catalog.include.is_empty()
            && !matches_patterns(&self.config.catalog.include, &relative)
        {
            return false;
        }

        true
    }

    /// Parse Markdown sections from the file at `path`, returning an ordered list
    /// of spans. Paths outside the configured scope return an empty vector.
    pub fn sections_for_path(&self, path: &Path) -> io::Result<Vec<DocumentSection>> {
        if !self.is_path_in_scope(path) {
            return Ok(Vec::new());
        }

        let absolute = self.absolute_path(path);
        let file = std::fs::File::open(&absolute)?;
        let mut reader = BufReader::new(file);
        self.sections_from_reader_internal(absolute, &mut reader)
    }

    /// Parse Markdown sections from an in-memory buffer, primarily for testing.
    pub fn sections_from_str(&self, path: &Path, contents: &str) -> Vec<DocumentSection> {
        if !self.is_path_in_scope(path) {
            return Vec::new();
        }

        let absolute = self.absolute_path(path);
        let cursor = Cursor::new(contents.as_bytes());
        let mut reader = BufReader::new(cursor);
        self.sections_from_reader_internal(absolute, &mut reader)
            .expect("string-backed parsing cannot fail")
    }

    /// Parse Markdown sections from an arbitrary reader, tagging them with the
    /// provided absolute path.
    pub fn sections_from_reader<R: Read>(
        &self,
        absolute_path: PathBuf,
        reader: &mut BufReader<R>,
    ) -> io::Result<Vec<DocumentSection>> {
        if !self.is_path_in_scope(&absolute_path) {
            return Ok(Vec::new());
        }
        self.sections_from_reader_internal(absolute_path, reader)
    }

    fn sections_from_reader_internal<R: Read>(
        &self,
        absolute_path: PathBuf,
        reader: &mut BufReader<R>,
    ) -> io::Result<Vec<DocumentSection>> {
        let mut contents = String::new();
        reader.read_to_string(&mut contents)?;

        if contents.is_empty() {
            return Ok(Vec::new());
        }

        let mut heading_reader = BufReader::new(Cursor::new(contents.as_bytes()));
        let headings = collect_headings_from_reader(&mut heading_reader);
        if headings.is_empty() {
            return Ok(Vec::new());
        }

        let line_records = collect_line_records(&contents);
        let file_len = contents.len();
        let relative = self.relative_path(&absolute_path);

        let sections = build_sections(
            &headings,
            &line_records,
            &absolute_path,
            &relative,
            file_len,
        );
        Ok(sections)
    }

    fn absolute_path(&self, path: &Path) -> PathBuf {
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.config.project.root.join(path)
        }
    }

    fn relative_path(&self, absolute: &Path) -> PathBuf {
        match absolute.strip_prefix(&self.config.project.root) {
            Ok(rel) if !rel.as_os_str().is_empty() => rel.to_path_buf(),
            Ok(_) => PathBuf::from("."),
            Err(_) => absolute.to_path_buf(),
        }
    }
}

/// Canonical heading metadata extracted from a document.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SectionHeading {
    pub depth: usize,
    pub raw: String,
    pub normalized: String,
    pub anchor: String,
    pub kind: HeadingKind,
    pub byte_range: Range<usize>,
}

/// Markdown section enriched with heading details and byte offsets.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DocumentSection {
    pub absolute_path: PathBuf,
    pub relative_path: PathBuf,
    pub heading: SectionHeading,
    pub byte_range: Range<usize>,
    pub lines: Vec<String>,
}

impl DocumentSection {
    /// Return the section body as a contiguous string with newline separators.
    pub fn body(&self) -> String {
        self.lines.join("\n")
    }
}

impl SectionHeading {
    fn from_markdown(heading: &MarkdownHeading) -> Self {
        SectionHeading {
            depth: heading.depth,
            raw: heading.raw.clone(),
            normalized: heading.normalized.clone(),
            anchor: generate_anchor(&heading.normalized),
            kind: heading.kind,
            byte_range: heading.start..heading.end,
        }
    }

    /// Normalize arbitrary heading text using the shared utility.
    pub fn normalize_text(input: &str) -> String {
        normalize_heading_text(input)
    }
}

/// Convert normalized heading text into a stable anchor identifier.
pub fn generate_anchor(normalized: &str) -> String {
    let mut anchor = String::new();
    let mut last_was_dash = false;

    for ch in normalized.chars().flat_map(|c| c.to_lowercase()) {
        if ch.is_alphanumeric() {
            anchor.push(ch);
            last_was_dash = false;
        } else if (ch.is_whitespace() || ch == '-') && !anchor.is_empty() && !last_was_dash {
            anchor.push('-');
            last_was_dash = true;
        }
    }

    if anchor.ends_with('-') {
        anchor.pop();
    }

    anchor
}

fn build_sections(
    headings: &[ParsedHeading],
    lines: &[LineRecord],
    absolute: &Path,
    relative: &Path,
    file_len: usize,
) -> Vec<DocumentSection> {
    headings
        .iter()
        .enumerate()
        .map(|(idx, parsed)| {
            let section_start = parsed.heading.start;
            let section_end = find_section_end(headings, idx, file_len);
            let section_lines = collect_section_lines(lines, section_start, section_end);
            DocumentSection {
                absolute_path: absolute.to_path_buf(),
                relative_path: relative.to_path_buf(),
                heading: SectionHeading::from_markdown(&parsed.heading),
                byte_range: section_start..section_end,
                lines: section_lines,
            }
        })
        .collect()
}

fn find_section_end(headings: &[ParsedHeading], index: usize, file_len: usize) -> usize {
    let current_depth = headings[index].heading.depth;
    for next in headings.iter().skip(index + 1) {
        if next.heading.depth <= current_depth {
            return next.heading.start;
        }
    }
    file_len
}

fn collect_section_lines(lines: &[LineRecord], start: usize, end: usize) -> Vec<String> {
    lines
        .iter()
        .filter(|line| line.start >= start && line.start < end)
        .map(|line| line.text.clone())
        .collect()
}

fn matches_patterns(patterns: &PatternList, path: &Path) -> bool {
    patterns.iter().any(|pattern| {
        let matcher: GlobMatcher = pattern.glob().compile_matcher();
        matcher.is_match(path)
    })
}

#[derive(Debug)]
struct LineRecord {
    text: String,
    start: usize,
}

fn collect_line_records(contents: &str) -> Vec<LineRecord> {
    let mut records = Vec::new();
    let bytes = contents.as_bytes();
    let mut index = 0usize;

    while index < bytes.len() {
        let line_start = index;

        while index < bytes.len() && bytes[index] != b'\n' {
            index += 1;
        }

        let mut text = contents[line_start..index].to_string();

        if index < bytes.len() && bytes[index] == b'\n' {
            index += 1;
        }

        if text.ends_with('\r') {
            text.pop();
        }

        records.push(LineRecord {
            text,
            start: line_start,
        });
    }

    records
}
