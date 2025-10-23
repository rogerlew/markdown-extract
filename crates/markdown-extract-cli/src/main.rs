use anyhow::{bail, Context, Result};
use clap::Parser;
use markdown_extract::{extract_from_path, extract_from_reader, MarkdownSection};
use regex::{Regex, RegexBuilder};
use std::{
    io::{self, BufReader, Write},
    path::PathBuf,
};

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// Print all matching sections (don't quit after first match)
    #[arg(short, long)]
    all: bool,

    /// Treat pattern as case sensitive
    #[arg(short = 's', long)]
    case_sensitive: bool,

    /// Do not include the matched heading in the output
    #[arg(short, long)]
    no_print_matched_heading: bool,

    /// Pattern to match against headings
    #[arg(value_name = "PATTERN")]
    pattern: String,

    /// Path to markdown file
    #[arg(value_name = "FILE")]
    path: PathBuf,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let regex = RegexBuilder::new(&cli.pattern)
        .case_insensitive(!cli.case_sensitive)
        .size_limit(1024 * 100) // 100 kb
        .build()
        .unwrap();

    let matches = collect_matches(&cli.path, &regex).with_context(|| {
        format!(
            "Unable to extract at path or stream: {}",
            cli.path.display()
        )
    })?;

    if matches.is_empty() {
        bail!("No matches found for pattern: {}", cli.pattern);
    }

    if !cli.all {
        return print_section(&matches[0], cli.no_print_matched_heading);
    }

    // match is a reserved keyword
    for m in matches {
        print_section(&m, cli.no_print_matched_heading)?;
    }

    Ok(())
}

fn print_section(section: &MarkdownSection, skip_printing_matched_heading: bool) -> Result<()> {
    let stdout = io::stdout();
    let mut handle = stdout.lock();

    for line in section
        .iter()
        .skip(if skip_printing_matched_heading { 1 } else { 0 })
    {
        match writeln!(handle, "{}", line) {
            Ok(_) => {}
            Err(err) if should_ignore_pipe_error(&err) => return Ok(()),
            Err(err) => {
                return Err(err).context(format!("Failed to print line: {}", line));
            }
        }
    }

    match handle.flush() {
        Ok(_) => Ok(()),
        Err(err) if should_ignore_pipe_error(&err) => Ok(()),
        Err(err) => Err(err).context("Failed to flush stdout"),
    }
}

fn should_ignore_pipe_error(err: &io::Error) -> bool {
    matches!(
        err.kind(),
        io::ErrorKind::BrokenPipe | io::ErrorKind::WouldBlock
    )
}

fn collect_matches(path: &PathBuf, regex: &Regex) -> Result<Vec<MarkdownSection>> {
    if path == &PathBuf::from("-") {
        let stdin = io::stdin();
        let mut reader = BufReader::new(stdin.lock());
        Ok(extract_from_reader(&mut reader, regex))
    } else {
        Ok(extract_from_path(path, regex)?)
    }
}
