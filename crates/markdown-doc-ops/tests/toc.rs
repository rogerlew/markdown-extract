use std::fs;
use std::io::Write;
use std::path::PathBuf;

use markdown_doc_config::{Config, LoadOptions};
use markdown_doc_ops::{Operations, ScanOptions, TocMode, TocOptions};
use tempfile::TempDir;

fn write_file(dir: &TempDir, name: &str, contents: &str) {
    let path = dir.path().join(name);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent directory");
    }
    let mut file = fs::File::create(path).expect("create file");
    file.write_all(contents.as_bytes()).expect("write file");
}

#[test]
fn toc_respects_ignore_file() {
    let temp = TempDir::new().expect("tempdir");

    write_file(
        &temp,
        ".markdown-doc.toml",
        r#"
        [lint]
        rules = ["broken-links"]
        "#,
    );

    write_file(&temp, ".markdown-doc-ignore", "ignored.md\n");

    write_file(
        &temp,
        "ignored.md",
        "<!-- toc -->\n- [Old](#old)\n<!-- tocstop -->\n\n## Ignored\n",
    );

    write_file(
        &temp,
        "kept.md",
        "<!-- toc -->\n- [Old](#old)\n<!-- tocstop -->\n\n## Kept\n",
    );

    let working_dir = fs::canonicalize(temp.path()).expect("canonicalize working dir");
    let config =
        Config::load(LoadOptions::default().with_working_dir(&working_dir)).expect("load config");
    let ops = Operations::new(config);

    let options = TocOptions {
        scan: ScanOptions {
            paths: Vec::new(),
            staged: false,
            respect_ignore: true,
        },
        mode: TocMode::Check,
        quiet: false,
    };

    let outcome = ops.toc(options).expect("toc execution with ignore");
    assert!(
        outcome
            .changes
            .iter()
            .any(|change| change.path == PathBuf::from("kept.md")),
        "kept.md should be processed",
    );
    assert!(
        outcome
            .changes
            .iter()
            .all(|change| change.path != PathBuf::from("ignored.md")),
        "ignored.md should be filtered when respect_ignore is true",
    );

    let options = TocOptions {
        scan: ScanOptions {
            paths: Vec::new(),
            staged: false,
            respect_ignore: false,
        },
        mode: TocMode::Check,
        quiet: false,
    };

    let outcome_no_ignore = ops.toc(options).expect("toc execution without ignore");
    assert!(
        outcome_no_ignore
            .changes
            .iter()
            .any(|change| change.path == PathBuf::from("ignored.md")),
        "ignored.md should be processed when respect_ignore is false",
    );
}
