use std::path::Path;

use markdown_doc_config::{Config, LoadOptions};
use markdown_doc_ops::refactor::graph::LinkOccurrence;
use markdown_doc_ops::{Operations, ScanOptions};
use tempfile::TempDir;

fn write_file(base: &TempDir, path: &str, contents: &str) {
    let absolute = base.path().join(path);
    if let Some(parent) = absolute.parent() {
        std::fs::create_dir_all(parent).expect("create parent directories");
    }
    std::fs::write(absolute, contents).expect("write fixture");
}

#[test]
fn link_graph_collects_anchors_links_and_definitions() {
    let temp = TempDir::new().expect("tempdir");

    write_file(
        &temp,
        "guide.md",
        r#"<!-- toc -->
- [Intro](#intro)
<!-- tocstop -->

# Guide

See [Intro](intro.md#overview).
Reference via [FAQ][faq].

[faq]: notes/faq.md#top
"#,
    );

    write_file(
        &temp,
        "intro.md",
        r#"# Intro

## Overview

Return to [Guide](guide.md#guide).
[FAQ][faq] for additional details.

[faq]: notes/faq.md#top
"#,
    );

    write_file(
        &temp,
        "notes/faq.md",
        r#"# FAQ

## Top

Go back to [Intro](../intro.md#overview).
"#,
    );

    let config =
        Config::load(LoadOptions::default().with_working_dir(temp.path())).expect("load config");
    let ops = Operations::new(config);

    let graph = ops
        .link_graph(ScanOptions::default())
        .expect("build link graph");

    let guide_entry = graph
        .file(Path::new("guide.md"))
        .expect("guide entry present");

    assert!(
        guide_entry
            .anchors()
            .iter()
            .any(|anchor| anchor.slug == "guide"),
        "anchors should include slug for top-level heading"
    );

    let guide_links = graph.links_from(Path::new("guide.md"));
    assert!(
        guide_links
            .iter()
            .any(|link| link.raw_target == "intro.md#overview"),
        "guide.md should include inline link to intro section"
    );

    let intro_inbound = graph.links_to(Path::new("intro.md"), Some("overview"));
    let mut intro_sources = intro_inbound
        .iter()
        .map(|occurrence| match occurrence {
            LinkOccurrence::Inline { source, .. } => source.clone(),
            LinkOccurrence::Definition { source, .. } => source.clone(),
        })
        .collect::<Vec<_>>();
    intro_sources.sort();
    assert_eq!(
        intro_sources,
        vec![
            Path::new("guide.md").to_path_buf(),
            Path::new("notes/faq.md").to_path_buf()
        ],
        "intro overview anchor should have inbound links from guide and faq"
    );

    let faq_refs = graph.links_to(Path::new("notes/faq.md"), Some("top"));
    assert!(
        faq_refs
            .iter()
            .any(|occurrence| matches!(occurrence, LinkOccurrence::Definition { source, .. } if source == &Path::new("guide.md").to_path_buf())),
        "faq definition in guide.md should be indexed"
    );
}
