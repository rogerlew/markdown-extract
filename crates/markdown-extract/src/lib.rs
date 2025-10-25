mod heading;
mod line;
mod state;

pub use heading::{
    detect_heading, normalize_heading_text, HeadingKind, MarkdownHeading, ParsedHeading,
};

use line::{read_lines, LineRecord};
use regex::Regex;
use state::State;
use std::{
    fs::File,
    io::{self, BufReader, Read},
    path::PathBuf,
};

pub type MarkdownSection = Vec<String>;

#[derive(Debug, Clone)]
pub struct SectionSpan {
    pub heading: MarkdownHeading,
    pub lines: Vec<String>,
    pub start: usize,
    pub end: usize,
}

pub fn collect_headings_from_reader<R: Read>(reader: &mut BufReader<R>) -> Vec<ParsedHeading> {
    let lines = read_lines(reader).expect("failed to read markdown input");

    let mut front_matter = FrontMatterState::default();
    let mut code_blocks = CodeBlockTracker::default();
    let mut skip_heading_idx: Option<usize> = None;
    let mut headings = Vec::new();

    for (idx, line) in lines.iter().enumerate() {
        if let Some(skip_idx) = skip_heading_idx {
            if idx <= skip_idx {
                continue;
            }
            skip_heading_idx = None;
        }

        if front_matter.consume(idx, line) {
            continue;
        }

        if code_blocks.process(&line.text) {
            continue;
        }

        if let Some(parsed) = detect_heading(&lines, idx) {
            let end_idx = *parsed.line_range.end();
            if end_idx > idx {
                skip_heading_idx = Some(end_idx);
            }
            headings.push(parsed);
        }
    }

    headings
}

pub fn extract_with_spans_from_path(path: &PathBuf, regex: &Regex) -> io::Result<Vec<SectionSpan>> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    Ok(extract_with_spans_from_reader(&mut reader, regex))
}

pub fn extract_with_spans_from_reader<R: Read>(
    reader: &mut BufReader<R>,
    regex: &Regex,
) -> Vec<SectionSpan> {
    let lines = read_lines(reader).expect("failed to read markdown input");

    let mut state = State::new();
    let mut front_matter = FrontMatterState::default();
    let mut code_blocks = CodeBlockTracker::default();
    let mut skip_heading_idx: Option<usize> = None;
    let mut skip_append_idx: Option<usize> = None;

    for (idx, line) in lines.iter().enumerate() {
        let mut skip_heading = false;
        if let Some(skip_idx) = skip_heading_idx {
            if skip_idx == idx {
                skip_heading = true;
                skip_heading_idx = None;
            } else if skip_idx < idx {
                skip_heading_idx = None;
            }
        }

        let mut line_already_appended = false;
        if let Some(skip_idx) = skip_append_idx {
            if skip_idx == idx {
                line_already_appended = true;
                skip_append_idx = None;
            } else if skip_idx < idx {
                skip_append_idx = None;
            }
        }

        if front_matter.consume(idx, line) {
            continue;
        }

        if code_blocks.process(&line.text) {
            if state.is_within_section() && !line_already_appended {
                state.append_line(line);
            }
            continue;
        }

        let heading = if skip_heading {
            None
        } else {
            detect_heading(&lines, idx)
        };

        if let Some(parsed_heading) = heading {
            let heading_depth = parsed_heading.heading.depth;
            let heading_start = parsed_heading.heading.start;
            let line_range = parsed_heading.line_range.clone();
            let end_idx = *line_range.end();

            if let Some(current_depth) = state.current_depth() {
                if heading_depth <= current_depth {
                    state.exit_section(heading_start);
                }
            }

            let matches_pattern = regex.is_match(&parsed_heading.heading.normalized);
            let can_start_new_section = matches_pattern && !state.is_within_section();

            if can_start_new_section {
                state.enter_section(parsed_heading.heading.clone());
                for line_idx in line_range.clone() {
                    if let Some(line) = lines.get(line_idx) {
                        state.append_line(line);
                    }
                }

                line_already_appended = true;

                if end_idx > idx {
                    skip_heading_idx = Some(end_idx);
                    skip_append_idx = Some(end_idx);
                }
            } else if end_idx > idx {
                skip_heading_idx = Some(end_idx);
            }
        }

        if state.is_within_section() && !line_already_appended {
            state.append_line(line);
        }
    }

    let final_offset = lines.last().map(|line| line.end).unwrap_or(0);
    state.finalize(final_offset)
}

pub fn extract_from_path(path: &PathBuf, regex: &Regex) -> io::Result<Vec<MarkdownSection>> {
    let spans = extract_with_spans_from_path(path, regex)?;
    Ok(spans.into_iter().map(|span| span.lines).collect())
}

pub fn extract_from_reader<R: Read>(
    reader: &mut BufReader<R>,
    regex: &Regex,
) -> Vec<MarkdownSection> {
    extract_with_spans_from_reader(reader, regex)
        .into_iter()
        .map(|span| span.lines)
        .collect()
}

#[derive(Default)]
struct FrontMatterState {
    active: bool,
    done: bool,
}

impl FrontMatterState {
    fn consume(&mut self, index: usize, line: &LineRecord) -> bool {
        if self.done {
            return false;
        }

        let trimmed = line.text.trim();

        if index == 0 && trimmed == "---" {
            self.active = true;
            return true;
        }

        if self.active {
            if trimmed == "---" || trimmed == "..." {
                self.active = false;
                self.done = true;
            }
            return true;
        }

        false
    }
}

#[derive(Default)]
struct CodeBlockTracker {
    fenced: Option<FencedBlock>,
    indented_active: bool,
}

#[derive(Clone, Copy)]
struct FencedBlock {
    fence_char: char,
    fence_len: usize,
}

impl CodeBlockTracker {
    fn process(&mut self, line: &str) -> bool {
        if let Some(fence) = self.fenced {
            if is_closing_fence(line, fence) {
                self.fenced = None;
                return true;
            }
            return true;
        }

        if let Some(fence) = detect_fence_start(line) {
            self.fenced = Some(fence);
            return true;
        }

        let is_blank = line.trim().is_empty();
        let is_indented = is_indented_code_line(line);

        if self.indented_active {
            if is_blank {
                self.indented_active = false;
                return true;
            }

            if is_indented {
                return true;
            }

            self.indented_active = false;
            return false;
        }

        if is_indented {
            self.indented_active = true;
            return true;
        }

        false
    }
}

fn detect_fence_start(line: &str) -> Option<FencedBlock> {
    let (indent_width, rest) = split_indent(line);
    if indent_width > 3 {
        return None;
    }

    let mut chars = rest.chars();
    let first = chars.next()?;
    if first != '`' && first != '~' {
        return None;
    }

    let mut count = 1usize;
    for ch in chars {
        if ch == first {
            count += 1;
        } else {
            break;
        }
    }

    if count < 3 {
        return None;
    }

    Some(FencedBlock {
        fence_char: first,
        fence_len: count,
    })
}

fn is_closing_fence(line: &str, fence: FencedBlock) -> bool {
    let (indent_width, rest) = split_indent(line);
    if indent_width > 3 {
        return false;
    }

    let trimmed = rest.trim_end();
    let mut count = 0usize;
    for ch in trimmed.chars() {
        if ch == fence.fence_char {
            count += 1;
        } else {
            return false;
        }
    }

    count >= fence.fence_len
}

fn is_indented_code_line(line: &str) -> bool {
    let mut width = 0usize;
    for ch in line.chars() {
        match ch {
            ' ' => {
                width += 1;
                if width >= 4 {
                    return true;
                }
            }
            '\t' => return true,
            _ => return false,
        }
    }
    false
}

fn split_indent(line: &str) -> (usize, &str) {
    let mut width = 0usize;
    let mut byte_index = 0usize;

    for (idx, ch) in line.char_indices() {
        match ch {
            ' ' => {
                width += 1;
                byte_index = idx + ch.len_utf8();
            }
            '\t' => {
                width += 4;
                byte_index = idx + ch.len_utf8();
            }
            _ => {
                byte_index = idx;
                break;
            }
        }
    }

    if byte_index == 0 {
        (width, line)
    } else {
        (width, &line[byte_index..])
    }
}
