use std::fs::File;
use std::io::{self, BufReader, Write};
use std::path::{Path, PathBuf};

use clap::{Parser, ValueEnum};
use markdown_edit_core::{
    apply_edit, EditError, EditOptions, EditOutcome, EditRequest, ExitCode, InsertOptions,
    Operation, PayloadSource, ReplaceOptions,
};
use regex::RegexBuilder;

#[derive(Parser, Debug)]
#[command(author, version, about = "Heading-aware Markdown editor", long_about = None)]
struct Cli {
    /// Path to markdown file
    #[arg(value_name = "FILE")]
    file: PathBuf,

    /// Operation to perform (replace, delete, append-to, prepend-to, insert-after, insert-before)
    #[arg(value_name = "OPERATION")]
    operation: OperationArg,

    /// Regex pattern to match headings (case-insensitive by default)
    #[arg(value_name = "PATTERN")]
    pattern: String,

    /// Read payload from file (use '-' for stdin)
    #[arg(long = "with", value_name = "PATH", allow_hyphen_values = true)]
    with: Option<PathBuf>,

    /// Inline payload with escape sequences (\\n, \\t, \\\\ , \\")
    #[arg(long = "with-string", value_name = "TEXT", allow_hyphen_values = true)]
    with_string: Option<String>,

    /// Keep the existing heading when replacing
    #[arg(long = "keep-heading", alias = "body-only")]
    keep_heading: bool,

    /// Treat pattern as case sensitive
    #[arg(short = 's', long = "case-sensitive")]
    case_sensitive: bool,

    /// Apply to every matching section
    #[arg(short = 'a', long = "all")]
    all: bool,

    /// Maximum number of matches allowed
    #[arg(long = "max-matches", value_name = "N")]
    max_matches: Option<usize>,

    /// Print diff without writing changes
    #[arg(long = "dry-run")]
    dry_run: bool,

    /// Force creation of backup (default behaviour)
    #[arg(long = "backup")]
    backup: bool,

    /// Disable backup creation
    #[arg(long = "no-backup", conflicts_with = "backup")]
    no_backup: bool,

    /// Suppress informational output (diffs, success messages)
    #[arg(short = 'q', long = "quiet")]
    quiet: bool,

    /// Do not treat duplicate payloads as changes
    #[arg(long = "allow-duplicate")]
    allow_duplicate: bool,
}

#[derive(Clone, Debug, ValueEnum)]
enum OperationArg {
    Replace,
    Delete,
    AppendTo,
    PrependTo,
    InsertAfter,
    InsertBefore,
}

fn main() -> std::process::ExitCode {
    let cli = Cli::parse();
    match run(cli) {
        Ok(exit) | Err(exit) => std::process::ExitCode::from(exit as u8),
    }
}

fn run(cli: Cli) -> Result<ExitCode, ExitCode> {
    let regex = build_regex(&cli)?;
    let options = build_options(&cli)?;
    let operation = build_operation(&cli)?;

    let request = EditRequest {
        path: cli.file.clone(),
        pattern: regex,
        options,
        operation,
    };

    match apply_edit(request) {
        Ok(outcome) => {
            handle_success(&cli, &outcome);
            Ok(outcome.exit_code)
        }
        Err(err) => {
            let exit = err.exit_code();
            handle_error(&cli, &err);
            Err(exit)
        }
    }
}

fn build_regex(cli: &Cli) -> Result<regex::Regex, ExitCode> {
    let mut builder = RegexBuilder::new(&cli.pattern);
    builder.case_insensitive(!cli.case_sensitive);
    builder.size_limit(1024 * 100);

    builder.build().map_err(|build_err| {
        eprintln!("Failed to compile pattern '{}': {build_err}", cli.pattern);
        ExitCode::InvalidArguments
    })
}

fn build_options(cli: &Cli) -> Result<EditOptions, ExitCode> {
    if let Some(n) = cli.max_matches {
        if n == 0 {
            eprintln!("--max-matches must be greater than 0");
            return Err(ExitCode::InvalidArguments);
        }
    }

    let backup = match (cli.backup, cli.no_backup) {
        (_, true) => false,
        (true, false) => true,
        (false, false) => true,
    };

    let max_matches = if let Some(n) = cli.max_matches {
        Some(n)
    } else if cli.all {
        None
    } else {
        Some(1)
    };

    Ok(EditOptions {
        allow_duplicate: cli.allow_duplicate,
        apply_to_all: cli.all,
        max_matches,
        dry_run: cli.dry_run,
        backup,
    })
}

fn build_operation(cli: &Cli) -> Result<Operation, ExitCode> {
    match cli.operation {
        OperationArg::Delete => {
            ensure_no_payload(cli)?;
            Ok(Operation::Delete)
        }
        OperationArg::Replace => {
            let payload = require_payload(cli)?;
            Ok(Operation::Replace(ReplaceOptions {
                payload,
                keep_heading: cli.keep_heading,
            }))
        }
        OperationArg::AppendTo => {
            let payload = require_payload(cli)?;
            Ok(Operation::AppendTo(payload))
        }
        OperationArg::PrependTo => {
            let payload = require_payload(cli)?;
            Ok(Operation::PrependTo(payload))
        }
        OperationArg::InsertAfter => {
            let payload = require_payload(cli)?;
            Ok(Operation::InsertAfter(InsertOptions { payload }))
        }
        OperationArg::InsertBefore => {
            let payload = require_payload(cli)?;
            Ok(Operation::InsertBefore(InsertOptions { payload }))
        }
    }
}

fn ensure_no_payload(cli: &Cli) -> Result<(), ExitCode> {
    if cli.with.is_some() || cli.with_string.is_some() {
        eprintln!("--with / --with-string cannot be used with 'delete'");
        return Err(ExitCode::InvalidArguments);
    }
    Ok(())
}

fn require_payload(cli: &Cli) -> Result<PayloadSource, ExitCode> {
    match (&cli.with, &cli.with_string) {
        (Some(_), Some(_)) => {
            eprintln!("--with and --with-string cannot be used together");
            Err(ExitCode::InvalidArguments)
        }
        (Some(path), None) => {
            if path == Path::new("-") {
                Ok(PayloadSource::Stdin)
            } else {
                Ok(PayloadSource::File(path.clone()))
            }
        }
        (None, Some(text)) => Ok(PayloadSource::Inline(text.clone())),
        (None, None) => {
            eprintln!(
                "Operation '{}' requires --with or --with-string",
                cli.operation
            );
            Err(ExitCode::InvalidArguments)
        }
    }
}

fn handle_success(cli: &Cli, outcome: &EditOutcome) {
    if cli.quiet {
        return;
    }

    if cli.dry_run {
        if let Some(diff) = &outcome.diff {
            print!("{diff}");
            io::stdout().flush().ok();
        } else {
            println!("No changes (dry run)");
        }
        return;
    }

    if outcome.changed {
        if let Some(diff) = &outcome.diff {
            print!("{diff}");
        }
        println!("Updated {}", cli.file.display());
    } else {
        println!("No changes applied (duplicate guard).");
    }
}

fn handle_error(cli: &Cli, err: &EditError) {
    match err {
        EditError::NotFound => {
            eprintln!("No matching sections found for pattern '{}'.", cli.pattern);
            if let Ok(headings) = collect_headings(&cli.file) {
                if !headings.is_empty() {
                    eprintln!("Candidate headings:");
                    for heading in headings.iter().take(20) {
                        eprintln!("  - {}", heading);
                    }
                }
            }
        }
        EditError::TooManyMatches { max, actual } => {
            eprintln!(
                "Pattern matched {actual} sections, exceeds limit {max}. Use --all or --max-matches to proceed."
            );
        }
        EditError::InvalidArguments(message)
        | EditError::InvalidContent(message)
        | EditError::Validation(message) => {
            eprintln!("{message}");
        }
        EditError::Io(io_err) => {
            eprintln!("I/O error: {io_err}");
        }
    }
}

fn collect_headings(path: &Path) -> io::Result<Vec<String>> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let parsed = markdown_extract::collect_headings_from_reader(&mut reader);
    Ok(parsed.into_iter().map(|p| p.heading.raw).collect())
}

impl std::fmt::Display for OperationArg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            OperationArg::Replace => "replace",
            OperationArg::Delete => "delete",
            OperationArg::AppendTo => "append-to",
            OperationArg::PrependTo => "prepend-to",
            OperationArg::InsertAfter => "insert-after",
            OperationArg::InsertBefore => "insert-before",
        })
    }
}
