use markdown_edit_core::error::EditError;
use markdown_edit_core::payload::PayloadSource;
use markdown_edit_core::{
    apply_edit, EditOptions, EditRequest, InsertOptions, Operation, ReplaceOptions,
};
use regex::RegexBuilder;
use tempfile::tempdir;

fn mk_regex(pattern: &str) -> regex::Regex {
    RegexBuilder::new(pattern)
        .case_insensitive(true)
        .size_limit(1024 * 100)
        .build()
        .unwrap()
}

fn write_fixture(initial: &str) -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempdir().unwrap();
    let path = dir.path().join("doc.md");
    std::fs::write(&path, initial).unwrap();
    (dir, path)
}

#[test]
fn replace_body_only_updates_content() {
    let (dir, path) = write_fixture("## Heading\n\nOld body\n\n");
    let request = EditRequest {
        path: path.clone(),
        pattern: mk_regex("heading"),
        options: EditOptions {
            dry_run: true,
            ..Default::default()
        },
        operation: Operation::Replace(ReplaceOptions {
            payload: PayloadSource::Inline("New body\n".into()),
            keep_heading: true,
        }),
    };

    let outcome = apply_edit(request).unwrap();
    assert!(outcome.changed);
    assert!(outcome.result.contains("New body"));
    assert!(outcome.result.contains("## Heading"));
    drop(dir);
}

#[test]
fn replace_with_new_heading_checks_level() {
    let (dir, path) = write_fixture("## Heading\n\nBody\n\n");
    let request = EditRequest {
        path: path.clone(),
        pattern: mk_regex("heading"),
        options: EditOptions {
            dry_run: true,
            ..Default::default()
        },
        operation: Operation::Replace(ReplaceOptions {
            payload: PayloadSource::Inline("# Wrong\n\nBody\n".into()),
            keep_heading: false,
        }),
    };

    let err = apply_edit(request).unwrap_err();
    assert!(matches!(err, EditError::Validation(_)));
    drop(dir);
}

#[test]
fn delete_removes_section() {
    let (dir, path) = write_fixture("# First\n\nBody\n\n# Second\n\nMore\n\n");
    let request = EditRequest {
        path: path.clone(),
        pattern: mk_regex("second"),
        options: EditOptions {
            dry_run: true,
            ..Default::default()
        },
        operation: Operation::Delete,
    };

    let outcome = apply_edit(request).unwrap();
    assert!(outcome.changed);
    assert!(!outcome.result.contains("Second"));
    drop(dir);
}

#[test]
fn append_duplicate_guard_no_change() {
    let (dir, path) = write_fixture("# Heading\n\nBody\n\n");
    let payload = PayloadSource::Inline("More\n".into());

    let request_append = EditRequest {
        path: path.clone(),
        pattern: mk_regex("heading"),
        options: EditOptions {
            dry_run: false,
            backup: false,
            ..Default::default()
        },
        operation: Operation::AppendTo(payload.clone()),
    };
    let outcome = apply_edit(request_append).unwrap();
    assert!(outcome.changed);

    let second_request = EditRequest {
        path: path.clone(),
        pattern: mk_regex("heading"),
        options: EditOptions {
            dry_run: true,
            ..Default::default()
        },
        operation: Operation::AppendTo(payload),
    };
    let second = apply_edit(second_request).unwrap();
    assert!(!second.changed);
    drop(dir);
}

#[test]
fn insert_after_same_level() {
    let (dir, path) = write_fixture("# First\n\nBody\n\n# Second\n\nMore\n\n");
    let request = EditRequest {
        path: path.clone(),
        pattern: mk_regex("first"),
        options: EditOptions {
            dry_run: true,
            ..Default::default()
        },
        operation: Operation::InsertAfter(InsertOptions {
            payload: PayloadSource::Inline("# Inserted\n\nHi\n".into()),
        }),
    };

    let outcome = apply_edit(request).unwrap();
    assert!(outcome.changed);
    assert!(outcome.result.contains("# Inserted\n\nHi"));
    drop(dir);
}

#[test]
fn insert_before_depth_mismatch_errors() {
    let (dir, path) = write_fixture("## Heading\n\nBody\n\n");
    let request = EditRequest {
        path: path.clone(),
        pattern: mk_regex("heading"),
        options: EditOptions {
            dry_run: true,
            ..Default::default()
        },
        operation: Operation::InsertBefore(InsertOptions {
            payload: PayloadSource::Inline("### Child\n\nText\n".into()),
        }),
    };

    let err = apply_edit(request).unwrap_err();
    assert!(matches!(err, EditError::Validation(_)));
    drop(dir);
}

#[test]
fn max_matches_limit_enforced() {
    let (dir, path) = write_fixture("# A\n\nOne\n\n# B\n\nTwo\n\n");
    let request = EditRequest {
        path: path.clone(),
        pattern: mk_regex("."),
        options: EditOptions {
            max_matches: Some(1),
            apply_to_all: true,
            ..Default::default()
        },
        operation: Operation::Delete,
    };

    let err = apply_edit(request).unwrap_err();
    assert!(matches!(err, EditError::TooManyMatches { .. }));
    drop(dir);
}

#[test]
fn dry_run_produces_diff() {
    let (dir, path) = write_fixture("# Heading\n\nBody\n\n");
    let request = EditRequest {
        path: path.clone(),
        pattern: mk_regex("heading"),
        options: EditOptions {
            dry_run: true,
            ..Default::default()
        },
        operation: Operation::AppendTo(PayloadSource::Inline("Tail\n".into())),
    };

    let outcome = apply_edit(request).unwrap();
    assert!(outcome.diff.is_some());
    drop(dir);
}

#[test]
fn non_ascii_headings_preserved() {
    let (dir, path) = write_fixture("## Café\n\nBody\n\n");
    let request = EditRequest {
        path: path.clone(),
        pattern: mk_regex("café"),
        options: EditOptions {
            dry_run: true,
            ..Default::default()
        },
        operation: Operation::AppendTo(PayloadSource::Inline("Nueva línea\n".into())),
    };

    let outcome = apply_edit(request).unwrap();
    assert!(outcome.result.contains("## Café"));
    assert!(outcome.result.contains("Nueva línea"));
    drop(dir);
}

#[test]
fn atomic_write_creates_backup() {
    let (dir, path) = write_fixture("# Heading\n\nBody\n\n");
    let request = EditRequest {
        path: path.clone(),
        pattern: mk_regex("heading"),
        options: EditOptions {
            dry_run: false,
            backup: true,
            ..Default::default()
        },
        operation: Operation::Delete,
    };

    let outcome = apply_edit(request).unwrap();
    assert!(outcome.changed);
    let backup = path.with_extension("bak");
    assert!(backup.exists());
    drop(dir);
}

#[test]
fn not_found_returns_error() {
    let (dir, path) = write_fixture("# Heading\n\nBody\n\n");
    let request = EditRequest {
        path: path.clone(),
        pattern: mk_regex("missing"),
        options: EditOptions {
            dry_run: true,
            ..Default::default()
        },
        operation: Operation::Delete,
    };

    let err = apply_edit(request).unwrap_err();
    assert!(matches!(err, EditError::NotFound));
    drop(dir);
}
