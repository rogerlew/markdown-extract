use std::fs;
use std::io::Write;
use std::path::Path;

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn setup_file(dir: &Path, relative: &str, contents: &str) {
    let path = dir.join(relative);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent directory");
    }
    let mut file = fs::File::create(&path).expect("create file");
    file.write_all(contents.as_bytes()).expect("write file");
}

#[test]
fn catalog_generates_markdown_and_writes_file() {
    let temp = TempDir::new().expect("tempdir");
    setup_file(
        temp.path(),
        "README.md",
        "# Overview\n\n## Getting Started\n",
    );

    let mut cmd = Command::cargo_bin("markdown-doc").expect("binary");
    let output = cmd
        .current_dir(temp.path())
        .arg("catalog")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8(output).expect("stdout utf8");
    assert!(stdout.contains("Documentation Catalog"));
    assert!(stdout.contains("README.md"));

    let catalog_path = temp.path().join("DOC_CATALOG.md");
    let catalog = fs::read_to_string(&catalog_path).expect("read catalog");
    assert!(catalog.contains("## README.md"));
    assert!(catalog.contains("Getting Started"));
}

#[test]
fn catalog_json_streams_to_stdout_without_file() {
    let temp = TempDir::new().expect("tempdir");
    setup_file(temp.path(), "docs/guide.md", "# Guide\n");

    let mut cmd = Command::cargo_bin("markdown-doc").expect("binary");
    let output = cmd
        .current_dir(temp.path())
        .args(["catalog", "--format", "json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8(output).expect("stdout utf8");
    assert!(stdout.contains("\"files\""));
    assert!(stdout.contains("docs/guide.md"));

    assert!(!temp.path().join("DOC_CATALOG.md").exists());
}

#[test]
fn lint_broken_links_reports_errors() {
    let temp = TempDir::new().expect("tempdir");
    setup_file(
        temp.path(),
        "docs/source.md",
        "# Source\n\nSee [missing](missing.md).\n",
    );

    let mut cmd = Command::cargo_bin("markdown-doc").expect("binary");
    cmd.current_dir(temp.path())
        .args(["lint", "--path", "docs"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("Broken link to 'missing.md'"));
}
