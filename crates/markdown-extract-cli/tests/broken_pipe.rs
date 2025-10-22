use assert_cmd::cargo::cargo_bin;
use std::path::PathBuf;
use std::process::{Command, Stdio};

#[test]
fn exits_successfully_when_downstream_pipe_closes() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::new(cargo_bin("markdown-extract"));
    let markdown = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../markdown-extract/tests/markdown/multiple_matches.md");

    cmd.arg("--all")
        .arg("%%")
        .arg(markdown)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = cmd.spawn()?;
    drop(child.stdout.take());

    let output = child.wait_with_output()?;
    assert!(
        output.status.success(),
        "expected success, got status: {status:?}",
        status = output.status
    );
    assert!(
        output.stderr.is_empty(),
        "expected stderr to be empty, got: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    Ok(())
}
