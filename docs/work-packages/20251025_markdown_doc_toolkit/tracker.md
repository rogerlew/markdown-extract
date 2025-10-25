# Tracker – markdown-doc Toolkit

> Living document tracking progress, decisions, risks, and communication for this work package.

## Quick Status

**Started**: 2025-10-25  
**Current phase**: Implementation (Phase 1 foundations)  
**Last updated**: 2025-10-25 (Config loader baseline)  
**Next milestone**: Complete Cross-Cutting Foundations checklist and Phase 1 MVP slice

## Task Board

### Ready / Backlog
- [ ] Implement markdown parsing layer with enriched spans (**Agent 2**)
- [ ] Build lint engine skeleton with rule plug-in architecture (**Agent 3**)
- [ ] Draft CI workflows (fmt, clippy, test, benchmarks) (**Agent 4**)

### In Progress
- [x] Workspace scaffolding, fixtures, and architecture docs seeded (2025-10-25)

### Blocked
- [ ] None currently

### Done
- [x] Bootstrap workspace crates, fixtures, and docs (2025-10-25)
- [x] Flesh out configuration resolver with precedence, defaults, and validation (**Agent 1**, 2025-10-25)

## Timeline

- **2025-10-25** – Package created, initial scaffolding complete
- **TBD** – Phase 1 MVP (`catalog`, `lint broken-links`) delivered
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
