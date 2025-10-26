use std::path::PathBuf;

use anyhow::Result;
use clap::{ArgAction, Args, Parser, Subcommand, ValueEnum};
use markdown_doc_config::{Config, LoadOptions};
use markdown_doc_core::MarkdownDoc;
use markdown_doc_format::{CatalogFormat, LintFormat};
use markdown_doc_ops::{CatalogOptions, CatalogOutcome, LintOptions, LintOutcome, ScanOptions};

/// Entry point for CLI execution. Returns the desired exit code.
pub fn run() -> Result<i32> {
    let cli = Cli::parse();

    let config = Config::load(LoadOptions::default())?;
    let engine = MarkdownDoc::bootstrap(config);
    let ops = engine.operations();

    match cli.command {
        Command::Catalog(args) => handle_catalog(ops, args),
        Command::Lint(args) => handle_lint(ops, args),
    }
}

fn handle_catalog(ops: &markdown_doc_ops::Operations, args: CatalogArgs) -> Result<i32> {
    let CatalogArgs {
        path,
        staged,
        format,
        output,
        ..
    } = args;

    let format = match format.unwrap_or(CatalogFormatValue::Markdown) {
        CatalogFormatValue::Markdown => CatalogFormat::Markdown,
        CatalogFormatValue::Json => CatalogFormat::Json,
    };

    let scan = ScanOptions {
        paths: path,
        staged,
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
    } = args;

    let format = match format.unwrap_or(LintFormatValue::Plain) {
        LintFormatValue::Plain => LintFormat::Plain,
        LintFormatValue::Json => LintFormat::Json,
        LintFormatValue::Sarif => LintFormat::Sarif,
    };

    let scan = ScanOptions {
        paths: path,
        staged,
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
