use std::fs;
use std::path::PathBuf;

use markdown_edit_core::ExitCode;
use predicates::prelude::*;
use tempfile::tempdir;

fn cargo_bin() -> assert_cmd::Command {
    assert_cmd::Command::cargo_bin("markdown-edit").unwrap()
}

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

#[test]
fn append_dry_run_prints_diff() {
    let fixture = fixture_path("sample.md");

    let mut cmd = cargo_bin();
    cmd.arg(&fixture)
        .arg("append-to")
        .arg("^Heading One$")
        .arg("--with-string")
        .arg("New line\\n")
        .arg("--dry-run");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("+New line"));
}

#[test]
fn replace_requires_payload() {
    let fixture = fixture_path("sample.md");

    let mut cmd = cargo_bin();
    cmd.arg(&fixture).arg("replace").arg("^Heading One$");

    cmd.assert()
        .failure()
        .code(ExitCode::InvalidArguments as i32)
        .stderr(predicate::str::contains("requires --with"));
}

#[test]
fn delete_not_found_lists_headings() {
    let temp_dir = tempdir().unwrap();
    let source = fixture_path("sample.md");
    let target = temp_dir.path().join("doc.md");
    fs::copy(source, &target).unwrap();

    let mut cmd = cargo_bin();
    cmd.arg(&target).arg("delete").arg("^Missing$");

    cmd.assert()
        .failure()
        .code(ExitCode::NotFound as i32)
        .stderr(predicate::str::contains("Candidate headings"));
}

#[test]
fn append_accepts_hyphen_prefixed_payload() {
    let temp_dir = tempdir().unwrap();
    let source = fixture_path("sample.md");
    let target = temp_dir.path().join("doc.md");
    fs::copy(source, &target).unwrap();

    let mut cmd = cargo_bin();
    cmd.arg(&target)
        .arg("append-to")
        .arg("^Sub Heading$")
        .arg("--with-string")
        .arg("- bullet item\\n")
        .arg("--dry-run");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("+- bullet item"));
}
