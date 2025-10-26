use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use clap::Parser;
use markdown_doc_config::{Config, LoadOptions};
use markdown_doc_core::MarkdownDoc;
use markdown_doc_format::{CatalogFormat, LintFormat};
use markdown_doc_ops::{CatalogOptions, LintOptions, ScanOptions};

#[derive(Parser)]
#[command(
    author,
    version,
    about = "Benchmark harness for markdown-doc operations"
)]
struct Args {
    /// Target directory or file to benchmark against
    #[arg(long, value_name = "PATH", default_value = "tests/markdown-doc/wepppy")]
    path: PathBuf,

    /// Number of warm-up iterations (not counted in results)
    #[arg(long, default_value_t = 1)]
    warmup: usize,

    /// Number of measured iterations
    #[arg(long, default_value_t = 3)]
    iterations: usize,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let config = Config::load(LoadOptions::default())?;
    let engine = MarkdownDoc::bootstrap(config);
    let ops = engine.operations();

    let target = normalize_target(&args.path)?;

    let catalog_result = benchmark("catalog", args.warmup, args.iterations, || {
        let options = CatalogOptions {
            scan: ScanOptions {
                paths: vec![target.clone()],
                staged: false,
            },
            format: CatalogFormat::Markdown,
            output_path: None,
            write_to_disk: false,
        };
        ops.catalog(options)?;
        Ok(())
    })?;

    let lint_result = benchmark("lint-broken-links", args.warmup, args.iterations, || {
        let options = LintOptions {
            scan: ScanOptions {
                paths: vec![target.clone()],
                staged: false,
            },
            format: LintFormat::Json,
        };
        ops.lint_broken_links(options)?;
        Ok(())
    })?;

    print_summary(&[catalog_result, lint_result]);
    Ok(())
}

fn normalize_target(path: &Path) -> Result<PathBuf> {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .context("failed to resolve repository root")?;

    if path.is_absolute() {
        let relative = path
            .strip_prefix(repo_root)
            .with_context(|| format!("path '{}' is outside the repository", path.display()))?
            .to_path_buf();
        return Ok(relative);
    }

    let joined = repo_root.join(path);
    if joined.exists() {
        Ok(path.to_path_buf())
    } else {
        anyhow::bail!("benchmark target '{}' not found", path.display())
    }
}

struct BenchResult {
    name: String,
    iterations: usize,
    average: Duration,
    median: Duration,
}

fn benchmark<F>(name: &str, warmup: usize, iterations: usize, mut f: F) -> Result<BenchResult>
where
    F: FnMut() -> Result<()>,
{
    for _ in 0..warmup {
        f()?;
    }

    let mut samples = Vec::with_capacity(iterations);
    for _ in 0..iterations {
        let start = Instant::now();
        f()?;
        samples.push(start.elapsed());
    }

    samples.sort();
    let total = samples
        .iter()
        .copied()
        .fold(Duration::ZERO, |acc, sample| acc + sample);
    let average = total / (iterations as u32);
    let median = samples[samples.len() / 2];

    Ok(BenchResult {
        name: name.to_string(),
        iterations,
        average,
        median,
    })
}

fn print_summary(results: &[BenchResult]) {
    println!("markdown-doc benchmark results:\n");
    for result in results {
        println!(
            "• {}: avg {:?} (median {:?}) over {} iterations",
            result.name, result.average, result.median, result.iterations
        );
    }
}
