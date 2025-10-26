# Agent Prompt – TOC Command & Severity Tuning

## Objective
Finish Phase 2 by shipping a production-ready `markdown-doc toc` command and the severity/ignore plumbing required by the spec. All work must land without regressing existing commands (`catalog`, `lint`, `validate`) or their configuration semantics.

## Current Context
- **TOC plumbing exists but needs refinement**: `Operations::toc` implements `--check`, `--diff`, and `--update` paths, uses atomic writes, and feeds `TocOutcome` metadata back to the CLI (`crates/markdown-doc-ops/src/lib.rs:205`). Shared helpers live in `crates/markdown-doc-ops/src/toc.rs`.
- **CLI wiring is in place**: the new `toc` subcommand accepts `--path`, `--staged`, `--check` (default), `--diff`, `--update`, `--no-ignore`, and `--quiet` (`crates/markdown-doc-cli/src/lib.rs:26`, `crates/markdown-doc-cli/src/lib.rs:144`).
- **Severity + ignore configuration landed**: `LintSettings` now tracks wildcard severities, per-path overrides, and `.markdown-doc-ignore` support (`crates/markdown-doc-config/src/lib.rs:48`, `crates/markdown-doc-config/src/lib.rs:96`). Tests exercise precedence rules (`crates/markdown-doc-config/tests/loader.rs:307`).
- **Lint runner consumes overrides**: severity is resolved per path via `LintSettings::severity_for_path`, and ignores map to rule-scoped glob matchers (`crates/markdown-doc-ops/src/lint.rs:231`).
- **Initial tests/docs**: We have a smoke test for `.markdown-doc-ignore` covering check mode (`crates/markdown-doc-ops/tests/toc.rs:1`) and high-level README notes on TOC markers, but no CLI/UX coverage yet. Tracker entries stop at Agent 6.

## Known Issues & Gaps
- `cargo test --all` presently fails: `GitignoreBuilder::add` is treated as returning `Option` and `is_err()` does not compile (`crates/markdown-doc-ops/src/lib.rs:409`). Fix the helper and add regression coverage.
- `toc::locate_block`/`render_items` warn about unused assignments on trailing lines (`crates/markdown-doc-ops/src/toc.rs:75`). Audit EOF handling and ensure missing end markers surface deterministic errors.
- No tests assert TOC generation fidelity (indent calculation, anchor normalization, multiline headings), diff output, or update mode writing.
- Lint severity overrides are unit-tested but not exercised end-to-end; lint JSON/SARIF renderers should confirm severity labels reflect overrides.
- README and `docs/markdown-doc/README.md` mention TOC markers but omit CLI usage, `.markdown-doc-ignore`, wildcard severity config, and the new `--no-ignore` flag.
- Work-package tracker lacks an Agent 7 log entry; update on completion.

## Deliverables
1. **TOC command polish**
   - Harden block detection/rendering (multiple blocks, Windows newlines, empty body cases) and share helpers with the `toc-sync` lint rule.
   - Ensure `--check`, `--diff`, and `--update` follow spec exit codes and never touch disk outside update mode.
   - Preserve newline style and allow optional backups if config demands it (reuse `markdown_doc_utils::atomic_write` scaffolding).
   - Gracefully report missing markers and aggregate actionable `TocChange` data for CLI/renderers.
2. **Severity & ignore enhancements**
   - Confirm wildcard (`"*"`) entries, per-path overrides, and `.markdown-doc-ignore` merge in deterministic order (CLI override > ignore file > config).
   - Expand lint runtime tests to cover warning/ignore transitions and verify rules re-enable when overrides demand it (`crates/markdown-doc-ops/tests/lint.rs:150` is the current baseline).
   - Expose effective severity maps to renderers so plain/JSON/SARIF outputs remain accurate.
3. **Documentation & UX**
   - Document the `toc` workflow, ignore precedence, and severity override syntax in README and architecture docs.
   - Add tracker notes summarising the shipped behaviour, tests, and any deferred follow-ups.

## Testing Requirements
- Unit tests for TOC parsing/rendering (including EOF cases, indentation, nested headings, duplicate markers).
- CLI/integration tests covering `--check`, `--diff`, `--update`, `.markdown-doc-ignore`, and `--no-ignore`.
- Configuration and lint tests exercising wildcard severity, per-path overrides, and JSON/SARIF severity output.
- Broken pipe handling for `toc --diff | head`.
- Re-run `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features`, and `cargo test --all` once the above lands.

## Documentation & Tracker Updates
- Update README + `docs/markdown-doc/README.md` with TOC command examples, ignore semantics, and severity configuration guidance.
- Record outcomes in `docs/work-packages/20251025_markdown_doc_toolkit/tracker.md` (Agent 7 entry).

## Acceptance Criteria
- All workspace checks pass (`cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features`, `cargo test --all`).
- `markdown-doc toc` handles selective scans, ignore overrides, and emits deterministic exit codes/messages.
- Severity tuning and ignore behaviour are covered by automated tests and reflected in rendered output/documentation.
- Work package artifacts updated (prompt, tracker, docs).

## Handoff Checklist
- Summarise shipped changes, outstanding TODOs, and test commands in the tracker and final handoff.
- Call out any follow-up risks (e.g., future enhancements to TOC diff formatting or ignore pattern performance).
