# Agent Prompt – Schema Matcher & `markdown-doc validate`

## Objective
Deliver the Phase 2 schema-matching subsystem and the accompanying `markdown-doc validate` command so documentation files can be checked against structured templates. This work must integrate with the expanded lint engine (Agent 5) and uphold existing performance and UX guarantees.

## Current Context
- Config loader (`markdown-doc-config`) already parses core project/catalog/lint settings and exposes typed structs. Schema-related tables are stubbed in the spec but not yet realised.
- Parser (`markdown-doc-parser`) emits `DocumentSection`s with normalised headings, anchor slugs, byte ranges, and line-level copies – these should be reused for matching to avoid reparsing.
- Lint engine now supports `required-sections` via a schema-provider stub; this implementation must supply real data so the rule surfaces actionable findings.
- CLI currently exposes `catalog` and `lint`; this task introduces the third entry point, `validate`, with dedicated exit codes and outputs.
- Benchmarks show Phase 1 lint/cat runs well under 100 ms on the WEPPpy fixture. Schema validation should keep runs in the same order of magnitude.

## Deliverables
1. **Configuration Enhancements** (crates/markdown-doc-config)
   - Implement `[schemas]` support with:
     - Named schema blocks containing `patterns` (glob array), `required` section definitions (ordered list), optional `allow_additional`, `min_heading_level`, `max_heading_level`, and custom messages.
     - Default schema entry (fallback when no pattern matches).
     - Validation errors for conflicting rules, malformed glob patterns, duplicate schema names, or missing defaults.
   - Extend `Config` to expose a `SchemaSettings` structure (e.g., `HashMap<String, SchemaDefinition>` plus resolved precedence ordering) with provenance metadata.
   - Update loader tests to cover precedence merges, invalid schema configurations, and default resolution.

2. **Schema Matcher Service** (likely in `markdown-doc-ops::schema`)
   - Build a matcher that, given a relative path and parsed sections, resolves the applicable schema using deterministic precedence (recommended: longest/most specific glob, then lexical order; document behaviour).
   - Provide APIs consumed by both lint (`required-sections` rule) and validate:
     ```rust
     pub struct SchemaMatch<'a> { schema: &'a SchemaDefinition, violations: Vec<SchemaViolation>, … }
     pub fn match_document(path: &Path, sections: &[DocumentSection]) -> SchemaMatch
     ```
   - Enforce required section list (presence, ordering, depth bounds) and `allow_additional` logic.
   - Surface violations with rich metadata: missing heading name + expected position, unexpected extra sections, depth mismatches, duplicate occurrences beyond allowed count, etc.
   - Ensure matcher caches computed anchors/headings where useful and is safe for parallel invocation (consider `Arc` + immutable data).

3. **`markdown-doc validate` Command**
   - Add CLI parsing (flags: `--schema <name>` to force a schema, `--path`, `--staged`, `--format {plain,json}`, `--quiet`).
   - Implement execution flow in `markdown-doc-ops` similar to lint: collect targets, run matcher, aggregate findings.
   - Define exit codes:
     - `0` success (no violations)
     - `1` validation failures (errors)
     - `2` schema not found / lookup error
     - `3` config or runtime failure
   - Provide renderers:
     - Plain: human-readable list with schema name, file, line, and message.
     - JSON: align with lint summary structure while incorporating schema names and violation kinds.
   - Update CLI docs/help strings.

4. **Lint Integration**
   - Replace the placeholder in `required-sections` rule (Agent 5) with the matcher so lint shares schema logic. Ensure lint severity/ignore handling still works.
   - Add unit tests ensuring lint findings mirror validate violations when both run on the same document.

5. **Testing Suite**
   - Unit tests for config parsing (`crates/markdown-doc-config/tests/`) covering schema precedence, validation errors, defaults, and glob matching.
   - Unit tests for matcher module verifying:
     - Correct schema selection given overlapping patterns.
     - Detection of missing, out-of-order, over-depth, and extra sections.
     - honouring `allow_additional` and depth bounds.
   - Integration fixtures under `tests/markdown-doc/schemas/` featuring:
     - Valid README adhering to schema.
     - README missing required sections.
     - Work package template with out-of-order section.
     - Schema referencing nested headings.
   - CLI tests (`crates/markdown-doc-cli/tests/`) to assert exit codes, plain output, and JSON structure for success/failure/not-found cases.

6. **Documentation & Tracker Updates**
   - README: add configuration samples for `[schemas]`, validate usage examples, and explanation of exit codes.
   - `docs/markdown-doc/README.md`: architecture section covering schema matcher design, caching, and integration with lint/validate.
   - Work package tracker: record progress entry with testing evidence, decisions, and follow-on TODOs.

## Key Behaviours & Constraints
- **Deterministic matching**: Document precedence rules (e.g., explicit schema selection via `--schema`, otherwise highest-specificity pattern, fallback to default). Ensure tie-breaking is stable.
- **Performance**: Use existing parallelism patterns. Cache schema pattern matchers (`globset::GlobMatcher`) to avoid recompilation per file.
- **Error handling**: Config validation must catch ambiguous setups (multiple defaults, no default, invalid heading levels). Runtime errors (missing schema, parse failures) should bubble up with context.
- **Reusability**: Matcher outputs should be lightweight and shareable; avoid cloning large strings unnecessarily.
- **Future-proofing**: Leave hooks for Agent 7 (severity tuning) by designing violation types with severity levels (error/warning) derived from schema definition.
- **Testing discipline**: Keep tests deterministic. Use small Markdown fixtures focusing on headings.

## Acceptance Criteria
- All deliverables implemented with tests passing (`cargo fmt`, `cargo clippy --all-targets --all-features`, `cargo test --all`).
- `markdown-doc validate` CLI demonstrates correct behaviour across plain/JSON, schema overrides, and error paths.
- Lint `required-sections` rule leverages schema matcher; redundant validation code removed.
- Documentation updated and tracker notes recorded.
- No measurable performance regression (>10% runtime increase) on WEPPpy fixture; capture benchmark if feasible.

## Handoff Checklist
- [x] Config schema extensions merged with coverage.
- [x] Matcher module implemented with unit tests.
- [x] CLI command + renderers added; tests verified.
- [x] Lint integration updated (required-sections).
- [x] Documentation + tracker refreshed.
- [x] Validation of fmt/clippy/test (commands recorded).
