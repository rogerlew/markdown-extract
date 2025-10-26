use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

use markdown_doc_config::{Config, LoadOptions};
use markdown_doc_ops::{Operations, RefsOptions, ScanOptions};
use tempfile::TempDir;

fn write_file(dir: &TempDir, path: &str, contents: &str) {
    let absolute = dir.path().join(path);
    if let Some(parent) = absolute.parent() {
        fs::create_dir_all(parent).expect("create parent directory");
    }
    fs::write(absolute, contents).expect("write file");
}

fn load_ops(temp: &TempDir) -> Operations {
    let config =
        Config::load(LoadOptions::default().with_working_dir(temp.path())).expect("load config");
    Operations::new(config)
}

#[test]
fn refs_finds_matches_for_path_and_anchor() {
    let temp = TempDir::new().expect("tempdir");
    write_file(
        &temp,
        ".markdown-doc.toml",
        r#"
        [lint]
        rules = ["broken-links"]
        "#,
    );

    write_file(
        &temp,
        "docs/guide.md",
        "# Guide\n\n## Overview\n\nSee [FAQ](docs/faq.md#top).\n",
    );

    write_file(
        &temp,
        "docs/faq.md",
        "# FAQ\n\n[Guide](guide.md#overview) explains more.\n",
    );

    write_file(
        &temp,
        "docs/overview.md",
        "# Overview\n\nSee [Guide](guide.md).\n\nSee overview [details](guide.md#overview).\n",
    );

    let ops = load_ops(&temp);

    let outcome = ops
        .refs(RefsOptions {
            scan: ScanOptions::default(),
            pattern: "docs/guide.md#overview".into(),
            anchor_only: false,
        })
        .expect("refs success");

    assert_eq!(outcome.exit_code, 0);
    let sources: HashSet<PathBuf> = outcome.matches.iter().map(|m| m.source.clone()).collect();
    assert!(sources.contains(&PathBuf::from("docs/faq.md")));
    assert!(sources.contains(&PathBuf::from("docs/overview.md")));
}

#[test]
fn refs_anchor_only_matches_slug() {
    let temp = TempDir::new().expect("tempdir");
    write_file(
        &temp,
        ".markdown-doc.toml",
        r#"
        [lint]
        rules = ["broken-links"]
        "#,
    );

    write_file(
        &temp,
        "guide.md",
        "# Guide\n\nSee [Intro](intro.md#overview).\n",
    );

    write_file(&temp, "intro.md", "# Intro\n\n## Overview\n");

    let ops = load_ops(&temp);

    let outcome = ops
        .refs(RefsOptions {
            scan: ScanOptions::default(),
            pattern: "overview".into(),
            anchor_only: true,
        })
        .expect("refs anchor");

    assert_eq!(outcome.exit_code, 0);
    assert!(outcome
        .matches
        .iter()
        .any(|m| m.source == PathBuf::from("guide.md")));
}

#[test]
fn refs_returns_exit_code_one_when_no_matches() {
    let temp = TempDir::new().expect("tempdir");
    write_file(
        &temp,
        ".markdown-doc.toml",
        r#"
        [lint]
        rules = ["broken-links"]
        "#,
    );
    write_file(&temp, "doc.md", "# Title\n\nNo refs.\n");

    let ops = load_ops(&temp);

    let outcome = ops
        .refs(RefsOptions {
            scan: ScanOptions::default(),
            pattern: "docs/missing.md".into(),
            anchor_only: false,
        })
        .expect("refs none");

    assert_eq!(outcome.exit_code, 1);
    assert!(outcome.matches.is_empty());
}
