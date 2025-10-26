use markdown_doc_config::TocSettings;
use markdown_doc_parser::DocumentSection;

use crate::anchors::normalize_anchor_fragment;

#[derive(Clone, Debug)]
pub struct TocEntry {
    pub anchor: String,
    pub text: String,
    pub line: usize,
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct TocBlock {
    pub start_line: usize,
    pub end_line: usize,
    pub start_offset: usize,
    pub end_offset: usize,
    pub entries: Vec<TocEntry>,
}

#[derive(Clone, Debug)]
pub struct GeneratedItem {
    pub indent: usize,
    pub text: String,
    pub anchor: String,
}

#[allow(unused_assignments)]
pub fn locate_block(contents: &str, settings: &TocSettings) -> Option<TocBlock> {
    let mut in_block = false;
    let mut start_line = 0usize;
    let mut start_offset = 0usize;
    let mut entries = Vec::new();

    let mut offset = 0usize;
    let mut line_number = 0usize;

    for segment in contents.split_inclusive('\n') {
        line_number += 1;
        let line_len = segment.len();
        let line_body = segment.trim_end_matches(&['\r', '\n'][..]);
        let trimmed = line_body.trim();

        if !in_block {
            if trimmed == settings.start_marker {
                in_block = true;
                start_line = line_number;
                start_offset = offset + line_len;
            }
        } else {
            if trimmed == settings.end_marker {
                return Some(TocBlock {
                    start_line,
                    end_line: line_number,
                    start_offset,
                    end_offset: offset,
                    entries,
                });
            }
            if let Some(entry) = parse_entry(line_body, line_number) {
                entries.push(entry);
            }
        }

        offset += line_len;
    }

    if offset < contents.len() {
        let line = &contents[offset..];
        if !line.is_empty() {
            line_number += 1;
            let trimmed = line.trim();
            if !in_block {
                if trimmed == settings.start_marker {
                    in_block = true;
                    start_line = line_number;
                    start_offset = offset + line.len();
                }
            } else {
                if trimmed == settings.end_marker {
                    return Some(TocBlock {
                        start_line,
                        end_line: line_number,
                        start_offset,
                        end_offset: offset,
                        entries,
                    });
                }
                if let Some(entry) = parse_entry(line, line_number) {
                    entries.push(entry);
                }
            }
        }
    }

    None
}

fn parse_entry(line: &str, line_number: usize) -> Option<TocEntry> {
    let trimmed_start = line.trim_start();
    if trimmed_start.is_empty() {
        return None;
    }

    let bullet = trimmed_start.chars().next()?;
    if bullet != '-' && bullet != '*' && bullet != '+' {
        return None;
    }

    let after_bullet = trimmed_start[1..].trim_start();
    if !after_bullet.starts_with('[') {
        return None;
    }

    let end_text = after_bullet.find(']')?;
    let text = after_bullet[1..end_text].trim().to_string();
    let remaining = after_bullet[end_text + 1..].trim_start();
    if !remaining.starts_with('(') {
        return None;
    }
    let end_paren = remaining.find(')')?;
    let target = remaining[1..end_paren].trim();
    if !target.starts_with('#') {
        return None;
    }

    let anchor = normalize_anchor_fragment(&target[1..]);

    Some(TocEntry {
        anchor,
        text,
        line: line_number,
    })
}

pub fn generate_items(sections: &[DocumentSection]) -> Vec<GeneratedItem> {
    sections
        .iter()
        .filter_map(|section| {
            let depth = section.heading.depth;
            if depth < 2 {
                return None;
            }
            let indent = depth.saturating_sub(2);
            Some(GeneratedItem {
                indent,
                text: section.heading.raw.trim().to_string(),
                anchor: section.heading.anchor.clone(),
            })
        })
        .collect()
}

#[allow(dead_code)]
pub fn render_items(items: &[GeneratedItem]) -> String {
    render_items_with_separator(items, "\n")
}

pub fn render_items_with_separator(items: &[GeneratedItem], line_sep: &str) -> String {
    if items.is_empty() {
        return String::new();
    }

    let mut output = String::new();
    for item in items {
        let indent = "  ".repeat(item.indent);
        output.push_str(&format!(
            "{indent}- [{}](#{}){}",
            item.text, item.anchor, line_sep
        ));
    }
    output
}
