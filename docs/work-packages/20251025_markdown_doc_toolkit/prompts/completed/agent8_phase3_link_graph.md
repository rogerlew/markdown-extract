# Agent Prompt – Phase 3 Link Graph Engine

## Objective
Deliver the reusable link graph + rewrite core that Phase 3 commands (`markdown-doc mv`, `markdown-doc refs`, future renames) will build on. This engine must analyse Markdown files, build a navigable graph of links/anchors, and expose transactional update helpers without regressing existing lint/catalog/validate behaviour.

## Current Context
- **Parser + anchors already available**: `markdown-doc-parser::ParserContext` emits `DocumentSection`s with normalized headings, anchor slugs, byte ranges, and per-line copies. The lint engine reuses this via `FileSnapshot` (`crates/markdown-doc-ops/src/lint.rs:309`).
- **Link extraction logic exists**: `lint.rs`’s `extract_links` uses `pulldown_cmark` offset iterators to collect `[text](target)` links with byte ranges and line numbers. Broken/broken-anchors rules resolve relative paths and normalize anchor fragments (`split_link_target`, `resolve_relative_path`, `normalize_path`).
- **Anchor caches + ignore plumbing**: `LintEnvironment::anchor_cache` memoizes heading anchors per file, and scan collection already respects config include/exclude patterns plus `.markdown-doc-ignore` (via `Operations::collect_targets` and `load_ignore_filter`, `crates/markdown-doc-ops/src/lib.rs:336`).
- **Diff/atomic write utilities**: `markdown_doc_utils::atomic_write` is used by catalog/toc/update flows; `build_diff` wraps `similar::TextDiff` for user-facing diffs (`crates/markdown-doc-ops/src/lib.rs:575` onward).
- **Phase 2 outcome**: Lint rules now cover `broken-links`, `broken-anchors`, etc., and `README` documents current behaviour. No dedicated refactor module exists yet; commands still only read data.
- **Open bugs**: Workspace tests fail right now because `load_ignore_filter` still calls `GitignoreBuilder::add(...).is_err()` (API now returns `Result`). Fixing this remains a prerequisite for green builds (see Agent 7 notes).

## Gaps to Close
- No shared representation ties together link sources/targets; lint gathers data ad hoc. We need reusable structs capturing source span, resolved path, anchor, and link type.
- Existing link extractor ignores reference-style definitions (`[foo]: path.md`) and images; address or document limitations.
- No backreference map or caching for refactor scenarios—`mv`/`refs` will need fast lookups by target path/anchor.
- Rewrite helpers are absent; we currently lack path normalisation utilities for rename-like operations.
- Tests focus on lint findings only; nothing asserts graph correctness or rewrite previews.

## Responsibilities
1. **Graph construction**
   - Traverse Markdown files selected by `ScanOptions`, respecting include/exclude patterns and `.markdown-doc-ignore` (ensure the helper is fixed first).
   - Capture:
     - Relative/absolute Markdown links (`[text](path.md)`, `[text](../foo.md#anchor)`).
     - Anchor-only links (`#section`), reference-style definitions, images (`![alt](img.png)`), and optionally autolinks (document if skipped).
   - Record for each link: source file, span/byte range, original text, resolved relative path, anchor fragment (normalised via `normalize_anchor_fragment`), link kind.
   - Record for each heading: file, anchor slug, normalized title, byte range.
   - Build a `LinkGraph` (or similar) exposing lookups: `links_from(path)`, `links_to(path)`, `anchors_in(path)`, `backrefs(path|path#anchor)`, etc.
   - Consider caching parser results (e.g., share `DocumentSection`s) to minimise duplicate IO.

2. **Rewrite utilities**
   - Add a refactor module (e.g., `crates/markdown-doc-ops/src/refactor`) that accepts mutation plans:
     - File move/rename (`mv docs/guide.md docs/manual.md`).
     - Anchor rename placeholder (provide API even if impl is stubbed with TODO for actual rename logic).
   - For file moves, compute updated relative paths for inbound/outbound links, respecting OS separators and path normalisation (reuse `normalize_path` or extract helper).
   - Offer dry-run diffs via `similar::TextDiff` consistent with `toc` and future CLI expectations.
   - Ensure writes go through `atomic_write`, supporting optional backups (align with eventual CLI flags).
   - Return structured `RefactorPlan` / `MutationResult` containing per-file edits, diff preview, and no-op detection.

3. **Integration hooks**
   - Expose constructors that accept `&Config`/`ScanOptions` akin to existing operations so CLI layers can instantiate the graph without bespoke plumbing.
   - Make graph building concurrency-friendly (e.g., use Rayon like lint does, or provide serial + parallel options).
   - Define error enums covering missing targets, ambiguous anchors, IO failures, or unsupported link formats.

4. **Testing & validation**
   - Unit tests for link parsing and path resolution (nested dirs, absolute paths, Windows separators).
   - Fixtures under `tests/markdown-doc/refactor/` representing small docs networks (multiple files referencing each other, images, mixed anchor casing).
   - Tests ensuring rewrite plans adjust links correctly for moves (up/down directory tree) and leave unaffected files untouched.
   - Validate backreference queries and ensure duplicate detection is deterministic.
   - Optional benchmark notes if the graph build noticeably impacts runtime.

## Deliverables
- New link graph/refactor modules under `markdown-doc-ops` with documented public APIs (Rustdoc + inline comments where logic is non-obvious).
- Tests covering graph construction, queries, and rewrite planning (unit + integration).
- Documentation update summarising link graph capabilities/limitations and how downstream agents can use the APIs.
- Tracker entry for Agent 8 capturing accomplishments, tests run, and follow-up TODOs.

## Constraints & Notes
- Do **not** add new CLI commands or flags (reserved for later agents), but ensure APIs are ready.
- Reuse existing helpers (`ParserContext`, `atomic_write`, `build_diff`, ignore filters) rather than cloning logic.
- Keep memory use reasonable—consider storing lightweight structs or indexing by `PathBuf`/`Arc<str>` to limit duplication.
- Document unsupported link types (autolinks, HTML) so future work can triage.
- Ensure workspace passes `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features`, and `cargo test --all` once the graph work is complete (fix existing ignore-loader regression as part of this effort).

## Acceptance Criteria
- `LinkGraph` (or equivalent) exposes documented APIs for outgoing/incoming link queries, anchor lookups, and mutation planning.
- Rewrite helpers adjust relative links correctly for representative move scenarios and provide dry-run diffs without mutating disk.
- Tests (unit + integration) cover link detection, backreference queries, path resolution edge cases, and rewrite preview accuracy.
- Documentation (`README` / architecture docs) and tracker contain clear guidance on the new engine, limitations, and future hooks.
- Full workspace checks succeed (`cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features`, `cargo test --all`).

## Handoff Checklist
- Summarise implemented features, tests run, and runtime observations in the tracker’s Agent 8 entry.
- Note any deferred capabilities (e.g., autolink parsing, anchor rename implementation) with TODOs for subsequent agents.
- Provide guidance for Phase 3 CLI implementers (`mv`, `refs`) on instantiating/reusing the graph and rewrite APIs.
