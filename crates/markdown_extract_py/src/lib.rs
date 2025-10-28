use std::convert::TryFrom;
use std::io::{self, BufReader};
use std::path::{Path, PathBuf};

use markdown_bindings_common::{build_regex as build_shared_regex, format_io_error};
use markdown_extract::{
    extract_with_spans_from_path, extract_with_spans_from_reader, HeadingKind, SectionSpan,
};
use pyo3::create_exception;
use pyo3::exceptions::{PyException, PyValueError};
use pyo3::prelude::*;
use regex::Regex;

create_exception!(markdown_extract_py, MarkdownExtractError, PyException);

#[pyclass(module = "markdown_extract_py")]
pub struct Section {
    #[pyo3(get)]
    heading: String,
    #[pyo3(get)]
    level: u8,
    #[pyo3(get)]
    title: String,
    #[pyo3(get)]
    body: String,
    #[pyo3(get)]
    full_text: String,
}

#[pyfunction(signature = (pattern, content, *, case_sensitive=false, all_matches=false, no_heading=false))]
fn extract(
    pattern: &str,
    content: &str,
    case_sensitive: bool,
    all_matches: bool,
    no_heading: bool,
) -> PyResult<Vec<String>> {
    let regex = build_regex(pattern, case_sensitive)?;
    let mut reader = BufReader::new(content.as_bytes());
    let spans = extract_with_spans_from_reader(&mut reader, &regex);
    Ok(convert_spans_to_strings(spans, all_matches, no_heading))
}

#[pyfunction(signature = (pattern, path, *, case_sensitive=false, all_matches=false, no_heading=false))]
fn extract_from_file(
    pattern: &str,
    path: &str,
    case_sensitive: bool,
    all_matches: bool,
    no_heading: bool,
) -> PyResult<Vec<String>> {
    let regex = build_regex(pattern, case_sensitive)?;
    let path_buf = PathBuf::from(path);
    let spans = extract_with_spans_from_path(&path_buf, &regex)
        .map_err(|err| map_io_error(err, &path_buf))?;
    Ok(convert_spans_to_strings(spans, all_matches, no_heading))
}

#[pyfunction(signature = (pattern, content, *, case_sensitive=false, all_matches=false))]
fn extract_sections(
    py: Python,
    pattern: &str,
    content: &str,
    case_sensitive: bool,
    all_matches: bool,
) -> PyResult<Vec<Py<Section>>> {
    let regex = build_regex(pattern, case_sensitive)?;
    let mut reader = BufReader::new(content.as_bytes());
    let spans = extract_with_spans_from_reader(&mut reader, &regex);
    convert_spans_to_py_sections(py, spans, all_matches)
}

#[pyfunction(signature = (pattern, path, *, case_sensitive=false, all_matches=false))]
fn extract_sections_from_file(
    py: Python,
    pattern: &str,
    path: &str,
    case_sensitive: bool,
    all_matches: bool,
) -> PyResult<Vec<Py<Section>>> {
    let regex = build_regex(pattern, case_sensitive)?;
    let path_buf = PathBuf::from(path);
    let spans = extract_with_spans_from_path(&path_buf, &regex)
        .map_err(|err| map_io_error(err, &path_buf))?;
    convert_spans_to_py_sections(py, spans, all_matches)
}

#[pymodule]
fn markdown_extract_py(py: Python, module: &PyModule) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(extract, module)?)?;
    module.add_function(wrap_pyfunction!(extract_from_file, module)?)?;
    module.add_function(wrap_pyfunction!(extract_sections, module)?)?;
    module.add_function(wrap_pyfunction!(extract_sections_from_file, module)?)?;
    module.add(
        "MarkdownExtractError",
        py.get_type::<MarkdownExtractError>(),
    )?;
    module.add_class::<Section>()?;
    Ok(())
}

fn build_regex(pattern: &str, case_sensitive: bool) -> PyResult<Regex> {
    build_shared_regex(pattern, case_sensitive)
        .map_err(|err| PyValueError::new_err(err.to_string()))
}

fn convert_spans_to_strings(
    spans: Vec<SectionSpan>,
    all_matches: bool,
    no_heading: bool,
) -> Vec<String> {
    let mut sections: Vec<String> = spans
        .into_iter()
        .map(|span| span_to_string(&span, no_heading))
        .collect();
    if !all_matches && sections.len() > 1 {
        sections.truncate(1);
    }
    sections
}

fn convert_spans_to_py_sections(
    py: Python,
    spans: Vec<SectionSpan>,
    all_matches: bool,
) -> PyResult<Vec<Py<Section>>> {
    let iter = spans.into_iter();
    let mut sections = Vec::new();
    for span in iter {
        let section = Py::new(py, Section::from_span(&span))?;
        sections.push(section);
        if !all_matches {
            break;
        }
    }
    Ok(sections)
}

impl Section {
    fn from_span(span: &SectionSpan) -> Self {
        let heading_line_count = heading_line_count(span);
        let full_text = join_lines(&span.lines);
        let body = join_lines(&span.lines[heading_line_count.min(span.lines.len())..]);
        let heading = render_heading(span);

        let level = u8::try_from(span.heading.depth).unwrap_or(u8::MAX);

        Section {
            heading,
            level,
            title: span.heading.raw.clone(),
            body,
            full_text,
        }
    }
}

fn span_to_string(span: &SectionSpan, no_heading: bool) -> String {
    let heading_line_count = if no_heading {
        heading_line_count(span)
    } else {
        0
    };
    let start = heading_line_count.min(span.lines.len());
    join_lines(&span.lines[start..])
}

fn heading_line_count(span: &SectionSpan) -> usize {
    match span.heading.kind {
        HeadingKind::Atx => 1,
        HeadingKind::Setext => 2,
    }
}

fn join_lines(lines: &[String]) -> String {
    lines.join("\n")
}

fn render_heading(span: &SectionSpan) -> String {
    let hashes = "#".repeat(span.heading.depth.max(1));
    if span.heading.raw.is_empty() {
        hashes
    } else {
        format!("{} {}", hashes, span.heading.raw)
    }
}

fn map_io_error(err: io::Error, path: &Path) -> PyErr {
    let message = format_io_error(&err, Some(path));
    MarkdownExtractError::new_err(message)
}
