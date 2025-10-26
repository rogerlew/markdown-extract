use markdown_doc_config::{Config, LoadOptions};
use markdown_doc_ops::{Operations, ScanOptions, ValidateOptions};
use markdown_doc_format::ValidateFormat;
use std::path::PathBuf;

#[test]
fn validate_detects_schema_violations() {
    let fixture = PathBuf::from("tests/markdown-doc/schemas");
    let config = Config::load(LoadOptions::default().with_working_dir(&fixture))
        .expect("load config");
    let ops = Operations::new(config);

    let options = ValidateOptions {
        scan: ScanOptions {
            paths: vec![PathBuf::from("invalid.md")],
            staged: false,
            respect_ignore: true,
        },
        format: ValidateFormat::Plain,
        schema: None,
        quiet: false,
    };

    let outcome = ops.validate(options).expect("validate execution");
    assert_eq!(outcome.exit_code, 1);
    assert!(outcome
        .report
        .findings
        .iter()
        .any(|finding| finding.message.contains("Missing required section 'Details'")));
}

#[test]
fn validate_succeeds_for_conformant_document() {
    let fixture = PathBuf::from("tests/markdown-doc/schemas");
    let config = Config::load(LoadOptions::default().with_working_dir(&fixture))
        .expect("load config");
    let ops = Operations::new(config);

    let options = ValidateOptions {
        scan: ScanOptions {
            paths: vec![PathBuf::from("valid.md")],
            staged: false,
            respect_ignore: true,
        },
        format: ValidateFormat::Plain,
        schema: None,
        quiet: false,
    };

    let outcome = ops.validate(options).expect("validate execution");
    assert_eq!(outcome.exit_code, 0);
    assert!(outcome.report.findings.is_empty());
}
