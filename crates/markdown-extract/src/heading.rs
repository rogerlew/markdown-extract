use crate::line::LineRecord;
use pulldown_cmark::{Event, Options, Parser};
use std::ops::RangeInclusive;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeadingKind {
    Atx,
    Setext,
}

#[derive(Debug, Clone)]
pub struct MarkdownHeading {
    pub depth: usize,
    pub raw: String,
    pub normalized: String,
    pub start: usize,
    pub end: usize,
    pub kind: HeadingKind,
}

#[derive(Debug, Clone)]
pub struct ParsedHeading {
    pub heading: MarkdownHeading,
    pub line_range: RangeInclusive<usize>,
}

pub fn detect_heading(lines: &[LineRecord], index: usize) -> Option<ParsedHeading> {
    detect_atx_heading(lines, index).or_else(|| detect_setext_heading(lines, index))
}

fn detect_atx_heading(lines: &[LineRecord], index: usize) -> Option<ParsedHeading> {
    let line = lines.get(index)?;
    let trimmed_start = line.text.trim_start();
    let leading_spaces = leading_indent_width(&line.text);
    if leading_spaces > 3 {
        return None;
    }

    let mut pound_count = 0usize;
    for ch in trimmed_start.chars() {
        if ch == '#' {
            pound_count += 1;
        } else {
            break;
        }
    }

    if pound_count == 0 || pound_count > 6 {
        return None;
    }

    let after_hashes = &trimmed_start[pound_count..];
    if !after_hashes.is_empty() && !after_hashes.starts_with(char::is_whitespace) {
        return None;
    }

    let mut content = after_hashes.trim_start().trim_end();
    let stripped_hashes = content.trim_end_matches('#');
    if stripped_hashes.len() < content.len() {
        let candidate = &content[..stripped_hashes.len()];
        if candidate.ends_with(char::is_whitespace) {
            content = candidate.trim_end();
        }
    }

    let raw = content.trim().to_string();
    let normalized = normalize_heading_text(&raw);

    Some(ParsedHeading {
        heading: MarkdownHeading {
            depth: pound_count,
            raw,
            normalized,
            start: line.start,
            end: line.end,
            kind: HeadingKind::Atx,
        },
        line_range: index..=index,
    })
}

fn detect_setext_heading(lines: &[LineRecord], index: usize) -> Option<ParsedHeading> {
    let line = lines.get(index)?;
    let next = lines.get(index + 1)?;

    if leading_indent_width(&line.text) > 3 {
        return None;
    }

    let raw_line = line.text.trim_end();
    if raw_line.trim().is_empty() {
        return None;
    }

    let trimmed_next = next.text.trim();
    if trimmed_next.is_empty() {
        return None;
    }

    let (depth, _) = match_setext_depth(trimmed_next)?;

    // Require that the underline is not indented more than three spaces to avoid code blocks.
    let underline_leading_spaces = leading_indent_width(&next.text);
    if underline_leading_spaces > 3 {
        return None;
    }

    let raw = raw_line.trim().to_string();
    let normalized = normalize_heading_text(&raw);

    Some(ParsedHeading {
        heading: MarkdownHeading {
            depth,
            raw,
            normalized,
            start: line.start,
            end: next.end,
            kind: HeadingKind::Setext,
        },
        line_range: index..=index + 1,
    })
}

fn match_setext_depth(line: &str) -> Option<(usize, char)> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }

    let fence_char = trimmed.chars().next()?;
    if fence_char != '=' && fence_char != '-' {
        return None;
    }

    if !trimmed.chars().all(|ch| ch == fence_char) {
        return None;
    }

    if trimmed.len() < 3 {
        return None;
    }

    let depth = if fence_char == '=' { 1 } else { 2 };
    Some((depth, fence_char))
}

pub fn normalize_heading_text(input: &str) -> String {
    let mut text_segments = Vec::new();
    let parser = Parser::new_ext(input, Options::empty());

    for event in parser {
        match event {
            Event::Text(cow) | Event::Code(cow) => text_segments.push(cow.to_string()),
            Event::SoftBreak | Event::HardBreak => text_segments.push(" ".to_string()),
            Event::FootnoteReference(name) => text_segments.push(name.to_string()),
            Event::Html(_) => {}
            _ => {}
        }
    }

    let normalized = text_segments.join("");
    let mut collapsed = String::new();

    for (idx, segment) in normalized.split_whitespace().enumerate() {
        if idx > 0 {
            collapsed.push(' ');
        }
        collapsed.push_str(segment);
    }

    collapsed
}

fn leading_indent_width(line: &str) -> usize {
    let mut width = 0usize;
    for ch in line.chars() {
        match ch {
            ' ' => width += 1,
            '\t' => width += 4,
            _ => break,
        }
    }
    width
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(text: &str, start: usize, end: usize) -> LineRecord {
        LineRecord {
            text: text.to_string(),
            start,
            end,
        }
    }

    #[test]
    fn parses_atx_heading() {
        let lines = vec![line("### Heading **Text** ##", 0, 23)];
        let parsed = detect_atx_heading(&lines, 0).unwrap();
        assert_eq!(parsed.heading.depth, 3);
        assert_eq!(parsed.heading.raw, "Heading **Text**");
        assert_eq!(parsed.heading.normalized, "Heading Text");
    }

    #[test]
    fn parses_setext_heading() {
        let lines = vec![
            line("Heading with [link](url)", 0, 24),
            line("------", 24, 30),
        ];
        let parsed = detect_setext_heading(&lines, 0).unwrap();
        assert_eq!(parsed.heading.depth, 2);
        assert_eq!(parsed.heading.raw, "Heading with [link](url)");
        assert_eq!(parsed.heading.normalized, "Heading with link");
        assert_eq!(*parsed.line_range.end(), 1);
    }

    #[test]
    fn rejects_invalid_setext_underlines() {
        let lines = vec![line("Heading", 0, 7), line("--=-", 7, 11)];
        assert!(detect_setext_heading(&lines, 0).is_none());
    }
}
