use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn reads_from_stdin_when_file_is_dash() {
    let mut cmd = Command::cargo_bin("markdown-extract").unwrap();
    cmd.arg("Target")
        .arg("-")
        .write_stdin("# Title\n\n## Target\nBody\n");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("## Target"));
}
