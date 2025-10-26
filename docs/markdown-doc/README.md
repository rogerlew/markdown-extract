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

## Catalog & Lint

The operations layer wires parser output into the user-facing commands:

- `markdown-doc catalog` walks Markdown files (respecting config include/exclude filters), renders the documentation catalog (`DOC_CATALOG.md` by default) via atomic writes, and supports `--format json` for agent workflows.
- `markdown-doc lint` now executes a configurable rule pipeline. Phase&nbsp;2 expands coverage beyond broken links to include:
  - `broken-anchors` – validates intra-/inter-file anchor fragments and suggests closest matches.
  - `duplicate-anchors` – flags repeated heading slugs within a single document.
  - `heading-hierarchy` – detects skipped levels and headings exceeding `lint.max_heading_depth`.
  - `toc-sync` – compares declared TOC blocks (between `lint.toc_start_marker`/`lint.toc_end_marker`) against the live heading tree.
  - `required-sections` – delegates to the shared schema matcher so linting surfaces the same structural issues as `validate`.

Rules respect severity overrides (`lint.severity`), per-path ignores, and emit findings annotated with the originating rule. Output formats (plain/JSON/SARIF) expose the same metadata, making it straightforward for downstream automation to slice by rule. Both commands continue to share the `ScanOptions` plumbing (`--path`, `--staged`) so future operations can reuse targeting logic.

Configuration additions:

- `lint.rules` defaults to `broken-links` but can enable any subset of the Phase&nbsp;2 rules.
- `lint.toc_start_marker` / `lint.toc_end_marker` control which markers delineate TOC regions for `toc-sync` (defaults remain `<!-- toc -->` / `<!-- tocstop -->`).
- `lint.max_heading_depth` still bounds allowable heading levels; `heading-hierarchy` enforces the limit.

## Validate Command & Schema Matcher

The `validate` command builds on the schema engine delivered in Phase&nbsp;2:

1. Configuration supplies `[schemas]` entries with glob patterns, required section order, depth bounds, and duplicate allowances.
2. `SchemaEngine` resolves the schema for each document (falling back to the default when no pattern matches or `--schema` is omitted).
3. Violations (missing sections, ordering mistakes, unexpected/extra headings, depth overflows, missing top-level headings, empty documents) are emitted as structured `SchemaViolation`s reused by both `validate` and the `required-sections` lint rule.

`markdown-doc validate` mirrors the lint UX: plain output for humans, JSON for tooling, and exit codes (`0` success, `1` validation failures, `2` unknown schema, `3` runtime/config errors). This keeps lightweight linting and deep template enforcement consistent while allowing future Agent&nbsp;6 work to extend schema semantics without duplicating logic.

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
