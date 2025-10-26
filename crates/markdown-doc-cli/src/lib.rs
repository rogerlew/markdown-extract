use std::path::PathBuf;

use anyhow::Result;
use clap::{ArgAction, Args, Parser, Subcommand, ValueEnum};
use markdown_doc_config::{Config, LoadOptions};
use markdown_doc_core::MarkdownDoc;
use markdown_doc_format::{CatalogFormat, LintFormat, ValidateFormat};
use markdown_doc_ops::OperationError;
use markdown_doc_ops::{
    CatalogOptions, CatalogOutcome, LintOptions, LintOutcome, MvFileStatus, MvOptions, ScanOptions,
    TocMode, TocOptions, TocOutcome, ValidateOptions, ValidateOutcome,
};
use serde_json::json;

/// Entry point for CLI execution. Returns the desired exit code.
pub fn run() -> Result<i32> {
    let cli = Cli::parse();

    let config = Config::load(LoadOptions::default())?;
    let engine = MarkdownDoc::bootstrap(config);
    let ops = engine.operations();

    match cli.command {
        Command::Catalog(args) => handle_catalog(ops, args),
        Command::Lint(args) => handle_lint(ops, args),
        Command::Validate(args) => handle_validate(ops, args),
        Command::Toc(args) => handle_toc(ops, args),
        Command::Mv(args) => handle_mv(ops, args),
    }
}

fn handle_catalog(ops: &markdown_doc_ops::Operations, args: CatalogArgs) -> Result<i32> {
    let CatalogArgs {
        path,
        staged,
        format,
        output,
        no_ignore,
        ..
    } = args;

    let format = match format.unwrap_or(CatalogFormatValue::Markdown) {
        CatalogFormatValue::Markdown => CatalogFormat::Markdown,
        CatalogFormatValue::Json => CatalogFormat::Json,
    };

    let scan = ScanOptions {
        paths: path,
        staged,
        respect_ignore: !no_ignore,
    };

    let write_to_disk = matches!(format, CatalogFormat::Markdown);
    let options = CatalogOptions {
        scan,
        format,
        output_path: output,
        write_to_disk,
    };

    let CatalogOutcome { rendered, .. } = ops.catalog(options)?;

    emit(&rendered)?;
    Ok(0)
}

fn handle_lint(ops: &markdown_doc_ops::Operations, args: LintArgs) -> Result<i32> {
    let LintArgs {
        path,
        staged,
        format,
        no_ignore,
    } = args;

    let format = match format.unwrap_or(LintFormatValue::Plain) {
        LintFormatValue::Plain => LintFormat::Plain,
        LintFormatValue::Json => LintFormat::Json,
        LintFormatValue::Sarif => LintFormat::Sarif,
    };

    let scan = ScanOptions {
        paths: path,
        staged,
        respect_ignore: !no_ignore,
    };

    let options = LintOptions { scan, format };
    let LintOutcome {
        rendered,
        exit_code,
        ..
    } = ops.lint(options)?;

    emit(&rendered)?;
    Ok(exit_code)
}

fn handle_validate(ops: &markdown_doc_ops::Operations, args: ValidateArgs) -> Result<i32> {
    let ValidateArgs {
        path,
        staged,
        format,
        schema,
        quiet,
        no_ignore,
    } = args;

    let format = match format.unwrap_or(ValidateFormatValue::Plain) {
        ValidateFormatValue::Plain => ValidateFormat::Plain,
        ValidateFormatValue::Json => ValidateFormat::Json,
    };

    let scan = ScanOptions {
        paths: path,
        staged,
        respect_ignore: !no_ignore,
    };
    let options = ValidateOptions {
        scan,
        format,
        schema,
        quiet,
    };

    match ops.validate(options) {
        Ok(ValidateOutcome {
            rendered,
            exit_code,
            ..
        }) => {
            if !rendered.is_empty() {
                emit(&rendered)?;
            }
            Ok(exit_code)
        }
        Err(OperationError::SchemaNotFound { name }) => {
            if !quiet {
                emit(&format!("schema '{name}' not found"))?;
            }
            Ok(2)
        }
        Err(err) => Err(err.into()),
    }
}

fn handle_toc(ops: &markdown_doc_ops::Operations, args: TocArgs) -> Result<i32> {
    let TocArgs {
        path,
        staged,
        check,
        update,
        diff,
        no_ignore,
        quiet,
    } = args;

    let mode = if update {
        TocMode::Update
    } else if diff {
        TocMode::Diff
    } else {
        // Explicit --check maps to same default mode.
        let _ = check;
        TocMode::Check
    };

    let scan = ScanOptions {
        paths: path,
        staged,
        respect_ignore: !no_ignore,
    };

    let options = TocOptions { scan, mode, quiet };
    let TocOutcome {
        rendered,
        exit_code,
        ..
    } = ops.toc(options)?;

    if !rendered.is_empty() {
        emit(&rendered)?;
    }

    Ok(exit_code)
}

fn handle_mv(ops: &markdown_doc_ops::Operations, args: MvArgs) -> Result<i32> {
    let MvArgs {
        source,
        destination,
        dry_run,
        force,
        no_backup,
        quiet,
        json,
        no_ignore,
    } = args;

    let scan = ScanOptions {
        paths: Vec::new(),
        staged: false,
        respect_ignore: !no_ignore,
    };

    let options = MvOptions {
        scan,
        source,
        destination,
        dry_run,
        force,
        create_backup: !no_backup,
        quiet,
        json,
    };

    match ops.mv(options) {
        Ok(outcome) => {
            if json {
                let payload = json!({
                    "dry_run": outcome.dry_run,
                    "files": outcome.changes.iter().map(|change| {
                        json!({
                            "original": change.original_path,
                            "output": change.output_path,
                            "status": match change.status {
                                MvFileStatus::Updated => "updated",
                                MvFileStatus::Relocated => "relocated",
                                MvFileStatus::Unchanged => "unchanged",
                            },
                            "diff": change.diff
                        })
                    }).collect::<Vec<_>>()
                });
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else if !quiet {
                for change in &outcome.changes {
                    match change.status {
                        MvFileStatus::Relocated => {
                            println!(
                                "📦 moved {} -> {}",
                                change.original_path.display(),
                                change.output_path.display()
                            );
                        }
                        MvFileStatus::Updated => {
                            println!("✏️  updated {}", change.original_path.display());
                        }
                        MvFileStatus::Unchanged => {
                            println!("ℹ️  no changes for {}", change.original_path.display());
                        }
                    }
                    if let Some(diff) = &change.diff {
                        print!("{diff}");
                        if !diff.ends_with('\n') {
                            println!();
                        }
                    }
                }
            }
            Ok(outcome.exit_code)
        }
        Err(OperationError::InvalidInput(message)) => {
            eprintln!("{message}");
            Ok(1)
        }
        Err(OperationError::Io { path, source }) => {
            eprintln!("I/O error on {}: {}", path.display(), source);
            Ok(4)
        }
        Err(err) => Err(err.into()),
    }
}

fn emit(content: &str) -> Result<()> {
    print!("{}", content);
    if !content.ends_with('\n') {
        println!();
    }
    Ok(())
}

#[derive(Parser)]
#[command(
    author,
    version,
    about = "markdown-doc toolkit",
    propagate_version = true
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Generate the unified documentation catalog
    Catalog(CatalogArgs),
    /// Run lint rules
    Lint(LintArgs),
    /// Validate documentation against schemas
    Validate(ValidateArgs),
    /// Synchronise table-of-contents blocks
    Toc(TocArgs),
    /// Move or rename a Markdown file and update references
    Mv(MvArgs),
}

#[derive(Args)]
struct CatalogArgs {
    /// Restrict catalog to specific paths
    #[arg(long = "path", value_name = "PATH", action = ArgAction::Append)]
    path: Vec<PathBuf>,
    /// Limit processing to staged files
    #[arg(long)]
    staged: bool,
    /// Output format (markdown or json)
    #[arg(long, value_enum)]
    format: Option<CatalogFormatValue>,
    /// Override catalog output path (markdown only)
    #[arg(long)]
    output: Option<PathBuf>,
    /// Disable `.markdown-doc-ignore` filtering
    #[arg(long = "no-ignore")]
    no_ignore: bool,
    /// Force regeneration (reserved for future use)
    #[arg(long)]
    #[allow(unused)]
    regen: bool,
}

#[derive(Args)]
struct LintArgs {
    /// Restrict linting to specific paths
    #[arg(long = "path", value_name = "PATH", action = ArgAction::Append)]
    path: Vec<PathBuf>,
    /// Limit linting to staged files
    #[arg(long)]
    staged: bool,
    /// Select lint output format
    #[arg(long, value_enum)]
    format: Option<LintFormatValue>,
    /// Disable `.markdown-doc-ignore` filtering
    #[arg(long = "no-ignore")]
    no_ignore: bool,
}

#[derive(Args)]
struct ValidateArgs {
    /// Restrict validation to specific paths
    #[arg(long = "path", value_name = "PATH", action = ArgAction::Append)]
    path: Vec<PathBuf>,
    /// Limit validation to staged files
    #[arg(long)]
    staged: bool,
    /// Select validate output format
    #[arg(long, value_enum)]
    format: Option<ValidateFormatValue>,
    /// Force a specific schema by name
    #[arg(long = "schema", value_name = "NAME")]
    schema: Option<String>,
    /// Suppress successful output
    #[arg(long)]
    quiet: bool,
    /// Disable `.markdown-doc-ignore` filtering
    #[arg(long = "no-ignore")]
    no_ignore: bool,
}

#[derive(Args)]
struct TocArgs {
    /// Restrict TOC updates to specific paths
    #[arg(long = "path", value_name = "PATH", action = ArgAction::Append)]
    path: Vec<PathBuf>,
    /// Limit TOC processing to staged files
    #[arg(long)]
    staged: bool,
    /// Report differences without modifying files (default)
    #[arg(long, conflicts_with_all = ["update", "diff"])]
    check: bool,
    /// Rewrite TOC blocks in place
    #[arg(long, conflicts_with_all = ["check", "diff"])]
    update: bool,
    /// Print unified diffs for out-of-sync TOCs
    #[arg(long, conflicts_with_all = ["check", "update"])]
    diff: bool,
    /// Disable `.markdown-doc-ignore` filtering
    #[arg(long = "no-ignore")]
    no_ignore: bool,
    /// Suppress output when no changes are required
    #[arg(long)]
    quiet: bool,
}

#[derive(Args)]
struct MvArgs {
    /// Source Markdown file (relative to project root)
    #[arg(value_name = "SOURCE")]
    source: PathBuf,
    /// Destination Markdown file
    #[arg(value_name = "DEST")]
    destination: PathBuf,
    /// Preview changes without writing
    #[arg(long = "dry-run")]
    dry_run: bool,
    /// Overwrite destination if it already exists
    #[arg(long = "force")]
    force: bool,
    /// Disable .bak backups for modified files
    #[arg(long = "no-backup")]
    no_backup: bool,
    /// Suppress per-file logs (errors still printed)
    #[arg(long = "quiet")]
    quiet: bool,
    /// Emit machine-readable JSON summary
    #[arg(long = "json")]
    json: bool,
    /// Ignore `.markdown-doc-ignore` patterns
    #[arg(long = "no-ignore")]
    no_ignore: bool,
}

#[derive(Clone, Copy, ValueEnum)]
enum CatalogFormatValue {
    Markdown,
    Json,
}

#[derive(Clone, Copy, ValueEnum)]
enum LintFormatValue {
    Plain,
    Json,
    Sarif,
}

#[derive(Clone, Copy, ValueEnum)]
enum ValidateFormatValue {
    Plain,
    Json,
}
