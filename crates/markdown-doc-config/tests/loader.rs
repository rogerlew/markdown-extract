use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use markdown_doc_config::{
    Config, ConfigError, ConfigSourceKind, LintIgnoreRules, LintRule, LoadOptions, Pattern,
    SeverityLevel,
};
use tempfile::TempDir;

fn write_file(path: impl AsRef<Path>, contents: &str) {
    let mut file = fs::File::create(path).expect("create config");
    file.write_all(contents.as_bytes()).expect("write config");
}

fn canonical(path: impl AsRef<Path>) -> PathBuf {
    fs::canonicalize(path).expect("canonicalize path")
}

fn pattern_strings<'a, I>(patterns: I) -> Vec<String>
where
    I: IntoIterator<Item = &'a Pattern>,
{
    patterns
        .into_iter()
        .map(|p| p.original().to_string())
        .collect()
}

#[test]
fn loads_defaults_when_no_files_present() {
    let temp = TempDir::new().expect("tempdir");
    let working_dir = canonical(temp.path());

    let config = Config::load(LoadOptions::default().with_working_dir(working_dir.clone()))
        .expect("load defaults");

    assert_eq!(config.project.root, working_dir);
    assert_eq!(
        config.catalog.output,
        config.project.root.join("DOC_CATALOG.md")
    );
    assert_eq!(
        pattern_strings(config.catalog.include.iter()),
        vec!["**/*.md".to_string()]
    );
    assert_eq!(
        pattern_strings(config.catalog.exclude.iter()),
        vec!["**/node_modules/**".to_string(), "**/vendor/**".to_string()]
    );
    assert!(config.project.exclude.is_empty());
    assert_eq!(config.lint.rules, vec![LintRule::BrokenLinks]);
    assert_eq!(config.lint.max_heading_depth, 4);
    assert!(config.lint.severity.is_empty());
    assert!(config.lint.ignore.is_empty());
    assert_eq!(config.lint.toc.start_marker, "<!-- toc -->");
    assert_eq!(config.lint.toc.end_marker, "<!-- tocstop -->");
    assert_eq!(config.schemas.default_schema, "default");
    assert!(config.schemas.patterns.is_empty());
    let default_schema = config
        .schemas
        .schemas
        .get("default")
        .expect("default schema");
    assert!(default_schema.required_sections.is_empty());
    assert!(default_schema.allow_additional);
    assert!(!default_schema.allow_empty);

    assert_eq!(config.sources.layers.len(), 1);
    assert_eq!(config.sources.layers[0].kind, ConfigSourceKind::Default);
}

#[test]
fn applies_precedence_and_merges_fields() {
    let temp = TempDir::new().expect("tempdir");
    let git_root = canonical(temp.path());
    fs::create_dir(git_root.join(".git")).expect("create .git");

    write_file(
        git_root.join(".markdown-doc.toml"),
        r#"
        [project]
        name = "root"
        exclude = ["**/build/**"]

        [catalog]
        output = "root_catalog.md"

        [lint]
        rules = ["broken-links", "heading-hierarchy"]
        max_heading_depth = 6

        [lint.severity]
        heading-hierarchy = "warning"

        [[lint.ignore]]
        path = "docs/vendor/**"
        rules = ["broken-links"]

        [schemas.default]
        required_sections = ["Intro"]

        [schemas.guide]
        patterns = ["docs/**"]
        required_sections = ["Overview"]
        allow_additional = true
        "#,
    );

    let workspace = git_root.join("workspace");
    fs::create_dir(&workspace).expect("create workspace");

    write_file(
        workspace.join(".markdown-doc.toml"),
        r#"
        [project]
        name = "workspace"

        [catalog]
        output = "local_catalog.md"

        [lint]
        rules = ["broken-links", "duplicate-anchors"]
        toc_start_marker = "<!-- table-of-contents -->"
        toc_end_marker = "<!-- /table-of-contents -->"

        [lint.severity]
        duplicate-anchors = "ignore"

        [[lint.ignore]]
        path = "docs/tmp/**"
        rules = ["duplicate-anchors"]

        [schemas.guide]
        allow_additional = false
        "#,
    );

    let override_path = workspace.join("override.toml");
    write_file(
        &override_path,
        r#"
        [catalog]
        output = "override_catalog.md"

        [lint.severity]
        broken-links = "warning"
        "#,
    );

    let config = Config::load(
        LoadOptions::default()
            .with_working_dir(&workspace)
            .with_override_path(&override_path),
    )
    .expect("load config with precedence");

    assert_eq!(config.project.name.as_deref(), Some("workspace"));
    assert_eq!(config.project.root, canonical(&workspace));
    assert_eq!(
        pattern_strings(config.project.exclude.iter()),
        vec!["**/build/**".to_string()]
    );
    assert_eq!(
        config.catalog.output,
        canonical(&workspace).join("override_catalog.md")
    );
    assert_eq!(
        config.lint.rules,
        vec![LintRule::BrokenLinks, LintRule::DuplicateAnchors]
    );
    assert_eq!(config.lint.max_heading_depth, 6);
    assert_eq!(config.lint.toc.start_marker, "<!-- table-of-contents -->");
    assert_eq!(config.lint.toc.end_marker, "<!-- /table-of-contents -->");

    assert_eq!(
        config.lint.severity.get(&LintRule::BrokenLinks),
        Some(&SeverityLevel::Warning)
    );
    assert_eq!(
        config.lint.severity.get(&LintRule::DuplicateAnchors),
        Some(&SeverityLevel::Ignore)
    );
    assert_eq!(
        config.lint.severity.get(&LintRule::HeadingHierarchy),
        Some(&SeverityLevel::Warning)
    );

    assert_eq!(config.schemas.default_schema, "default");
    let schema_settings = &config.schemas;
    let default_schema = schema_settings
        .schemas
        .get("default")
        .expect("default schema");
    assert_eq!(default_schema.required_sections, vec!["Intro".to_string()]);
    let guide_schema = schema_settings.schemas.get("guide").expect("guide schema");
    assert!(!guide_schema.allow_additional);
    assert_eq!(guide_schema.required_sections, vec!["Overview".to_string()]);
    assert_eq!(schema_settings.patterns.len(), 1);
    assert_eq!(schema_settings.patterns[0].schema, "guide");
    assert_eq!(schema_settings.patterns[0].matcher.original(), "docs/**");

    assert_eq!(config.lint.ignore.len(), 2);
    assert_eq!(
        pattern_strings(config.lint.ignore.iter().map(|entry| &entry.path)),
        vec!["docs/vendor/**".to_string(), "docs/tmp/**".to_string()]
    );

    let kinds: Vec<_> = config
        .sources
        .layers
        .iter()
        .map(|layer| layer.kind)
        .collect();
    assert_eq!(
        kinds,
        vec![
            ConfigSourceKind::Default,
            ConfigSourceKind::GitRoot,
            ConfigSourceKind::Local,
            ConfigSourceKind::Override
        ]
    );
}

#[test]
fn invalid_lint_rule_surfaces_validation_error() {
    let temp = TempDir::new().expect("tempdir");
    let working_dir = canonical(temp.path());
    write_file(
        working_dir.join(".markdown-doc.toml"),
        r#"
        [lint]
        rules = ["broken-links", "unknown-rule"]
        "#,
    );

    let err = Config::load(LoadOptions::default().with_working_dir(&working_dir))
        .expect_err("expected validation failure");

    match err {
        ConfigError::Validation(errors) => {
            let joined = errors.to_string();
            assert!(
                joined.contains("unknown lint rule 'unknown-rule'"),
                "unexpected error output: {joined}"
            );
        }
        other => panic!("unexpected error: {other}"),
    }
}

#[test]
fn invalid_glob_pattern_is_rejected() {
    let temp = TempDir::new().expect("tempdir");
    let working_dir = canonical(temp.path());
    write_file(
        working_dir.join(".markdown-doc.toml"),
        r#"
        [project]
        exclude = ["[["]
        "#,
    );

    let err = Config::load(LoadOptions::default().with_working_dir(&working_dir))
        .expect_err("expected validation failure");

    match err {
        ConfigError::Validation(errors) => {
            let joined = errors.to_string();
            assert!(
                joined.contains("invalid glob pattern '[['"),
                "unexpected error output: {joined}"
            );
        }
        other => panic!("unexpected error: {other}"),
    }
}

#[test]
fn ignore_entries_require_rules() {
    let temp = TempDir::new().expect("tempdir");
    let working_dir = canonical(temp.path());
    write_file(
        working_dir.join(".markdown-doc.toml"),
        r#"
        [[lint.ignore]]
        path = "docs/**"
        "#,
    );

    let err = Config::load(LoadOptions::default().with_working_dir(&working_dir))
        .expect_err("expected validation failure");

    match err {
        ConfigError::Validation(errors) => {
            let joined = errors.to_string();
            assert!(
                joined.contains("must specify at least one rule"),
                "unexpected error output: {joined}"
            );
        }
        other => panic!("unexpected error: {other}"),
    }
}

#[test]
fn severity_overrides_and_wildcard_resolution() {
    let temp = TempDir::new().expect("tempdir");
    let working_dir = canonical(temp.path());
    write_file(
        working_dir.join(".markdown-doc.toml"),
        r#"
        [lint]
        rules = ["broken-links"]

        [lint.severity]
        "*" = "warning"

        [[lint.ignore]]
        path = "generated/**"
        rules = ["*"]

        [[lint.severity_overrides]]
        path = "docs/**"
        [lint.severity_overrides.rules]
        "*" = "warning"
        broken-links = "error"

        [[lint.severity_overrides]]
        path = "legacy/**"
        [lint.severity_overrides.rules]
        "*" = "ignore"

        [[lint.severity_overrides]]
        path = "docs/**"
        [lint.severity_overrides.rules]
        broken-links = "error"

        "#,
    );

    let config =
        Config::load(LoadOptions::default().with_working_dir(&working_dir)).expect("load config");

    assert_eq!(
        config.lint.severity_wildcard,
        Some(SeverityLevel::Warning),
        "global wildcard severity should be recorded",
    );

    let ignore_entry = config.lint.ignore.first().expect("ignore entry present");
    assert!(matches!(ignore_entry.rules, LintIgnoreRules::All));

    assert_eq!(
        config.lint.severity_for(LintRule::BrokenLinks),
        SeverityLevel::Warning,
        "base severity should fall back to wildcard",
    );

    assert_eq!(
        config
            .lint
            .severity_for_path(Path::new("docs/readme.md"), LintRule::BrokenLinks),
        SeverityLevel::Error,
        "docs override should elevate severity",
    );

    assert_eq!(
        config
            .lint
            .severity_for_path(Path::new("legacy/reference.md"), LintRule::BrokenLinks),
        SeverityLevel::Ignore,
        "legacy override should suppress the rule",
    );

    assert!(
        config.lint.is_rule_enabled(LintRule::BrokenLinks),
        "rule remains enabled because at least one path requires it",
    );
}
