use crate::heading::MarkdownHeading;
use crate::line::LineRecord;
use crate::SectionSpan;

#[derive(Default)]
pub struct State {
    matches: Vec<SectionSpan>,
    current: Option<SectionBuilder>,
}

impl State {
    pub fn new() -> Self {
        Self {
            matches: Vec::new(),
            current: None,
        }
    }

    pub fn is_within_section(&self) -> bool {
        self.current.is_some()
    }

    pub fn current_depth(&self) -> Option<usize> {
        self.current.as_ref().map(|section| section.heading.depth)
    }

    pub fn enter_section(&mut self, heading: MarkdownHeading) {
        self.current = Some(SectionBuilder::new(heading));
    }

    pub fn append_line(&mut self, line: &LineRecord) {
        if let Some(current) = &mut self.current {
            current.push_line(line);
        }
    }

    pub fn exit_section(&mut self, end_offset: usize) {
        if let Some(mut current) = self.current.take() {
            current.set_end(end_offset);
            self.matches.push(current.into_section());
        }
    }

    pub fn finalize(mut self, end_offset: usize) -> Vec<SectionSpan> {
        if self.current.is_some() {
            self.exit_section(end_offset);
        }
        self.matches
    }
}

struct SectionBuilder {
    heading: MarkdownHeading,
    lines: Vec<String>,
    start: usize,
    end: usize,
}

impl SectionBuilder {
    fn new(heading: MarkdownHeading) -> Self {
        let start = heading.start;
        Self {
            end: start,
            heading,
            lines: Vec::new(),
            start,
        }
    }

    fn push_line(&mut self, line: &LineRecord) {
        self.lines.push(line.text.clone());
        self.end = line.end;
    }

    fn set_end(&mut self, end: usize) {
        self.end = end;
    }

    fn into_section(self) -> SectionSpan {
        SectionSpan {
            heading: self.heading,
            lines: self.lines,
            start: self.start,
            end: self.end,
        }
    }
}
