use std::fs;
use std::path::PathBuf;

use markdown_doc_config::{Config, LoadOptions};
use markdown_doc_ops::{MvFileStatus, MvOptions, Operations, ScanOptions};
use tempfile::TempDir;

fn write_file(base: &TempDir, path: &str, contents: &str) {
    let absolute = base.path().join(path);
    if let Some(parent) = absolute.parent() {
        fs::create_dir_all(parent).expect("create parent directory");
    }
    fs::write(absolute, contents).expect("write fixture");
}

fn load_ops(temp: &TempDir) -> Operations {
    let config =
        Config::load(LoadOptions::default().with_working_dir(temp.path())).expect("load config");
    Operations::new(config)
}

#[test]
fn mv_updates_links_and_moves_file() {
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

    write_file(
        &temp,
        "intro.md",
        "# Intro\n\n## Overview\n\nReturn to [Guide](guide.md#guide).\n",
    );

    let ops = load_ops(&temp);

    let outcome = ops
        .mv(MvOptions {
            scan: ScanOptions::default(),
            source: PathBuf::from("intro.md"),
            destination: PathBuf::from("docs/intro.md"),
            dry_run: false,
            force: false,
            create_backup: false,
            quiet: false,
            json: false,
        })
        .expect("mv success");

    assert_eq!(outcome.exit_code, 0);
    assert!(temp.path().join("docs/intro.md").exists());
    assert!(
        !temp.path().join("intro.md").exists(),
        "original file should be relocated"
    );

    let guide = fs::read_to_string(temp.path().join("guide.md")).expect("read guide");
    assert!(
        guide.contains("(docs/intro.md#overview)"),
        "link should point at new location"
    );

    let relocated = outcome
        .changes
        .iter()
        .find(|change| change.original_path == PathBuf::from("intro.md"))
        .expect("rename change present");
    assert_eq!(relocated.status, MvFileStatus::Relocated);
}

#[test]
fn mv_dry_run_emits_diff_without_writing() {
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

    write_file(
        &temp,
        "intro.md",
        "# Intro\n\n## Overview\n\nReturn to [Guide](guide.md#guide).\n",
    );

    let ops = load_ops(&temp);

    let outcome = ops
        .mv(MvOptions {
            scan: ScanOptions::default(),
            source: PathBuf::from("intro.md"),
            destination: PathBuf::from("docs/intro.md"),
            dry_run: true,
            force: false,
            create_backup: false,
            quiet: false,
            json: false,
        })
        .expect("mv dry-run");

    assert!(outcome.dry_run);
    assert!(
        temp.path().join("intro.md").exists(),
        "dry-run must not move files"
    );
    assert!(
        outcome.changes.iter().any(|change| change.diff.is_some()),
        "dry-run should include diff output"
    );
}

#[test]
fn mv_creates_backups_when_requested() {
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

    write_file(
        &temp,
        "intro.md",
        "# Intro\n\n## Overview\n\nReturn to [Guide](guide.md#guide).\n",
    );

    let ops = load_ops(&temp);

    ops.mv(MvOptions {
        scan: ScanOptions::default(),
        source: PathBuf::from("intro.md"),
        destination: PathBuf::from("docs/intro.md"),
        dry_run: false,
        force: false,
        create_backup: true,
        quiet: false,
        json: false,
    })
    .expect("mv with backups");

    assert!(
        temp.path().join("guide.md.bak").exists(),
        "guide backup should exist"
    );
    assert!(
        temp.path().join("intro.md.bak").exists(),
        "source backup should exist"
    );
}

#[test]
fn mv_respects_ignore_filters() {
    let temp = TempDir::new().expect("tempdir");

    write_file(
        &temp,
        ".markdown-doc.toml",
        r#"
        [lint]
        rules = ["broken-links"]
        "#,
    );
    write_file(&temp, ".markdown-doc-ignore", "guide.md\n");

    write_file(
        &temp,
        "guide.md",
        "# Guide\n\nSee [Intro](intro.md#overview).\n",
    );
    write_file(
        &temp,
        "intro.md",
        "# Intro\n\n## Overview\n\nReturn to [Guide](guide.md#guide).\n",
    );

    let ops = load_ops(&temp);

    ops.mv(MvOptions {
        scan: ScanOptions::default(),
        source: PathBuf::from("intro.md"),
        destination: PathBuf::from("docs/intro.md"),
        dry_run: false,
        force: false,
        create_backup: false,
        quiet: false,
        json: false,
    })
    .expect("mv with ignore");

    let guide = fs::read_to_string(temp.path().join("guide.md")).expect("read guide");
    assert!(
        guide.contains("(intro.md#overview)"),
        "ignored file should remain untouched"
    );

    // Restore original layout then run without ignore filtering and expect rewrite.
    fs::rename(
        temp.path().join("docs/intro.md"),
        temp.path().join("intro.md"),
    )
    .expect("restore intro.md");

    ops.mv(MvOptions {
        scan: ScanOptions {
            paths: Vec::new(),
            staged: false,
            respect_ignore: false,
        },
        source: PathBuf::from("intro.md"),
        destination: PathBuf::from("docs/intro.md"),
        dry_run: false,
        force: false,
        create_backup: false,
        quiet: false,
        json: false,
    })
    .expect("mv no-ignore");

    let guide = fs::read_to_string(temp.path().join("guide.md")).expect("read guide");
    assert!(
        guide.contains("(docs/intro.md#overview)"),
        "guide should be updated when ignores are disabled"
    );
}
