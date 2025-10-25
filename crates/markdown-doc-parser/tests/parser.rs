use std::fs;
use std::path::Path;

use markdown_doc_config::{Config, LoadOptions};
use markdown_doc_parser::{generate_anchor, ParserContext};
use markdown_extract::HeadingKind;
use tempfile::TempDir;

fn parser_for_dir(dir: &Path) -> Config {
    Config::load(LoadOptions::default().with_working_dir(dir))
        .expect("load config for temp workspace")
}

#[test]
fn parses_sections_with_offsets_and_normalization() {
    let temp = TempDir::new().expect("tempdir");
    let config = parser_for_dir(temp.path());
    let parser = ParserContext::new(config);

    let doc_path = temp.path().join("docs/sample.md");
    fs::create_dir_all(doc_path.parent().unwrap()).expect("create docs/");
    let contents = r#"---
title: Example
---

# Heading *One*

Intro line.

```rust
# not a heading
```

## Second _Heading_

Paragraph.

    ### Indented still code

Setext Title
------------

~~~md
# ignored heading
~~~

### São Tomé & Príncipe

Final line.
"#;

    fs::write(&doc_path, contents).expect("write sample");

    let sections = parser
        .sections_for_path(&doc_path)
        .expect("parse sections from file");
    assert_eq!(sections.len(), 4, "expected four top-level headings");

    let first = &sections[0];
    assert_eq!(first.heading.depth, 1);
    assert_eq!(first.heading.normalized, "Heading One");
    assert_eq!(first.heading.anchor, "heading-one");
    assert_eq!(
        first.byte_range.start,
        contents
            .find("# Heading *One*")
            .expect("find first heading")
    );
    assert!(
        first.body().contains("# not a heading"),
        "code block content should remain inside section"
    );

    let second = &sections[1];
    assert_eq!(second.heading.depth, 2);
    assert_eq!(second.heading.normalized, "Second Heading");
    assert_eq!(second.heading.anchor, "second-heading");

    let third = &sections[2];
    assert_eq!(third.heading.kind, HeadingKind::Setext);
    assert_eq!(third.heading.depth, 2);
    assert_eq!(third.heading.normalized, "Setext Title");

    let fourth = &sections[3];
    assert_eq!(fourth.heading.depth, 3);
    assert_eq!(fourth.heading.normalized, "São Tomé & Príncipe");
    assert_eq!(fourth.heading.anchor, "são-tomé-príncipe");
    assert_eq!(
        fourth.byte_range.start,
        contents
            .find("### São Tomé & Príncipe")
            .expect("find unicode heading")
    );

    assert!(
        fourth.byte_range.end <= contents.len(),
        "section end must not exceed document length"
    );
}

#[test]
fn scope_checks_respect_include_and_exclude_patterns() {
    let temp = TempDir::new().expect("tempdir");
    let config_path = temp.path().join(".markdown-doc.toml");
    fs::write(
        &config_path,
        r#"
        [project]
        exclude = ["skip/**"]

        [catalog]
        include_patterns = ["docs/**/*.md"]
        exclude_patterns = ["**/ignore.md"]
        "#,
    )
    .expect("write config file");

    let config = parser_for_dir(temp.path());
    let parser = ParserContext::new(config);

    let included = temp.path().join("docs/topic.md");
    let excluded_catalog = temp.path().join("docs/ignore.md");
    let outside_include = temp.path().join("notes/readme.md");
    let excluded_project = temp.path().join("skip/doc.md");

    assert!(parser.is_path_in_scope(&included));
    assert!(!parser.is_path_in_scope(&excluded_catalog));
    assert!(!parser.is_path_in_scope(&outside_include));
    assert!(!parser.is_path_in_scope(&excluded_project));

    // Anchor helper remains stable for reuse in other crates.
    assert_eq!(generate_anchor("Heading One"), "heading-one");
}
