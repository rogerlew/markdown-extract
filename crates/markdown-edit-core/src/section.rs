use std::collections::HashMap;
use std::io::BufReader;
use std::ops::Range;

use markdown_extract::{
    collect_headings_from_reader, extract_with_spans_from_reader, MarkdownHeading, ParsedHeading,
    SectionSpan,
};
use regex::Regex;

#[derive(Debug, Clone)]
pub struct SectionTree {
    sections: Vec<SectionSpan>,
    parents: Vec<Option<usize>>,
    heading_indices: Vec<Option<usize>>,
    document_headings: Vec<DocumentHeading>,
}

#[derive(Debug, Clone)]
pub struct DocumentHeading {
    pub heading: MarkdownHeading,
    pub parent: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct SectionNode<'a> {
    pub index: usize,
    pub section: &'a SectionSpan,
    pub parent: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct MatchedSection<'a> {
    pub node: SectionNode<'a>,
}

#[derive(Debug, Clone)]
pub struct SectionEdit {
    pub range: Range<usize>,
    pub original: String,
    pub replacement: String,
    pub heading: MarkdownHeading,
}

impl SectionTree {
    pub fn build(content: &str, regex: &Regex) -> Self {
        let mut matches_reader = BufReader::new(std::io::Cursor::new(content.as_bytes()));
        let sections = extract_with_spans_from_reader(&mut matches_reader, regex);

        let mut headings_reader = BufReader::new(std::io::Cursor::new(content.as_bytes()));
        let parsed_headings = collect_headings_from_reader(&mut headings_reader);
        let document_headings = build_document_headings(&parsed_headings);
        let (heading_indices, parents) =
            map_sections_to_document_headings(&sections, &document_headings);

        Self {
            sections,
            parents,
            heading_indices,
            document_headings,
        }
    }

    pub fn sections(&self) -> &[SectionSpan] {
        &self.sections
    }

    pub fn len(&self) -> usize {
        self.sections.len()
    }

    pub fn is_empty(&self) -> bool {
        self.sections.is_empty()
    }

    pub fn node(&self, index: usize) -> SectionNode<'_> {
        SectionNode {
            index,
            section: &self.sections[index],
            parent: self.parents[index],
        }
    }

    pub fn matched<'a>(&'a self, _regex: &Regex) -> Vec<MatchedSection<'a>> {
        self.sections
            .iter()
            .enumerate()
            .map(|(index, _)| MatchedSection {
                node: self.node(index),
            })
            .collect()
    }

    pub fn next_section(&self, index: usize) -> Option<&SectionSpan> {
        self.sections.get(index + 1)
    }

    pub fn previous_section(&self, index: usize) -> Option<&SectionSpan> {
        if index == 0 {
            None
        } else {
            self.sections.get(index - 1)
        }
    }

    pub fn document_heading_index(&self, section_index: usize) -> Option<usize> {
        self.heading_indices.get(section_index).and_then(|idx| *idx)
    }

    pub fn document_headings(&self) -> &[DocumentHeading] {
        &self.document_headings
    }
}

impl<'a> MatchedSection<'a> {
    pub fn section(&self) -> &'a SectionSpan {
        self.node.section
    }

    pub fn index(&self) -> usize {
        self.node.index
    }

    pub fn parent(&self) -> Option<usize> {
        self.node.parent
    }

    pub fn depth(&self) -> usize {
        self.section().heading.depth
    }

    pub fn heading(&self) -> &'a MarkdownHeading {
        &self.section().heading
    }
}

fn build_document_headings(parsed: &[ParsedHeading]) -> Vec<DocumentHeading> {
    let mut document_headings = Vec::with_capacity(parsed.len());
    let mut stack: Vec<usize> = Vec::new();

    for (idx, parsed_heading) in parsed.iter().enumerate() {
        let depth = parsed_heading.heading.depth;
        while let Some(&last_idx) = stack.last() {
            let last_depth = parsed[last_idx].heading.depth;
            if last_depth < depth {
                break;
            }
            stack.pop();
        }

        let parent = stack.last().copied();
        document_headings.push(DocumentHeading {
            heading: parsed_heading.heading.clone(),
            parent,
        });
        stack.push(idx);
    }

    document_headings
}

fn map_sections_to_document_headings(
    sections: &[SectionSpan],
    document_headings: &[DocumentHeading],
) -> (Vec<Option<usize>>, Vec<Option<usize>>) {
    let mut heading_lookup: HashMap<usize, usize> = HashMap::new();
    for (idx, heading) in document_headings.iter().enumerate() {
        heading_lookup.insert(heading.heading.start, idx);
    }

    let mut heading_indices = Vec::with_capacity(sections.len());
    for section in sections {
        heading_indices.push(heading_lookup.get(&section.start).copied());
    }

    let mut heading_to_section: HashMap<usize, usize> = HashMap::new();
    for (section_idx, heading_idx_opt) in heading_indices.iter().enumerate() {
        if let Some(heading_idx) = heading_idx_opt {
            heading_to_section.insert(*heading_idx, section_idx);
        }
    }

    let mut parents = Vec::with_capacity(sections.len());
    for heading_idx_opt in &heading_indices {
        if let Some(heading_idx) = heading_idx_opt {
            if let Some(parent_heading_idx) = document_headings[*heading_idx].parent {
                if let Some(section_idx) = heading_to_section.get(&parent_heading_idx) {
                    parents.push(Some(*section_idx));
                } else {
                    parents.push(None);
                }
            } else {
                parents.push(None);
            }
        } else {
            parents.push(None);
        }
    }

    (heading_indices, parents)
}

pub fn section_slice<'a>(content: &'a str, section: &SectionSpan) -> &'a str {
    &content[section.start..section.end]
}

pub fn split_section_header<'a>(content: &'a str, section: &SectionSpan) -> (&'a str, &'a str) {
    let slice = section_slice(content, section);
    let header_len = section
        .heading
        .end
        .saturating_sub(section.start)
        .min(slice.len());
    slice.split_at(header_len)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn document_headings_capture_hierarchy() {
        let markdown = "# A\n\n## B\n\n### C\n\n## D\n";
        let mut reader = BufReader::new(std::io::Cursor::new(markdown.as_bytes()));
        let parsed = collect_headings_from_reader(&mut reader);
        let headings = build_document_headings(&parsed);
        assert_eq!(headings.len(), 4);
        assert_eq!(headings[0].parent, None);
        assert_eq!(headings[1].parent, Some(0));
        assert_eq!(headings[2].parent, Some(1));
        assert_eq!(headings[3].parent, Some(0));
    }
}
