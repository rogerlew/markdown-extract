# Tracker – markdown-doc Toolkit

> Living document tracking progress, decisions, risks, and communication for this work package.

## Quick Status

**Started**: 2025-10-25  
**Current phase**: Implementation (Phase 1 foundations)  
**Last updated**: 2025-10-25 (Config loader + parser spans + CI/bench)  
**Next milestone**: Complete Cross-Cutting Foundations checklist and Phase 1 MVP slice

## Task Board

### Ready / Backlog
- [ ] Evaluate CI workflow alignment with parent `/workdir/wepppy` pipelines before duplicating jobs (**PM/Agent 4**)

### In Progress
- [ ] Monitor benchmark baseline; rerun after significant parser/IO changes

### Blocked
- [ ] None currently

### Done
- [x] Bootstrap workspace crates, fixtures, and docs (2025-10-25)
- [x] Flesh out configuration resolver with precedence, defaults, and validation (**Agent 1**, 2025-10-25)
- [x] Implement markdown parsing layer with enriched spans (**Agent 2**, 2025-10-25)
- [x] Ship catalog generation + broken-links lint CLI slice (**Agent 3**, 2025-10-25)
- [x] Benchmark harness delivered + baseline captured (**Agent 4**, 2025-10-25)

## Timeline

- **2025-10-25** – Package created, initial scaffolding complete
- **2025-10-25** – Phase 1 MVP (`catalog`, `lint broken-links`) delivered
- **TBD** – Phase 2 quality gates released
- **TBD** – Phase 3 refactoring support shipped
- **TBD** – Phase 4 intelligence features delivered and package closed

## Decisions Log

### 2025-10-25: Adopt work-package model for markdown-doc rollout
**Context**: Long-running, multi-agent effort needs structured coordination.

**Options considered**:
1. Track progress solely in project plan – lacking status/decision history.
2. Open GitHub Project board – heavier setup, less textual guidance for agents.
3. Follow existing work package conventions – proven process in WEPPpy.

**Decision**: Use work package structure with `package.md`/`tracker.md` plus prompts folders.

**Impact**: Provides consistent templates for handoffs, decision tracking, and historical archiving.

## Risks and Issues

| Risk | Severity | Likelihood | Mitigation | Status |
|------|----------|------------|------------|--------|
| Performance targets for catalog/lint may slip on large repos | Medium | Medium | Benchmark early with fixtures, profile hotspots | Open |
| Link rewrite accuracy during `mv` operations | High | Medium | Develop comprehensive fixture-based tests and dry-run diffs | Open |
| Configuration complexity overwhelming users | Medium | Low | Deliver sensible defaults, layered docs, and validation errors | Open |

## Verification Checklist

### Code Quality
- [ ] All tests passing (`cargo test --all`)
- [ ] Clippy clean (`cargo clippy --all-targets --all-features`)
- [ ] fmt clean (`cargo fmt --check`)
- [ ] Benchmark targets satisfied for catalog/lint
- [ ] No new security vulnerabilities

### Documentation
- [ ] README/quickstart updated for each phase
- [ ] Agents.md / process docs revised if workflows change
- [ ] Inline comments for complex algorithms
- [ ] CLI help text synced with spec
- [ ] Work package closure notes complete

### Testing
- [ ] Unit coverage for parser/config/rules
- [ ] Integration tests across fixtures
- [ ] Dry-run diff verification for write operations
- [ ] Edge cases for selective scanning (`--path`, `--staged`)
- [ ] Backward compatibility proven against legacy CLI behavior

### Deployment
- [ ] Release artifacts published (crates.io/binaries) if applicable
- [ ] CI pipelines green with new jobs
- [ ] Pre-commit hook examples validated
- [ ] Rollback plan documented for CLI regressions

## Progress Notes

### 2025-10-25: Scaffolding & Planning Session
**Agent/Contributor**: Codex PM Agent

**Work completed**:
- Mirrored WEPPpy markdown corpus into `tests/markdown-doc/`
- Added `markdown-doc` crate suite and utilities to workspace
- Created architecture primer and phase checklist plan
- Established work package structure (`package.md`, `tracker.md`)

**Blockers encountered**:
- None

**Next steps**:
- Implement configuration resolver with precedence stack
- Design parser interface leveraging `markdown-extract` spans
- Draft CI workflows and benchmarking harness

### 2025-10-25: Config loader implementation
**Agent/Contributor**: Agent 1 (Codex)

**Work completed**:
- Implemented `markdown-doc-config` loader with precedence stack (override → local → git root → defaults).
- Added typed settings (`Project`, `Catalog`, `Lint`) with glob compilation, lint rule enums, and severity helpers.
- Introduced validation for unknown rules, invalid glob patterns, and malformed ignore entries.
- Captured provenance metadata (`ConfigSources`) for downstream diagnostics.

**Blockers encountered**:
- None

**Next steps**:
1. Extend parser crate to emit enriched spans for downstream operations.
2. Integrate config settings into core/ops crates once engines land.
3. Wire CI tasks (fmt, clippy, test) across the `markdown-doc` workspace.

**Test results**:
```bash
cargo fmt
cargo test -p markdown-doc-config
```

### 2025-10-25: Parser spans implementation
**Agent/Contributor**: Agent 2 (Codex)

**Work completed**:
- Implemented `ParserContext` in `markdown-doc-parser` to surface `DocumentSection` data with normalized headings, anchors, and byte ranges.
- Reused `markdown-extract` heading detection while mapping sections with hierarchical depth boundaries and path-scoped filters from configuration.
- Added anchor generation utility and re-exported heading normalization for downstream lint/catalog rules.
- Introduced parser-focused integration tests covering front matter, fenced/indented code blocks, Setext headings, Unicode titles, and include/exclude scope checks.

**Blockers encountered**:
- None

**Next steps**:
1. Wire parser spans into ops/lint layers to drive catalog and rule evaluation.
2. Extend tests to cover large fixtures once lint engine lands.
3. Surface streaming APIs if performance profiling reveals hotspots.

**Test results**:
```bash
cargo fmt
cargo test -p markdown-doc-parser
```

### 2025-10-25: Catalog & lint MVP
**Agent/Contributor**: Agent 3 (Codex)

**Work completed**:
- Implemented `catalog` and `lint` (broken-links) operations in `markdown-doc-ops`, including selective scanning, git-staged support, and atomic writes.
- Added renderer support for Markdown/JSON/SARIF outputs and wired the CLI via `clap` with new integration tests.
- Delivered basic link resolver, config-driven ignores/severity, and JSON/SARIF structured summaries ready for CI piping.

**Blockers encountered**:
- None

**Next steps**:
1. Expand lint engine with additional rules (anchors, hierarchy, required sections).
2. Layer catalog benchmarking/CI hooks (handoff to Agent 4 plan).
3. Explore caching/streaming improvements once larger fixture profiling is available.

**Test results**:
```bash
cargo fmt
cargo clippy --all-targets --all-features
cargo test --all
```

### 2025-10-25: Benchmark harness + baseline
**Agent/Contributor**: Agent 4 (Codex)

**Work completed**:
- Delivered `tools/markdown-doc-bench` CLI with configurable warm-up/iteration counts to profile catalog and lint operations.
- Recorded baseline performance against `tests/markdown-doc/wepppy` fixtures (3 iterations, 1 warm-up) and documented results in `docs/markdown-doc/README.md`.
- Verified release build timings (~78 ms catalog, ~77 ms lint) to confirm MVP meets <5 s requirement with ample headroom.
- Captured fixture scale statistics via `cloc` (734 Markdown files, ~82.7k lines) and added them to the documentation for context.

**Blockers encountered**:
- None (CI workflow integration deferred—underlying project relies on parent `/workdir/wepppy` pipelines).

**Next steps**:
1. Monitor benchmarks after major parser/IO changes and refresh documentation as needed.
2. Coordinate with parent project maintainers before adding duplicate CI jobs in this repo.
3. Feed baseline numbers into future performance regression alerts once CI hooks exist.

**Test results**:
```bash
cargo run -p markdown-doc-bench --release -- --path tests/markdown-doc/wepppy
```

### 2025-10-25: CI + Benchmark Harness
**Agent/Contributor**: Agent 4 (Codex)

**Work completed**:
- Added Rust fmt/clippy/test enforcement in the main GitHub Actions workflow and restricted ancillary jobs to push events.
- Created nightly/manual benchmark workflow that runs the new `markdown-doc-bench` binary and captures results as artifacts/summary.
- Implemented reusable benchmark harness (`tools/markdown-doc-bench`) timing catalog and lint operations against the WEPPpy fixtures.
- Documented CI/benchmark usage in the markdown-doc README.

**Blockers encountered**:
- None

**Next steps**:
1. Extend benchmark coverage once additional lint rules land.
2. Track performance trends over time (e.g., add regression thresholds after baseline).
3. Consider caching fixture parsing results if runs exceed targets.

**Test results**:
```bash
cargo fmt
cargo clippy --all-targets --all-features
cargo test --all
cargo run -p markdown-doc-bench -- --iterations 1 --warmup 0
```

## Watch List

- **Fixture freshness**: Schedule periodic refresh of WEPPpy snapshot to catch evolving documentation patterns.
- **Dependency growth**: Monitor compile times as new crates (e.g., `pulldown-cmark`, `tantivy`) are added.

## Communication Log

### 2025-10-25: Kickoff alignment
**Participants**: Codex PM Agent, project owner  
**Question/Topic**: Confirm adoption of work package model for markdown-doc toolkit  
**Outcome**: Greenlit; package scaffolded with initial plan

## Handoff Summary Template

**From**: _<fill during handoff>_  
**To**: _<fill during handoff>_  
**Date**: _<fill during handoff>_

**What's complete**:
- [Describe deliverables]

**What's next**:
1. [Priority task]
2. [Priority task]
3. [Priority task]

**Context needed**:
- [Key background]

**Open questions**:
- [Question needing input]

**Files modified this session**:
- `path/to/file`

**Tests to run**:
```bash
cargo test --all
```
