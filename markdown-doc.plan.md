# markdown-doc Implementation Plan

## Cross-Cutting Foundations
- [x] Establish `markdown-doc` workspace members (`markdown-doc-core`, `markdown-doc-cli`, shared crates) in `Cargo.toml`
- [x] Add shared utilities crate for config resolution, path filters, parallel executor, atomic writer
- [ ] Wire CI pipelines (fmt, clippy, test, benchmarks) and CONTRIBUTING notes
- [x] Create `tests/markdown-doc/` fixture corpus approximating 388-file repo
- [x] Document architecture overview in `docs/markdown-doc/README.md`

## Phase 1 — MVP Vertical Slice
- [x] Implement configuration resolver with precedence (flags → local config → repo root → defaults)
- [x] Provide typed settings structs and validation errors surfaced via exit codes
- [x] Build markdown parsing layer (pulldown-cmark wrapper + heading/anchor capture using enriched spans)
- [x] Implement file enumeration respecting include/exclude patterns and git staging filters
- [x] Deliver `catalog` command with concurrent scan, `--path` / `--staged`, Markdown + JSON outputs, atomic writes
- [x] Deliver `lint` command with `broken-links` rule, severity levels, ignore lists, JSON/SARIF outputs
- [x] Add CLI acceptance tests covering selective scan, exit codes, output formats
- [x] Benchmark `catalog` over large fixture (<5s target) and record results
- [x] Publish quickstart docs (setup, config, pre-commit/CI examples)

## Phase 2 — Quality Gates Expansion
- [x] Extend lint engine with `broken-anchors`, `duplicate-anchors`, `heading-hierarchy`, `required-sections`, `toc-sync`
- [x] Implement schema matcher shared by lint/validate using `[schemas]` config patterns
- [x] Build `validate` command with deep template conformance messaging
- [x] Add severity tuning (`lint.severity`, per-path ignores, `.markdown-doc-ignore`)
- [x] Implement `toc` command for marker update/diff workflows
- [x] Expand fixture coverage and golden outputs for each lint rule
- [x] Update documentation for advanced linting and schema management

## Phase 3 — Refactoring Support
- [x] Construct link graph update engine leveraging `SectionSpan` data
- [x] Implement `mv` command with dry-run, backups, transactional reference rewrites
- [x] Implement `refs` command (file/anchor reference discovery) with JSON output option
- [x] Add stress tests for nested directories, mixed link styles, rollback scenarios
- [x] Document safe refactoring workflows and `wctl` wrappers

## Phase 4 — Intelligence & Automation
- [ ] Define acceptance criteria for search (latency, ranking, snippets)
- [ ] Implement indexing backend (evaluate `tantivy` vs custom) with cache/invalidation
- [ ] Ship `search` command with ranked results and machine-readable output
- [ ] Explore watch mode (`notify`-based) for auto lint/catalog refresh
- [ ] Provide documentation on advanced automation and agent integration

## Testing & QA
- [ ] Enforce `cargo fmt`, `cargo clippy --all-targets --all-features`, `cargo test --all` in CI
- [ ] Maintain benchmarking job tracking catalog/lint performance
- [ ] Cover broken pipe handling and exit code contracts across commands
- [ ] Ensure atomic writer tested for concurrent runs and backup behavior
- [ ] Track coverage for configuration precedence and error messages

## Immediate Next Steps
- [x] Confirm crate layout and scaffolding approach with stakeholders
- [x] Kick off Phase 1 tasks (config resolver, parser, catalog, lint)
- [x] Prepare shared fixtures/bench harness and assign implementation agents
- [x] Launch Phase 3 refactoring efforts (link graph, mv, refs)
- [ ] Define CI alignment plan with parent repo (fmt/clippy/test automation)
