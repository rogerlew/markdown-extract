# markdown-doc Architecture Primer

The `markdown-doc` toolkit is composed of a collection of crates that layer responsibilities from configuration to the CLI. This structure keeps the codebase modular and easy to reuse from agents or other automation entry points.

## Crate Layout

- `markdown-doc-config` – owns configuration loading, precedence handling, and typed settings.
- `markdown-doc-parser` – wraps markdown parsing utilities (e.g., `pulldown-cmark`) and surfaces enriched section spans.
- `markdown-doc-format` – renders output across Markdown, JSON, and SARIF targets.
- `markdown-doc-ops` – implements the catalog, lint, move, and auxiliary operations shared by the CLI and other front-ends.
- `markdown-doc-core` – orchestrates the modules above and exposes a stable engine API.
- `markdown-doc-cli` – command-line interface built on `clap`, mapping user intent to the core engine.
- `markdown-doc-test-support` – helpers and fixtures for integration and performance testing.

## Testing Corpus

Integration tests live under `tests/markdown-doc/` and currently mirror the markdown tree from `/workdir/wepppy`. Regenerate the snapshot with the copy script noted in the Cross-Cutting Foundations plan when upstream documentation changes.

### Fixture Scale

`cloc` (2025-10-25) reports the snapshot contains **734 Markdown files** (797 text files total) comprising **82,695 lines of Markdown code** and **31,385 blank lines**. The benchmark baseline and future profiling expectations assume this corpus size; re-run `cloc` after refreshing the fixtures to keep stats current.

## Configuration Loading

The `markdown-doc-config` crate resolves configuration using the precedence stack defined in the spec:

1. CLI override (`--config`)
2. `.markdown-doc.toml` in the working directory
3. `.markdown-doc.toml` at the git repository root
4. Built-in defaults

Loader output is a typed `Config` struct covering `project`, `catalog`, and `lint` settings. Headings, glob patterns, and lint rule identifiers are validated on load, and each resolved layer is tracked via `ConfigSources` so higher-level crates can surface provenance in diagnostics.

## Parser Spans

`markdown-doc-parser` exposes a `ParserContext` that reuses `markdown-extract` heading detection to emit `DocumentSection` spans. Each span includes:

- Normalised heading text and stable anchor identifiers.
- Section byte ranges (heading through trailing body) plus per-line copies of the raw content.
- Path metadata (`absolute`/`relative`) filtered through config-driven include/exclude patterns.

Anchors are generated with Markdown-style slug rules and the heading normaliser is re-exported for lint/catalog consumers. Parser tests cover ATX/Setext headings, YAML front matter, fenced/indented code blocks, and Unicode titles so downstream engines can rely on consistent offsets.

## Catalog & Lint (MVP)

Phase 1 now wires the parser into real operations:

- `markdown-doc catalog` walks Markdown files (respecting config include/exclude filters), renders the documentation catalog (`DOC_CATALOG.md` by default) via atomic writes, and supports `--format json` for agent workflows.
- `markdown-doc lint` currently ships the `broken-links` rule. It checks intra-repo Markdown links, honours severity overrides and ignore patterns, and emits plain, JSON, or SARIF reports.

Both commands accept selective scanning flags (`--path`, `--staged`) and share the `ScanOptions` plumbing so future operations can reuse the same targeting logic.

## CI & Benchmarks

Continuous integration now enforces formatting, linting, and tests for every push/PR via `.github/workflows/build_and_test.yml`:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all --all-features
```

Nightly (or manual) benchmark runs live in `.github/workflows/bench.yml`. The harness (`tools/markdown-doc-bench`) measures `catalog` and `lint --format json` against the WEPPpy fixtures:

```bash
cargo run -p markdown-doc-bench --release -- --path tests/markdown-doc/wepppy
```

The workflow uploads `benchmark-results.txt` and writes a summary to the job output so we can track regressions over time.

### Current Baseline (2025-10-25)

Running `cargo run -p markdown-doc-bench --release -- --path tests/markdown-doc/wepppy` on the reference environment (3 iterations, 1 warm-up) yields:

- `catalog`: average **78.09 ms** (median 76.41 ms)
- `lint-broken-links`: average **76.95 ms** (median 77.87 ms)

Re-run the harness after significant parser or IO changes to keep this baseline up to date.

## Roadmap

The accompanying `markdown-doc.plan.nd` file tracks phased milestones (MVP, Quality Gates, Refactoring Support, Intelligence). Update the checklists as features land to keep the roadmap accurate.
