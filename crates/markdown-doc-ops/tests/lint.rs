use std::fs;
use std::io::Write;
use std::path::PathBuf;

use markdown_doc_config::{Config, LintRule, LoadOptions};
use markdown_doc_format::LintFormat;
use markdown_doc_ops::{LintOptions, Operations, ScanOptions};
use tempfile::TempDir;

fn setup_file(dir: &TempDir, name: &str, contents: &str) {
    let path = dir.path().join(name);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent directory");
    }
    let mut file = fs::File::create(path).expect("create file");
    file.write_all(contents.as_bytes()).expect("write file");
}

fn base_config(dir: &TempDir) -> Config {
    Config::load(LoadOptions::default().with_working_dir(dir.path())).expect("load config")
}

fn lint_options(paths: &[&str]) -> LintOptions {
    LintOptions {
        scan: ScanOptions {
            paths: paths.iter().map(PathBuf::from).collect(),
            staged: false,
            respect_ignore: true,
        },
        format: LintFormat::Plain,
    }
}

#[test]
fn broken_anchors_reports_missing_anchor() {
    let temp = TempDir::new().expect("tempdir");
    setup_file(
        &temp,
        "docs/guide.md",
        "# Overview\n\nSee [details](#details).\n",
    );

    let mut config = base_config(&temp);
    config.lint.rules = vec![LintRule::BrokenAnchors];

    let ops = Operations::new(config);
    let outcome = ops
        .lint(lint_options(&["docs/guide.md"]))
        .expect("lint execution");

    assert_eq!(outcome.exit_code, 1);
    assert_eq!(outcome.report.error_count, 1);
    assert!(outcome
        .report
        .findings
        .iter()
        .any(|finding| finding.rule == LintRule::BrokenAnchors));
}

#[test]
fn duplicate_anchors_flagged() {
    let temp = TempDir::new().expect("tempdir");
    setup_file(
        &temp,
        "docs/dupe.md",
        "# Title\n\n## Section\n\n## Section\n",
    );

    let mut config = base_config(&temp);
    config.lint.rules = vec![LintRule::DuplicateAnchors];

    let ops = Operations::new(config);
    let outcome = ops
        .lint(lint_options(&["docs/dupe.md"]))
        .expect("lint execution");

    assert_eq!(outcome.exit_code, 1);
    assert!(outcome.report.error_count >= 1);
    assert!(outcome
        .report
        .findings
        .iter()
        .any(|finding| finding.rule == LintRule::DuplicateAnchors));
}

#[test]
fn heading_hierarchy_detects_skipped_level() {
    let temp = TempDir::new().expect("tempdir");
    setup_file(&temp, "guide.md", "# Title\n\n### Details\n");

    let mut config = base_config(&temp);
    config.lint.rules = vec![LintRule::HeadingHierarchy];

    let ops = Operations::new(config);
    let outcome = ops
        .lint(lint_options(&["guide.md"]))
        .expect("lint execution");

    assert_eq!(outcome.exit_code, 1);
    assert!(outcome
        .report
        .findings
        .iter()
        .any(|finding| finding.rule == LintRule::HeadingHierarchy));
}

#[test]
fn toc_sync_reports_mismatches() {
    let temp = TempDir::new().expect("tempdir");
    setup_file(
        &temp,
        "README.md",
        "<!-- toc -->\n- [Extra](#extra)\n<!-- tocstop -->\n\n# Overview\n\n## Details\n",
    );

    let mut config = base_config(&temp);
    config.lint.rules = vec![LintRule::TocSync];

    let ops = Operations::new(config);
    let outcome = ops
        .lint(lint_options(&["README.md"]))
        .expect("lint execution");

    assert_eq!(outcome.exit_code, 1);
    assert!(outcome
        .report
        .findings
        .iter()
        .any(|finding| finding.rule == LintRule::TocSync));
}

#[test]
fn required_sections_noop_without_schema() {
    let temp = TempDir::new().expect("tempdir");
    setup_file(&temp, "doc.md", "# Title\n");

    let mut config = base_config(&temp);
    config.lint.rules = vec![LintRule::RequiredSections];

    let ops = Operations::new(config);
    let outcome = ops.lint(lint_options(&["doc.md"])).expect("lint execution");

    assert_eq!(outcome.exit_code, 0);
    assert!(outcome.report.findings.is_empty());
}

#[test]
fn required_sections_rule_reports_missing_sections() {
    let temp = TempDir::new().expect("tempdir");
    setup_file(&temp, "docs/guide.md", "# Title\n");

    let mut config = base_config(&temp);
    config.lint.rules = vec![LintRule::RequiredSections];
    if let Some(default_schema) = config.schemas.schemas.get_mut("default") {
        default_schema.required_sections = vec!["Overview".to_string()];
        default_schema.allow_additional = true;
    }

    let ops = Operations::new(config);
    let outcome = ops
        .lint(lint_options(&["docs/guide.md"]))
        .expect("lint execution");

    assert_eq!(outcome.exit_code, 1);
    assert!(outcome.report.findings.iter().any(|finding| finding.rule
        == LintRule::RequiredSections
        && finding
            .message
            .contains("Missing required section 'Overview'")));
}

#[test]
fn per_path_severity_override_reenables_rule() {
    let temp = TempDir::new().expect("tempdir");
    let config_contents = concat!(
        "[lint]\n",
        "rules = [\"broken-links\"]\n\n",
        "[lint.severity]\n",
        "broken-links = \"ignore\"\n\n",
        "[[lint.severity_overrides]]\n",
        "path = \"docs/**\"\n",
        "[lint.severity_overrides.rules]\n",
        "broken-links = \"error\"\n\n",
        "[[lint.severity_overrides]]\n",
        "path = \"legacy/**\"\n",
        "[lint.severity_overrides.rules]\n",
        "\"*\" = \"ignore\"\n",
    );
    setup_file(&temp, ".markdown-doc.toml", config_contents);

    setup_file(
        &temp,
        "docs/source.md",
        "# Title\n\nSee [missing](missing.md).\n",
    );

    let config = base_config(&temp);
    let ops = Operations::new(config);
    let outcome = ops
        .lint(lint_options(&["docs/source.md"]))
        .expect("lint execution");

    assert_eq!(outcome.exit_code, 1, "override should re-enable rule");
    assert!(
        outcome
            .report
            .findings
            .iter()
            .any(|finding| finding.message.contains("missing.md")),
        "expected broken-links finding even though base severity was ignore",
    );
}
