# Agent Prompt – Catalog & Lint MVP

## Objective
Implement the Phase 1 command slice inside `markdown-doc-ops`/`markdown-doc-cli`: `catalog` generation and `lint` with the `broken-links` rule, wired through configuration and parser layers.

## Deliverables
- Operational `catalog` command:
  - Supports `--path`, `--staged`, default full scan.
  - Generates Markdown output matching spec, plus `--format json` variant.
  - Writes atomically to configured output (`DOC_CATALOG.md` default).
  - Meets performance target in fixtures (<5s on 388 files; coordinate with benchmark harness).
- `lint` command (broken-links rule only):
  - Resolves internal links, reports errors/warnings per severity.
  - Supports ignore lists from config.
  - Outputs plain text and `--format json`/`--format sarif`.
  - Exit codes per convention (0 success, 1 validation failure, etc.).
- CLI wiring with `clap` options defined in spec.
- Integration tests covering selective scanning, output formats, exit codes, broken pipe resilience.

## Constraints & Notes
- Reuse config/ parser APIs from Agents 1 & 2.
- Use `rayon` executor for concurrency; ensure deterministic ordering in outputs.
- Implement atomic writer in utils crate (`markdown-doc-utils`), with optional backups (flag stub acceptable if spec defers).
- Testing with fixtures under `tests/markdown-doc/`; consider snapshot tests for catalog output.
- Update tracker with progress, mark tasks when done.

## Acceptance Criteria
- `cargo test -p markdown-doc-cli` and relevant integration tests pass.
- Commands behave per spec examples (selective scanning, JSON field names, exit codes).
- Catalog performance validated and recorded (coordinate with Agent 4).
- Documentation/usage examples added to README or separate guide.
