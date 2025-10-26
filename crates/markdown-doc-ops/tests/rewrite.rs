use std::collections::HashMap;
use std::path::Path;

use markdown_doc_config::{Config, LoadOptions};
use markdown_doc_ops::refactor::graph::LinkGraph;
use markdown_doc_ops::refactor::rewrite::{plan_file_moves, FileMove};
use markdown_doc_ops::{Operations, ScanOptions};
use tempfile::TempDir;

fn write_file(base: &TempDir, path: &str, contents: &str) {
    let absolute = base.path().join(path);
    if let Some(parent) = absolute.parent() {
        std::fs::create_dir_all(parent).expect("create parent directories");
    }
    std::fs::write(absolute, contents).expect("write fixture");
}

fn build_graph(temp: &TempDir) -> LinkGraph {
    let config =
        Config::load(LoadOptions::default().with_working_dir(temp.path())).expect("load config");
    let ops = Operations::new(config);
    ops.link_graph(ScanOptions::default())
        .expect("build link graph")
}

#[test]
fn plan_file_moves_updates_relative_links() {
    let temp = TempDir::new().expect("tempdir");

    write_file(
        &temp,
        "guide.md",
        r#"# Guide

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
[FAQ][faq] entries cover common topics.

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

    let graph = build_graph(&temp);
    let moves = [FileMove {
        from: Path::new("intro.md").to_path_buf(),
        to: Path::new("docs/intro.md").to_path_buf(),
    }];

    let plan = plan_file_moves(&graph, temp.path(), &moves).expect("compute rewrite plan");

    assert_eq!(plan.moves.len(), 1);

    let edits = plan
        .file_edits
        .into_iter()
        .map(|edit| (edit.original_path.clone(), edit))
        .collect::<HashMap<_, _>>();

    let guide_edit = edits
        .get(Path::new("guide.md"))
        .expect("guide edit present");
    assert_eq!(guide_edit.output_path, Path::new("guide.md"));
    assert!(
        guide_edit
            .updated_contents
            .contains("See [Intro](docs/intro.md#overview)"),
        "guide.md link should reference moved intro.md location"
    );

    let faq_edit = edits
        .get(Path::new("notes/faq.md"))
        .expect("faq edit present");
    assert!(
        faq_edit
            .updated_contents
            .contains("Go back to [Intro](../docs/intro.md#overview)"),
        "faq link should be rewritten relative to new intro.md location"
    );

    let intro_edit = edits
        .get(Path::new("intro.md"))
        .expect("intro edit present");
    assert_eq!(intro_edit.output_path, Path::new("docs/intro.md"));
    assert!(
        intro_edit
            .updated_contents
            .contains("Return to [Guide](../guide.md#guide)"),
        "moved intro.md should update outbound link to guide.md"
    );
    assert!(
        intro_edit
            .updated_contents
            .contains("[faq]: ../notes/faq.md#top"),
        "reference definition should be re-based relative to new intro.md location"
    );
}
