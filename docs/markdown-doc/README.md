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

## Configuration Loading

The `markdown-doc-config` crate resolves configuration using the precedence stack defined in the spec:

1. CLI override (`--config`)
2. `.markdown-doc.toml` in the working directory
3. `.markdown-doc.toml` at the git repository root
4. Built-in defaults

Loader output is a typed `Config` struct covering `project`, `catalog`, and `lint` settings. Headings, glob patterns, and lint rule identifiers are validated on load, and each resolved layer is tracked via `ConfigSources` so higher-level crates can surface provenance in diagnostics.

## Roadmap

The accompanying `markdown-doc.plan.nd` file tracks phased milestones (MVP, Quality Gates, Refactoring Support, Intelligence). Update the checklists as features land to keep the roadmap accurate.
