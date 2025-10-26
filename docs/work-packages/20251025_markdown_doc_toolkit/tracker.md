# Tracker – markdown-doc Toolkit

> Living document tracking progress, decisions, risks, and communication for this work package.

## Quick Status

**Started**: 2025-10-25  
**Current phase**: Phase 3 – Refactoring support in progress  
**Last updated**: 2025-10-27 (Link graph + `mv` command)  
**Next milestone**: Deliver `markdown-doc refs` and stress-test fixtures

## Task Board

### Ready / Backlog
- [ ] Evaluate CI workflow alignment with parent `/workdir/wepppy` pipelines before duplicating jobs (**PM/Agent 4**)
- [ ] Identify additional fixtures for Phase 3 stress tests (nested directories, mixed links) (**Future Agent**)
- [ ] Implement `markdown-doc refs` command & stress-test fixtures (**Agent 10**) – `prompts/active/agent10_phase3_refs.md`

### In Progress
- [ ] Monitor benchmark baseline; rerun after significant parser/IO changes

### Blocked
- [ ] None currently

- [x] Bootstrap workspace crates, fixtures, and docs (2025-10-25)
- [x] Flesh out configuration resolver with precedence, defaults, and validation (**Agent 1**, 2025-10-25)
- [x] Implement markdown parsing layer with enriched spans (**Agent 2**, 2025-10-25)
- [x] Ship catalog generation + broken-links lint CLI slice (**Agent 3**, 2025-10-25)
- [x] Benchmark harness delivered + baseline captured (**Agent 4**, 2025-10-25)
- [x] Acceptance testing + README quickstart documentation (Claude, 2025-10-25)
- [x] Phase 2 lint rule suite (broken-anchors, duplicate-anchors, heading-hierarchy, toc-sync) landed (**Agent 5**, 2025-10-26)
- [x] Schema matcher + `markdown-doc validate` command delivered (**Agent 6**, 2025-10-26)
- [x] TOC command, severity tuning, and `.markdown-doc-ignore` support shipped (**Agent 7**, 2025-10-26)
- [x] Link graph engine & rewrite planning utilities implemented (**Agent 8**, 2025-10-27)
- [x] `markdown-doc mv` command delivered with transactional rewrites (**Agent 9**, 2025-10-27)
- [x] Phase 3 documentation refresh (mv command comprehensive coverage) (**Claude**, 2025-10-27)

## Timeline

- **2025-10-25** – Package created, initial scaffolding complete
- **2025-10-25** – Phase 1 MVP (`catalog`, `lint broken-links`) delivered
- **2025-10-26** – Phase 2 quality gates (lint rule expansions) released
- **2025-10-26** – Schema matcher + validate command available for template enforcement
- **2025-10-27** – Link graph + `mv` command shipped (Phase 3 kickoff)
- **TBD** – `refs` command & stress fixtures delivered
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

### 2025-10-25: Acceptance testing & README updates
**Agent/Contributor**: Claude (QA/Docs)

**Work completed**:
- Executed acceptance sweep (`cargo fmt`, `cargo clippy --all-targets --all-features`, `cargo test --all`) against the updated workspace.
- Performed manual CLI spot-checks for `markdown-doc catalog` (Markdown/JSON) and `markdown-doc lint` (plain/JSON, staged/path filters) using WEPPpy fixtures.
- Authored README sections covering markdown-doc quickstart, configuration examples, agent workflows, and performance benchmarks.

**Blockers encountered**:
- None; tooling behaved per spec.

**Next steps**:
1. PM review of documentation tone/coverage (pending).
2. Plan Phase 2 briefs (expanded lint rules) now that MVP docs are in place.

**Test results**:
```bash
cargo fmt
cargo clippy --all-targets --all-features
cargo test --all
markdown-doc catalog --format json --path README.md
markdown-doc lint --path docs/ --format plain
```

### 2025-10-25: Phase 2 planning & prompt prep
**Agent/Contributor**: Codex PM Agent

**Work completed**:
- Marked Phase 1 items complete in plan/tracker and captured benchmark + documentation baselines.
- Authored Phase 2 prompts for lint rule expansion (Agent 5), schema matcher/validate (Agent 6), and TOC & severity tuning (Agent 7).
- Updated task board backlog to reflect upcoming assignments and remaining PM doc review.

**Blockers encountered**:
- None – awaiting new workers for Phase 2 execution.

**Next steps**:
1. Brief incoming agents with the prepared prompts and coordinate sequencing.
2. Complete PM review pass on README updates (assigned in backlog).
3. Align with `/workdir/wepppy` maintainers before adjusting CI workflows in this repo.

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

### 2025-10-26: Phase 2 lint rule implementation
**Agent/Contributor**: Agent 5 (Codex)

**Work completed**:
- Refactored lint engine to a pluggable rule pipeline, adding support for `broken-anchors`, `duplicate-anchors`, `heading-hierarchy`, and `toc-sync` alongside the existing `broken-links` rule.
- Extended configuration (`lint.toc_start_marker`/`lint.toc_end_marker`) and renderer output so findings carry rule identifiers across plain/JSON/SARIF formats.
- Added unit/integration coverage for the new rules and updated README + architecture docs to describe behaviour and configuration knobs.

**Blockers encountered**:
- None (required-sections rule currently stubs pending schema matcher delivery).

**Next steps**:
1. Integrate Agent 6 schema matcher so `required-sections` surfaces actionable findings.
2. Expand fixtures covering cross-file anchors and large TOC documents for performance benchmarking.
3. Monitor lint runtime on WEPPpy corpus after schema integration lands.

**Test results**:
```bash
cargo fmt
cargo clippy --all-targets --all-features
cargo test --all
```

### 2025-10-26: Schema matcher + validate command
**Agent/Contributor**: Agent 6 (Codex)

**Work completed**:
- Added schema-aware lint pipeline backed by a reusable `SchemaEngine`, replacing the placeholder `required-sections` logic.
- Implemented `markdown-doc validate` with plain/JSON outputs, exit code contract, and CLI bindings (`--schema`, `--format`, `--quiet`).
- Extended configuration loader to parse `[schemas]` definitions (patterns, required sections, depth bounds, allow flags), plus documentation and tracker updates.
- Added unit/integration tests covering schema matching, validate CLI behaviour, and fixture-based validation scenarios.

**Blockers encountered**:
- None.

**Next steps**:
1. Integrate Agent 6 schema matcher with upcoming deep validation features (min/max heading levels, child section constraints).
2. Monitor runtime impact across the WEPPpy corpus and expand fixtures as new schemas are added.
3. Coordinate with Agent 7 on severity tuning and `.markdown-doc-ignore` support.

**Test results**:
```bash
cargo fmt
cargo clippy --all-targets --all-features
cargo test --all
```

### 2025-10-27: Link graph + rewrite planning
**Agent/Contributor**: Agent 8 (Codex)

**Work completed**:
- Added a public `refactor::graph` module that captures anchors, inline links, and reference definitions with byte spans plus anchor-aware backreferences.
- Implemented `refactor::rewrite::plan_file_moves`, producing `RewritePlan`/`FileEdit` payloads so future CLI commands can adjust relative links during renames.
- Shared path utilities across lint + refactor (`paths.rs`) and exposed `Operations::link_graph` for consumers.
- Created integration tests (`crates/markdown-doc-ops/tests/graph.rs`, `crates/markdown-doc-ops/tests/rewrite.rs`) exercising graph traversal and rename rewrites.
- Documented the new engine in `README.md` and `docs/markdown-doc/README.md`.

**Blockers encountered**:
- None.

**Next steps**:
1. Layer CLI commands (`markdown-doc mv`, `markdown-doc refs`) on top of the new refactor APIs.
2. Extend rewrite planning to handle anchor renames and multi-file payload edits once specifications land.
3. Consider caching link graphs across operations to avoid repeated parsing for large repositories.

**Test results**:
```bash
cargo test -p markdown-doc-ops
```

### 2025-10-28: `markdown-doc mv` command
**Agent/Contributor**: Agent 9 (Codex)

**Work completed**:
- Wired `markdown-doc mv` CLI (dry-run, JSON, force, no-backup, no-ignore, quiet flags) to the refactor engine.
- Extended ops layer with transactional rename support, including backups, rollback logic, and ignore-aware graph construction.
- Added integration tests for mv (ops + CLI) covering dry-run diffs, backups, ignore handling, and link rewrites.
- Documented mv usage in README + architecture primer and refreshed Agent prompt.

**Blockers encountered**:
- None.

**Next steps**:
1. Build `markdown-doc refs` on top of the same engine (Phase 3 follow-up).
2. Incorporate directory move support and batched operations in future agents.
3. Explore caching link graphs across successive refactor commands for large repositories.

**Test results**:
```bash
cargo fmt
cargo clippy --all-targets --all-features
cargo test --all
```

### 2025-10-26: TOC command & severity tuning
**Agent/Contributor**: Agent 7 (Codex)

**Work completed**:
- Introduced `markdown-doc toc` with `--check`, `--update`, `--diff` modes, selective scanning, and support for `.markdown-doc-ignore` filtering.
- Extended configuration to support wildcard severity overrides (`[lint.severity_overrides]`) and per-path ignore semantics reused by lint/TOC.
- Implemented TOC parsing/rendering helpers, ignore-file resolution, and severity override plumbing in the lint runner; added integration tests for TOC diff/update and ignore precedence.
- Updated README + architecture docs to document TOC usage, ignore files, and severity tuning examples.

**Blockers encountered**:
- None; coordination with Agents 5/6 handled via shared helpers.

**Next steps**:
1. Monitor TOC performance on large repos and expand fixtures if necessary.
2. Explore exposing TOC depth filters as future enhancement (tracked in backlog).
3. Surface severity override examples in quickstart once PM review completes.

**Test results**:
```bash
cargo fmt
cargo clippy --all-targets --all-features
cargo test --all
markdown-doc toc --path README.md --check
```

### 2025-10-27: Link graph engine delivery
**Agent/Contributor**: Agent 8 (Codex)

**Work completed**:
- Implemented `LinkGraph` module capturing anchors, links, and backreferences across Markdown files (respecting config filters and `.markdown-doc-ignore`).
- Added rewrite planning helpers (`rewrite::plan_move`) that compute updated contents for file moves, supporting dry-run diffs and atomic write stages for downstream commands.
- Added unit/integration tests under `crates/markdown-doc-ops/tests/graph.rs` and `tests/markdown-doc/refactor/` covering nested directories, reference links, and anchors.
- Documented the API via Rustdoc and updated architecture notes to describe the refactoring engine.

**Blockers encountered**:
- None; noted TODO for future anchor-renaming support.

**Next steps**:
1. Hand off rewrite utilities to Agent 9 for `markdown-doc mv`.
2. Evaluate caching strategies if multiple commands request the graph in a single run.

**Test results**:
```bash
cargo fmt
cargo clippy --all-targets --all-features
cargo test --all
```

### 2025-10-27: `markdown-doc mv` implementation
**Agent/Contributor**: Agent 9 (Codex)

**Work completed**:
- Added CLI/ops implementation for `markdown-doc mv`, supporting `--dry-run`, `--force`, `--no-backup`, `--quiet`, and `--format json`.
- Leveraged the link graph rewrite planner to update relative links and anchors when moving files; ensured transactional writes with temp files and optional `.bak` copies.
- Added integration tests (`tests/mv.rs`, CLI tests) covering dry-run diffs, backup handling, ignore filters, and JSON output.
- Updated README and architecture documentation with usage examples and agent workflows.

**Blockers encountered**:
- None; directory moves deferred to future work (tracked in backlog).

**Next steps**:
1. Collaborate with Agent 10 on `refs`, reusing graph APIs.
2. Monitor performance on large refactors; consider graph caching if needed.

**Test results**:
```bash
cargo fmt
cargo clippy --all-targets --all-features
cargo test --all
markdown-doc mv docs/source.md docs/destination.md --dry-run
```

### 2025-10-26: Phase 3 kickoff planning
**Agent/Contributor**: Codex PM Agent

**Work completed**:
- Authored Phase 3 prompts for link graph engine (Agent 8), `markdown-doc mv` (Agent 9), and `markdown-doc refs` + stress fixtures (Agent 10).
- Updated implementation plan and tracker backlog with new assignments and immediate next steps.
- Confirmed workspace is clean after README QA pass; ready for next agent wave.

**Blockers encountered**:
- None – awaiting assignment of Phase 3 agents and README QA confirmation.

**Next steps**:
1. Assign Agents 8–10 and coordinate deliverable sequencing (graph → mv → refs).
2. After prompt completion, refresh plan/timeline and evaluate remaining backlog (CI alignment, Phase 4 scoping).

### 2025-10-26: README Quickstart Review & Validation
**Agent/Contributor**: Claude (QA/Documentation Review)

**Work completed**:
- Comprehensive content review of all markdown-doc sections in README.md, validating terminology alignment with CLI help text, exit codes, and configuration options.
- Hands-on verification of all four commands (`catalog`, `lint`, `validate`, `toc`) with multiple flags and output formats.
- Validated JSON/SARIF schema examples against actual tool output; confirmed structural accuracy and field naming.
- Tested integration examples including Python code snippets, bash patterns, and CI/CD configurations for syntactic correctness.
- Cross-verified configuration TOML examples against the implemented config loader.

**Test results**:
```bash
# Catalog testing
cargo run -p markdown-doc-cli --release -- catalog --path README.md --format json 2>/dev/null | jq '.summary'
cargo run -p markdown-doc-cli --release -- catalog --path tests/markdown-doc/wepppy/AGENTS.md --format markdown

# Lint testing (all formats)
cargo run -p markdown-doc-cli --release -- lint --path README.md --format json 2>/dev/null | jq '.summary'
cargo run -p markdown-doc-cli --release -- lint --path markdown-doc.spec.md --format json 2>/dev/null | jq '.findings[0]'
cargo run -p markdown-doc-cli --release -- lint --format sarif 2>/dev/null | jq '{version, schema: ."$schema"}'

# Validate testing
cargo run -p markdown-doc-cli --release -- validate --path README.md 2>/dev/null

# TOC testing
cargo run -p markdown-doc-cli --release -- toc --check 2>/dev/null | head -10
```

**Findings**:

✅ **PASS - Documentation Accuracy**
- JSON schema examples match actual output structure (keys, types, nesting)
- SARIF 2.1.0 format correctly documented with proper schema reference
- Exit codes documented (0, 1, 2, 3) align with CLI behavior
- Flag names and descriptions match `--help` output exactly
- Configuration TOML examples use valid keys recognized by the loader

✅ **PASS - Code Examples**
- Python integration snippets are syntactically correct
- Bash examples use proper quoting and escaping
- GitHub Actions/GitLab CI YAML is well-formed
- jq filter patterns work against actual JSON output

✅ **PASS - Cross-References**
- Links to markdown-edit.spec.md are valid
- Anchor references within README are correct
- All internal documentation links resolve

⚠️ **MINOR ISSUE - Missing Documentation**
- **TOC command is not documented** in README despite being fully implemented and working
  - Command exists: `markdown-doc toc [--check|--update|--diff]`
  - Supports `--path`, `--staged`, `--no-ignore`, `--quiet` flags
  - Returns clear output for missing markers, sync status
  - Should be added as subsection under "Commands" alongside `catalog`/`lint`/`validate`

⚠️ **MINOR ISSUE - Validate Command Details**
- Validate command is documented in README but lacks:
  - Output format examples (plain vs JSON)
  - Schema configuration examples from the actual TOML
  - Typical error messages agents might encounter
  - The `--no-ignore` and `--quiet` flags are not mentioned

**Recommendations**:

1. **Add TOC Command Section** (high priority)
   ```markdown
   #### `toc` - Table of Contents Synchronization
   
   Manages TOC blocks between `<!-- toc -->` and `<!-- tocstop -->` markers.
   
   ```console
   # Check if TOCs are in sync (default)
   $ markdown-doc toc --check
   
   # Update TOC blocks in place
   $ markdown-doc toc --update
   
   # Show what would change (unified diff)
   $ markdown-doc toc --diff
   
   # Target specific paths
   $ markdown-doc toc --path docs/ --update
   ```
   ```

2. **Expand Validate Documentation** (medium priority)
   - Add JSON output example
   - Show sample schema definition from TOML
   - Document common validation errors

3. **Add Agent Quick Reference Table** (nice-to-have)
   - Command matrix showing exit codes for each subcommand
   - Output format compatibility matrix

**Blockers encountered**:
- None; all documented features work as described.

**Next steps**:
1. PM to add TOC command documentation to README.
2. Consider expanding validate section with more examples.
3. Phase 2 documentation appears complete pending TOC addition.

**Files reviewed**:
- `README.md` (markdown-doc sections, lines ~395-750)
- CLI help output for all commands
- Sample JSON/SARIF outputs

**Verification status**: ✅ **PASS** with minor documentation gaps (TOC command missing, validate could be expanded)

### 2025-10-26: README Documentation Gap Fix (TOC & Validate)
**Agent/Contributor**: Claude (Documentation)

**Work completed**:
- Added comprehensive `markdown-doc toc` command section to README.md covering all operation modes (`--check`, `--update`, `--diff`), configuration, output examples, exit codes, and ignore filtering behavior.
- Enhanced `markdown-doc validate` section with complete flag documentation (`--quiet`, `--no-ignore`, `--staged`), JSON output schema example, detailed schema configuration samples, common error messages, and exit code reference.
- Verified consistency with CLI help output (`toc --help`, `validate --help`) to ensure flag names and descriptions match implementation.
- Confirmed documentation style aligns with existing sections (emoji usage, code block formatting, table structure).
- Validated no broken links introduced via lint check.

**Documentation additions**:

1. **TOC Command Section** (~50 lines)
   - Three operation modes explained with examples
   - Configuration markers (`toc_start_marker`/`toc_end_marker`)
   - Output examples for check/diff modes
   - Exit codes and ignore filtering behavior

2. **Validate Command Enhancements** (~60 lines)
   - Complete flag reference including `--quiet`, `--no-ignore`, `--staged`
   - JSON output schema with representative findings
   - Common error messages table with explanations
   - Detailed schema configuration examples (default, readme, agents schemas)
   - Schema configuration keys (patterns, required_sections, allow flags, depth constraints)

**Blockers encountered**:
- None; all features documented work as described in CLI.

**Verification commands**:
```bash
# Confirmed CLI help matches documentation
cargo run -p markdown-doc-cli --release -- toc --help
cargo run -p markdown-doc-cli --release -- validate --help

# Verified JSON output structure
cargo run -p markdown-doc-cli --release -- validate --path README.md --format json 2>/dev/null

# Confirmed no broken links in updated README
cargo run -p markdown-doc-cli --release -- lint --path README.md 2>/dev/null

# Verified new sections appear in catalog
cargo run -p markdown-doc-cli --release -- catalog --path README.md --format json 2>/dev/null | \
  jq -r '.files[0].headings[] | select(.text | contains("toc") or contains("validate"))'
```

**Next steps**:
1. Phase 2 documentation now complete with all four commands fully documented.
2. Consider adding agent quick reference table showing command × exit code matrix (deferred as nice-to-have).
3. README ready for Phase 2 release.

**Files modified**:
- `README.md` (added TOC section, enhanced validate section, ~110 lines added)
- `docs/work-packages/20251025_markdown_doc_toolkit/tracker.md` (this entry)

**Verification status**: ✅ **COMPLETE** - All Phase 2 documentation gaps closed

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

### 2025-10-27: README Phase 3 Documentation Refresh (mv Command)
**Agent/Contributor**: Claude (Documentation)

**Work completed**:
- Enhanced `markdown-doc mv` command documentation in README.md from brief (~23 lines) to comprehensive (~91 lines) coverage matching the quality level of catalog/lint/validate/toc sections.
- Added detailed command variations covering all flags (`--dry-run`, `--force`, `--no-backup`, `--json`, `--quiet`, `--no-ignore`).
- Documented what gets updated (inbound/outbound links, images, reference definitions, anchor fragments) with checklist format.
- Provided before/after example scenario showing link rewrites in both directions.
- Included JSON output schema with representative structure (`status`, `original`, `output`, `files_updated`, `diff`).
- Listed exit codes (0/1/4) with explanations for each code.
- Documented safety features (dry-run, automatic backups, atomic writes, rollback, ignore filtering) with detailed explanations.
- Added "Common workflows" section with three practical examples (safe rename, directory move, automation-friendly).
- Verified architecture doc (`docs/markdown-doc/README.md`) already has comprehensive link graph + refactor engine coverage from Agent 8/9 work; no updates needed.

**Documentation additions**:

1. **mv Command Section Expansion** (+68 lines)
   - 6 command variations (was 3)
   - "What gets updated" checklist (5 items with emoji markers)
   - Before/after example showing bi-directional link rewriting
   - JSON schema with sample output
   - Exit codes table (0/1/4)
   - Safety features list (5 items with bold headings)
   - Common workflows section (3 examples)

**Verification commands**:
```bash
# Verified CLI flags match documentation
cargo run -p markdown-doc-cli --release -- mv --help

# Confirmed section structure and line count
markdown-extract 'mv.*Safe' README.md | wc -l  # Output: 91 lines

# Validated enhanced documentation reads correctly
markdown-extract 'mv.*Safe' README.md | head -30
markdown-extract 'mv.*Safe' README.md | tail -30

# Confirmed total README length increased appropriately
wc -l README.md  # Now 1188 lines (was 1121, +67 lines)
```

**Integration with Phase 3 work**:
- Builds on Agent 8's link graph engine documentation (already in architecture doc)
- Complements Agent 9's CLI implementation (verified via `--help` output)
- Aligns with `claude_phase3_docs_refresh.md` prompt requirements
- Architecture doc already has "Refactor Engine (Link Graph + Rewrite)" section with LinkGraph/RewritePlan coverage; no updates needed

**Task board updates**:
- [x] Phase 3 documentation refresh (human-friendly pass) (**Claude**) – moved to completed

**Blockers encountered**:
- `replace_string_in_file` tool failed repeatedly due to whitespace matching issues; resolved by using `markdown-edit` CLI tool for the replacement operation (dogfooding FTW).

**Next steps**:
1. Phase 3 `mv` command fully documented and ready for use.
2. When Agent 10 delivers `markdown-doc refs`, add similar comprehensive section.
3. Consider adding link graph utility examples to README if refs command exposes them (deferred until refs lands).
4. README now has complete coverage: catalog/lint/validate/toc/mv all documented at similar depth.

**Files modified**:
- `README.md` (enhanced mv section, +67 lines, total now 1188 lines)
- `docs/work-packages/20251025_markdown_doc_toolkit/tracker.md` (this entry)

**Verification status**: ✅ **COMPLETE** - Phase 3 mv documentation comprehensive and aligned with claude_phase3_docs_refresh.md requirements

