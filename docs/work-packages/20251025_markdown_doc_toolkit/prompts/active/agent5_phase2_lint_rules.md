# Agent Prompt – Phase 2 Lint Rules Expansion

## Objective
Extend the `markdown-doc` lint engine beyond the Phase 1 `broken-links` rule so it enforces all Phase 2 quality gates (`broken-anchors`, `duplicate-anchors`, `heading-hierarchy`, `required-sections`, `toc-sync`) without regressing current CLI behavior, performance targets, or configuration semantics.

## Context & Dependencies
- **Existing stack**: `markdown-doc-config` (layered settings + severity/ignore lists), `markdown-doc-parser` (normalized headings, anchors, byte spans), `markdown-doc-ops` (lint execution and reporting), `markdown-doc-format` (plain/JSON/SARIF emitters), `markdown-doc-cli`.
- **Parser guarantees** (Agent 2): every `DocumentSection` already carries normalized heading text, anchor slug, byte ranges, code-block awareness, and TOC markers. This prompt relies on reusing that metadata; do not re-parse files.
- **Shared infrastructure**: Rayon executor for parallel scans, atomic writer utilities, benchmark harness (`tools/markdown-doc-bench`).
- **Upstream coordination**: Agent 6 will deliver schema matcher APIs. This prompt must define the integration points so `required-sections` can consume those results once available (stub acceptable with TODO when schema matcher is pending).

## Deliverables
- Rule plug-in architecture inside `markdown-doc-ops`:
  - Introduce a `LintRule` trait (or equivalent enum dispatcher) with `name`, `severity(config)`, `run(file_ctx) -> Vec<LintFinding>`, and wiring into the lint runner.
  - Register new rules with config-driven enable/disable logic and ignore pattern filtering (per file + per finding ID).
- Implement five Phase 2 rules using parser metadata:
  1. **broken-anchors** – unresolved intra-file anchors (`#foo`, `[text](#foo)`), cross-document anchors (`docs/guide.md#foo`), and reference-style links. Provide fix suggestions (nearest heading match) when helpful.
  2. **duplicate-anchors** – report duplicate anchor slugs within a file (case-insensitive); include pointer to first definition.
  3. **heading-hierarchy** – detect level skips (H2 → H4) and headings exceeding `lint.max_heading_depth`; respect per-path ignores and inline suppressions (if supported).
  4. **required-sections** – consume schema matcher output (once Agent 6 lands) or temporarily gate behind feature flag; ensure missing sections, wrong order, and depth mismatches surface with actionable messages.
  5. **toc-sync** – compare declared TOC blocks (between configurable markers, default `<!-- toc -->` / `<!-- tocstop -->`) with actual heading structure; highlight additions/removals/out-of-order entries.
- Extend configuration layer (`markdown-doc-config`) only if additional fields are required:
  - Rule toggle/override structure is already present; add rule-specific sections when necessary (e.g., TOC markers, schema references).
  - Document any new config keys in README + `docs/markdown-doc/README.md`.
- Update CLI outputs:
  - Plain formatter shows combined findings with rule names and severities.
  - JSON/SARIF outputs include new rule types, obey severity mapping, and expose supplementary data (e.g., anchor slug, expected heading).
- Comprehensive test suite:
  - Unit tests per rule (positives, negatives, ignore handling).
  - Integration fixtures under `tests/markdown-doc/lint_phase2/` covering mixed-rule scenarios and config overrides.
  - Snapshot or JSON assertions for `broken-anchors` and `toc-sync` to ensure structured payloads remain stable.
- Documentation updates:
  - README quickstart + `docs/markdown-doc/README.md` sections describing each rule, configuration options, and sample outputs.
  - Update work-package tracker with progress notes and performance observations.

## Implementation Strategy
### 1. Rule Infrastructure
1. Introduce `markdown_doc_ops::lint::rule` module encapsulating:
   - `LintRule` trait or enum with `fn evaluate(&self, ctx: &FileContext, cfg: &LintRuleConfig) -> Vec<Finding>`.
   - Shared helpers for severity resolution, ignore checks, anchor normalization, TOC parsing.
2. Modify the lint runner to:
   - Build active rule set based on `Config::lint.rules` map (reusing severity/ignore logic from Phase 1).
   - Execute rules in parallel per file when beneficial; aggregate findings while preserving deterministic ordering (e.g., sort by byte offset, then rule).
   - Update summary counts to reflect new rule severities.

### 2. Data Requirements
- Ensure parser exposes (or confirm existing access to) the following:
  - Anchor slug per heading (`heading.anchor_slug`), list of inbound links with span + target.
  - TOC block spans + parsed bullet structure (if not available, extend parser accordingly).
  - Within-file section ordering and depth metadata (`DocumentSection.depth`, `DocumentSection.byte_range`).
- Create lightweight caches per file (e.g., `AnchorsIndex`, `LinksIndex`, `HeadingHierarchy`) to avoid recomputation across rules. Store them in `FileContext`.

### 3. Rule Details
- **broken-anchors**
  - Collect all anchor references: inline links (`[text](#slug)`), reference links, HTML anchors (`<a href="#slug">`), and cross-doc links.
  - Resolve to either (a) local headings, (b) include/external references (defer to broken-links if HTTP). For cross-doc anchors, ensure target file parse results provide anchor slug list (may require two-pass cache keyed by path).
  - Emit finding when anchor missing; message includes suggestion (`Did you mean '#existing-anchor'?`) using Levenshtein or normalized comparison (`strsim` crate already in workspace? add if needed).

- **duplicate-anchors**
  - Build map of slug → list of heading spans. If list > 1, emit findings for duplicates (exclude first occurrence or mark as source).
  - Provide context: `Duplicate anchor 'overview' also defined at line X`.

- **heading-hierarchy**
  - Iterate headings in document order; compare depth difference between adjacent headings.
  - Record skip when `next_depth > current_depth + 1`.
  - Validate against `max_heading_depth` (pull from config; default maybe 6). Provide fix hints.

- **required-sections**
  - Integrate with schema matcher via trait/adapter:
    ```rust
    trait SchemaProvider {
        fn matches(&self, path: &Path) -> Option<SchemaMatch>;
    }
    ```
  - Until Agent 6 lands, mock provider returning `None` (feature flag) or create stub with TODO comment referencing dependency.
  - Findings should capture missing headings, wrong order, duplicates beyond allowed count. Each message should include expected heading text and location context.

- **toc-sync**
  - Leverage parser-collected TOC section (if not present, parse between markers as raw Markdown).
  - Normalize actual heading tree to list (`Vec<TocEntry>`). Compare with declared TOC entries:
    - Missing entry → severity per config.
    - Extra entry → warn (include anchor).
    - Out-of-order → specify expected neighbor.
  - Provide auto-fix hint referencing future `toc` command.

### 4. Configuration & UX Updates
- Extend `markdown-doc-config` if needed:
  - `lint.toc.markers` (start/end strings, default `<!-- toc -->`/`<!-- tocstop -->`).
  - `lint.heading.max_depth` (existing?) ensure rule reads from config.
  - `lint.required_sections.schema` mapping (if not already defined).
- Ensure CLI help text enumerates new rules and flags (if new toggles).
- Update JSON schema (if maintained) for config.

### 5. Performance & Reliability
- Reuse Rayon for file-level parallelism; avoid O(n²) comparisons by using hash maps.
- Guard cross-file anchor resolution with shared cache (Arc + dashmap?) but default to sequential fallback if complexity high.
- Add benchmarks (optional) to confirm lint run remains <5 s on WEPPpy fixture; log results in tracker.
- Maintain broken pipe handling (no change expected).

## Testing Strategy
- **Unit Tests** (under each rule module):
  - Synthetic documents for straightforward assertions (e.g., duplicate anchors, hierarchy).
  - Mock config to verify severity overrides + ignore behavior.
  - Negative cases to ensure rules ignore code blocks/front matter as parser already handles.
- **Integration Fixtures** (`tests/markdown-doc/lint_phase2/`):
  - `anchors.md` – mix of valid/missing anchors, cross-file references.
  - `dupe_anchors.md` – slug collisions with varying case.
  - `hierarchy.md` – heading level gaps, over-depth examples.
  - `required_sections/` – align with schema definitions (once Agent 6 ready); include passing/failing samples.
  - `toc.md` – TOC markers with mismatched entries.
  - Config TOMLs enabling/disabling rules, severity variations, ignore lists.
- **CLI Acceptance**:
  - Extend existing `markdown-doc-cli/tests/lint.rs` (or add) verifying combined rule output, exit codes (error when severity ≥ error), JSON/SARIF snapshots.
- **Regression**:
  - Ensure `cargo test --all`, `cargo clippy --all-targets --all-features`, and `cargo fmt` pass.
  - Optional: run benchmark harness and capture results in tracker.

## Documentation Updates
- README top-level highlights new lint capabilities; include example command and excerpted output.
- `docs/markdown-doc/README.md`:
  - Detailed rule descriptions, configuration snippets, sample findings.
  - Cross-reference schema matcher for required sections (call out dependency).
  - Performance notes if benchmarks change.
- Update work-package tracker `tracker.md` with progress log entries (dates, tests run, benchmark results).

## Risks & Mitigations
- **Schema dependency** (Agent 6 not ready): design rule to gracefully no-op or operate with stub provider; clearly note TODO for integration.
- **Performance regression**: monitor cross-file anchor lookups; cache anchor sets lazily and share across rules.
- **Config complexity**: document defaults and provide sample config; add validation to prevent conflicting markers or unsupported rules.
- **Output compatibility**: keep JSON/SARIF schema stable; add tests to guard against accidental breaking changes.

## Handoff Checklist
- [ ] Rule infrastructure merged with unit coverage.
- [ ] Individual rule modules implemented with documentation comments.
- [ ] Config schema & CLI help updated.
- [ ] Integration tests + snapshots added; `cargo test --all` passing.
- [ ] Tracker updated with summary, benchmarks (if run), remaining follow-ups.
- [ ] README + docs refreshed describing new rules and configuration.

