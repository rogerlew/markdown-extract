#![allow(clippy::too_many_arguments)]

use std::path::{Path, PathBuf};

use markdown_bindings_common::{build_regex as build_shared_regex, format_io_error};
use markdown_edit_core::{
    apply_edit, EditError, EditOptions, EditOutcome, EditRequest, InsertOptions, Operation,
    PayloadSource, ReplaceOptions,
};
use pyo3::create_exception;
use pyo3::prelude::*;
use regex::Regex;

create_exception!(
    markdown_edit_py,
    MarkdownEditError,
    pyo3::exceptions::PyException
);

#[pyclass(module = "markdown_edit_py")]
pub struct EditResult {
    #[pyo3(get)]
    applied: bool,
    #[pyo3(get)]
    exit_code: i32,
    #[pyo3(get)]
    diff: Option<String>,
    #[pyo3(get)]
    messages: Vec<String>,
    #[pyo3(get)]
    written_path: Option<String>,
}

#[pyfunction(signature = (file, pattern, replacement, *, case_sensitive=false, all_matches=false, body_only=false, keep_heading=false, allow_duplicate=false, max_matches=None, dry_run=false, backup=true, with_path=None, with_string=None))]
fn replace(
    py: Python,
    file: &str,
    pattern: &str,
    replacement: &str,
    case_sensitive: bool,
    all_matches: bool,
    body_only: bool,
    keep_heading: bool,
    allow_duplicate: bool,
    max_matches: Option<usize>,
    dry_run: bool,
    backup: bool,
    with_path: Option<&str>,
    with_string: Option<&str>,
) -> PyResult<Py<EditResult>> {
    let payload = resolve_payload(Some(replacement), with_path, with_string)?;
    let operation = Operation::Replace(ReplaceOptions {
        payload,
        keep_heading: keep_heading || body_only,
    });
    run_edit(
        py,
        file,
        pattern,
        case_sensitive,
        all_matches,
        max_matches,
        allow_duplicate,
        dry_run,
        backup,
        operation,
    )
}

#[pyfunction(signature = (file, pattern, *, case_sensitive=false, all_matches=false, allow_duplicate=false, max_matches=None, dry_run=false, backup=true))]
fn delete(
    py: Python,
    file: &str,
    pattern: &str,
    case_sensitive: bool,
    all_matches: bool,
    allow_duplicate: bool,
    max_matches: Option<usize>,
    dry_run: bool,
    backup: bool,
) -> PyResult<Py<EditResult>> {
    run_edit(
        py,
        file,
        pattern,
        case_sensitive,
        all_matches,
        max_matches,
        allow_duplicate,
        dry_run,
        backup,
        Operation::Delete,
    )
}

#[pyfunction(signature = (file, pattern, payload, *, case_sensitive=false, all_matches=false, allow_duplicate=false, max_matches=None, dry_run=false, backup=true, with_path=None, with_string=None))]
fn append_to(
    py: Python,
    file: &str,
    pattern: &str,
    payload: &str,
    case_sensitive: bool,
    all_matches: bool,
    allow_duplicate: bool,
    max_matches: Option<usize>,
    dry_run: bool,
    backup: bool,
    with_path: Option<&str>,
    with_string: Option<&str>,
) -> PyResult<Py<EditResult>> {
    let payload = resolve_payload(Some(payload), with_path, with_string)?;
    run_edit(
        py,
        file,
        pattern,
        case_sensitive,
        all_matches,
        max_matches,
        allow_duplicate,
        dry_run,
        backup,
        Operation::AppendTo(payload),
    )
}

#[pyfunction(signature = (file, pattern, payload, *, case_sensitive=false, all_matches=false, allow_duplicate=false, max_matches=None, dry_run=false, backup=true, with_path=None, with_string=None))]
fn prepend_to(
    py: Python,
    file: &str,
    pattern: &str,
    payload: &str,
    case_sensitive: bool,
    all_matches: bool,
    allow_duplicate: bool,
    max_matches: Option<usize>,
    dry_run: bool,
    backup: bool,
    with_path: Option<&str>,
    with_string: Option<&str>,
) -> PyResult<Py<EditResult>> {
    let payload = resolve_payload(Some(payload), with_path, with_string)?;
    run_edit(
        py,
        file,
        pattern,
        case_sensitive,
        all_matches,
        max_matches,
        allow_duplicate,
        dry_run,
        backup,
        Operation::PrependTo(payload),
    )
}

#[pyfunction(signature = (file, pattern, payload, *, case_sensitive=false, all_matches=false, allow_duplicate=false, max_matches=None, dry_run=false, backup=true, with_path=None, with_string=None))]
fn insert_after(
    py: Python,
    file: &str,
    pattern: &str,
    payload: &str,
    case_sensitive: bool,
    all_matches: bool,
    allow_duplicate: bool,
    max_matches: Option<usize>,
    dry_run: bool,
    backup: bool,
    with_path: Option<&str>,
    with_string: Option<&str>,
) -> PyResult<Py<EditResult>> {
    let payload = resolve_payload(Some(payload), with_path, with_string)?;
    run_edit(
        py,
        file,
        pattern,
        case_sensitive,
        all_matches,
        max_matches,
        allow_duplicate,
        dry_run,
        backup,
        Operation::InsertAfter(InsertOptions { payload }),
    )
}

#[pyfunction(signature = (file, pattern, payload, *, case_sensitive=false, all_matches=false, allow_duplicate=false, max_matches=None, dry_run=false, backup=true, with_path=None, with_string=None))]
fn insert_before(
    py: Python,
    file: &str,
    pattern: &str,
    payload: &str,
    case_sensitive: bool,
    all_matches: bool,
    allow_duplicate: bool,
    max_matches: Option<usize>,
    dry_run: bool,
    backup: bool,
    with_path: Option<&str>,
    with_string: Option<&str>,
) -> PyResult<Py<EditResult>> {
    let payload = resolve_payload(Some(payload), with_path, with_string)?;
    run_edit(
        py,
        file,
        pattern,
        case_sensitive,
        all_matches,
        max_matches,
        allow_duplicate,
        dry_run,
        backup,
        Operation::InsertBefore(InsertOptions { payload }),
    )
}

#[pymodule]
fn markdown_edit_py(py: Python, module: &PyModule) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(replace, module)?)?;
    module.add_function(wrap_pyfunction!(delete, module)?)?;
    module.add_function(wrap_pyfunction!(append_to, module)?)?;
    module.add_function(wrap_pyfunction!(prepend_to, module)?)?;
    module.add_function(wrap_pyfunction!(insert_after, module)?)?;
    module.add_function(wrap_pyfunction!(insert_before, module)?)?;
    module.add("MarkdownEditError", py.get_type::<MarkdownEditError>())?;
    module.add_class::<EditResult>()?;
    Ok(())
}

fn run_edit(
    py: Python,
    file: &str,
    pattern: &str,
    case_sensitive: bool,
    all_matches: bool,
    max_matches: Option<usize>,
    allow_duplicate: bool,
    dry_run: bool,
    backup: bool,
    operation: Operation,
) -> PyResult<Py<EditResult>> {
    let regex = build_regex(pattern, case_sensitive)?;
    let path = PathBuf::from(file);
    let options = build_edit_options(all_matches, max_matches, allow_duplicate, dry_run, backup)?;

    let request = EditRequest {
        path: path.clone(),
        pattern: regex,
        options,
        operation,
    };

    match apply_edit(request) {
        Ok(outcome) => {
            let result = EditResult::from_outcome(outcome, &path, dry_run);
            Py::new(py, result)
        }
        Err(err) => Err(map_edit_error(err, &path)),
    }
}

fn build_regex(pattern: &str, case_sensitive: bool) -> PyResult<Regex> {
    build_shared_regex(pattern, case_sensitive)
        .map_err(|err| MarkdownEditError::new_err(format!("invalid pattern: {err}")))
}

fn build_edit_options(
    all_matches: bool,
    max_matches: Option<usize>,
    allow_duplicate: bool,
    dry_run: bool,
    backup: bool,
) -> PyResult<EditOptions> {
    if let Some(0) = max_matches {
        return Err(MarkdownEditError::new_err(
            "max_matches must be greater than 0",
        ));
    }

    let max_matches_value = if let Some(max) = max_matches {
        Some(max)
    } else if all_matches {
        None
    } else {
        Some(1)
    };

    Ok(EditOptions {
        allow_duplicate,
        apply_to_all: all_matches,
        max_matches: max_matches_value,
        dry_run,
        backup,
    })
}

fn resolve_payload(
    inline: Option<&str>,
    with_path: Option<&str>,
    with_string: Option<&str>,
) -> PyResult<PayloadSource> {
    if with_path.is_some() && with_string.is_some() {
        return Err(MarkdownEditError::new_err(
            "with_path and with_string cannot be used together",
        ));
    }

    if let Some(path) = with_path {
        if path == "-" {
            return Ok(PayloadSource::Stdin);
        }
        return Ok(PayloadSource::File(PathBuf::from(path)));
    }

    if let Some(text) = with_string {
        return Ok(PayloadSource::Inline(text.to_string()));
    }

    if let Some(text) = inline {
        return Ok(PayloadSource::Inline(text.to_string()));
    }

    Err(MarkdownEditError::new_err(
        "payload required (provide argument or with_path/with_string)",
    ))
}

fn map_edit_error(err: EditError, path: &Path) -> PyErr {
    match err {
        EditError::Io(io_err) => MarkdownEditError::new_err(format_io_error(&io_err, Some(path))),
        other => MarkdownEditError::new_err(other.to_string()),
    }
}

impl EditResult {
    fn from_outcome(outcome: EditOutcome, path: &Path, dry_run: bool) -> Self {
        let applied = outcome.changed && !dry_run;
        let written_path = applied.then(|| path.to_string_lossy().into_owned());

        let mut messages = Vec::new();
        if outcome.changed {
            if dry_run {
                messages.push("Dry-run: changes not written".to_string());
            } else {
                messages.push(format!("Applied edit to {}", path.display()));
            }
        } else {
            messages.push("No changes applied".to_string());
        }

        Self {
            applied,
            exit_code: outcome.exit_code as u8 as i32,
            diff: outcome.diff,
            messages,
            written_path,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn module_registers() {
        Python::with_gil(|py| {
            let module = PyModule::new(py, "markdown_edit_py").unwrap();
            markdown_edit_py(py, module).unwrap();
        });
    }
}
