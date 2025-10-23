use markdown_extract::{extract_from_path, extract_with_spans_from_path, HeadingKind, SectionSpan};
use regex::{Regex, RegexBuilder};
use std::path::PathBuf;

fn create_regex(pattern: &str) -> Regex {
    RegexBuilder::new(pattern)
        .case_insensitive(true)
        .size_limit(1024 * 100) // 100 kb
        .build()
        .unwrap()
}

#[test]
fn should_handle_multiple_matching_sections() {
    // Given
    let path = PathBuf::from(r"tests/markdown/multiple_matches.md");
    let regex = create_regex("^%%");

    // When
    let matches = extract_from_path(&path, &regex).unwrap();

    // Then
    assert_eq!(matches.len(), 2);
    assert_eq!(matches[0][0], "## %% (Heading 1)");
    assert_eq!(matches[1][0], "# %% (Heading 2)");
}

#[test]
fn should_not_match_headings_in_code_blocks() {
    // Given
    let path = PathBuf::from(r"tests/markdown/heading_in_code_block.md");
    let regex = create_regex("^%%");

    // When
    let matches = extract_from_path(&path, &regex).unwrap();

    // Then
    assert_eq!(matches.len(), 0);
}

#[test]
fn should_support_setext_and_normalized_matches() {
    // Given
    let path = PathBuf::from(r"tests/markdown/mixed_headings.md");
    let regex = create_regex("^child heading with link$");

    // When
    let matches = extract_from_path(&path, &regex).unwrap();

    // Then
    assert_eq!(matches.len(), 1);
    assert_eq!(
        matches[0][0],
        "## Child *Heading* with [Link](https://example.com)"
    );
    assert_eq!(matches[0][1], "");
    assert_eq!(matches[0][2], "Content under child heading.");

    // And: Setext headings are matched with normalized text
    let regex_setext = create_regex("^heading two$");
    let matches_setext = extract_from_path(&path, &regex_setext).unwrap();
    assert_eq!(matches_setext.len(), 1);
    assert_eq!(matches_setext[0][0], "Heading Two");
    assert_eq!(matches_setext[0][1], "-----------");
}

#[test]
fn should_ignore_headings_inside_fenced_and_indented_code_blocks() {
    // Given
    let path = PathBuf::from(r"tests/markdown/mixed_headings.md");
    let regex = create_regex("^fenced heading$");
    let regex_tilde = create_regex("^another heading$");
    let regex_indented = create_regex("^indented heading$");

    // When
    let fenced_matches = extract_from_path(&path, &regex).unwrap();
    let tilde_matches = extract_from_path(&path, &regex_tilde).unwrap();
    let indented_matches = extract_from_path(&path, &regex_indented).unwrap();

    // Then
    assert_eq!(fenced_matches.len(), 0);
    assert_eq!(tilde_matches.len(), 0);
    assert_eq!(indented_matches.len(), 0);
}

#[test]
fn should_return_section_spans_with_offsets() {
    // Given
    let path = PathBuf::from(r"tests/markdown/offsets.md");
    let regex = create_regex("heading");

    // When
    let spans = extract_with_spans_from_path(&path, &regex).unwrap();

    // Then
    assert_eq!(spans.len(), 2);
    assert_eq!(spans[0].heading.raw, "First Heading");
    assert_eq!(spans[0].heading.kind, HeadingKind::Atx);
    assert_eq!(spans[1].heading.kind, HeadingKind::Setext);

    let content = std::fs::read_to_string(&path).unwrap();
    assert_span_slice(
        &spans[0],
        &content,
        "## First Heading\nLine one\nLine two\n\n",
    );
    assert_span_slice(
        &spans[1],
        &content,
        "Second Heading\n--------------\nBody line\n",
    );
    assert_eq!(spans[1].lines[0], "Second Heading");
    assert_eq!(spans[1].lines[1], "--------------");
}

fn assert_span_slice(span: &SectionSpan, content: &str, expected: &str) {
    let actual = &content[span.start..span.end];
    assert_eq!(actual, expected);
}
