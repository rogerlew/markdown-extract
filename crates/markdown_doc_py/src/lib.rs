use std::path::PathBuf;

use markdown_bindings_common::format_io_error;
use markdown_doc_config::{Config, LoadOptions};
use markdown_doc_ops::{OperationError, Operations, ScanOptions, TocMode, TocOptions, TocStatus};
use pyo3::create_exception;
use pyo3::prelude::*;

create_exception!(
    markdown_doc_py,
    MarkdownDocError,
    pyo3::exceptions::PyException
);

#[pyclass(module = "markdown_doc_py")]
pub struct TocResult {
    #[pyo3(get)]
    mode: String,
    #[pyo3(get)]
    status: String,
    #[pyo3(get)]
    diff: Option<String>,
    #[pyo3(get)]
    messages: Vec<String>,
}

#[pyfunction(signature = (path, *, mode="check", no_ignore=false, quiet=false))]
fn toc(path: &str, mode: &str, no_ignore: bool, quiet: bool) -> PyResult<TocResult> {
    let config = Config::load(LoadOptions::default())
        .map_err(|err| MarkdownDocError::new_err(err.to_string()))?;
    let operations = Operations::new(config);

    let scan = ScanOptions {
        paths: vec![PathBuf::from(path)],
        staged: false,
        respect_ignore: !no_ignore,
    };

    let mode = parse_mode(mode)?;
    let options = TocOptions { scan, mode, quiet };

    let outcome = operations.toc(options).map_err(map_operation_error)?;

    Ok(TocResult::from_outcome(outcome, mode))
}

#[pymodule]
fn markdown_doc_py(_py: Python, module: &PyModule) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(toc, module)?)?;
    module.add("MarkdownDocError", _py.get_type::<MarkdownDocError>())?;
    module.add_class::<TocResult>()?;
    Ok(())
}

fn parse_mode(mode: &str) -> PyResult<TocMode> {
    match mode {
        "check" => Ok(TocMode::Check),
        "update" => Ok(TocMode::Update),
        "diff" => Ok(TocMode::Diff),
        other => Err(MarkdownDocError::new_err(format!(
            "unsupported toc mode: {other}"
        ))),
    }
}

fn map_operation_error(err: OperationError) -> PyErr {
    match err {
        OperationError::Io { path, source } => {
            MarkdownDocError::new_err(format_io_error(&source, Some(&path)))
        }
        OperationError::Git { message, .. } => MarkdownDocError::new_err(message),
        OperationError::SchemaNotFound { name } => {
            MarkdownDocError::new_err(format!("schema '{name}' not found"))
        }
        OperationError::InvalidInput(message) => MarkdownDocError::new_err(message),
        OperationError::Rewrite(rewrite_err) => MarkdownDocError::new_err(rewrite_err.to_string()),
        OperationError::Other(message) => MarkdownDocError::new_err(message),
    }
}

impl TocResult {
    fn from_outcome(outcome: markdown_doc_ops::TocOutcome, mode: TocMode) -> Self {
        let mut status = "clean".to_string();
        let mut diff_segments = Vec::new();
        let mut has_error = false;
        let mut has_changes = false;

        for change in &outcome.changes {
            match change.status {
                TocStatus::MissingMarkers => {
                    has_error = true;
                }
                TocStatus::NeedsUpdate | TocStatus::Updated => {
                    has_changes = true;
                }
                TocStatus::UpToDate => {}
            }

            if let Some(diff) = &change.diff {
                diff_segments.push(diff.clone());
            }
        }

        if has_error {
            status = "error".to_string();
        } else if has_changes {
            status = "changed".to_string();
        }

        let diff = if diff_segments.is_empty() {
            None
        } else {
            Some(diff_segments.join("\n"))
        };

        let messages = if outcome.rendered.is_empty() {
            Vec::new()
        } else {
            outcome
                .rendered
                .lines()
                .map(|line| line.to_string())
                .collect()
        };

        TocResult {
            mode: match mode {
                TocMode::Check => "check".to_string(),
                TocMode::Update => "update".to_string(),
                TocMode::Diff => "diff".to_string(),
            },
            status,
            diff,
            messages,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn module_registers() {
        Python::with_gil(|py| {
            let module = PyModule::new(py, "markdown_doc_py").unwrap();
            markdown_doc_py(py, module).unwrap();
        });
    }
}
