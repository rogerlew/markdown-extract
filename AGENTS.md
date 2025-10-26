AGENT NOTES: markdown-extract Architecture
=========================================

Overview
--------
- `markdown-extract` is a small Rust workspace built around reading Markdown files and returning complete sections whose headings match a regex.
- It ships as two crates: a reusable library (`crates/markdown-extract`) and a CLI wrapper (`crates/markdown-extract-cli`) that exposes the functionality via a command-line interface.
- Core guarantees: headings inside fenced code blocks are ignored, matches span the heading line through the section body, and operations succeed even when downstream pipes close.

Workspace layout
----------------
- `crates/markdown-extract`: library crate that implements parsing and extraction.
- `crates/markdown-extract-cli`: binary crate providing the `markdown-extract` CLI. Depends on the library crate.
- `tests/markdown`: shared fixtures for integration tests (e.g., multiple matches, headings inside code blocks).
- `target/`: build artifacts; not relevant to architecture.

Library crate (`crates/markdown-extract`)
-----------------------------------------

Public API
~~~~~~~~~~
- `extract_from_path(path: &PathBuf, regex: &Regex) -> Result<Vec<MarkdownSection>, io::Error>`  
  Opens the file, wraps it in a `BufReader`, and delegates to `extract_from_reader`.
- `extract_from_reader(reader: &mut BufReader<R>, regex: &Regex) -> Vec<MarkdownSection>`  
  Scans the Markdown stream line by line, using an internal state machine to accumulate sections whose headings match the regex. Returns a list of sections, where each section is `Vec<String>` containing the heading line plus its body.

Key modules
~~~~~~~~~~~
- `heading.rs`  
  - Defines `MarkdownHeading { depth: usize, content: String }`.  
  - `try_parse_heading(line: &str) -> Option<MarkdownHeading>` counts leading `#` characters to determine heading depth and trims the remaining text.
- `state.rs`  
  - Encapsulates mutable traversal state: current matches, whether the parser is inside a match, current heading depth, code-block toggle, and the section accumulator.  
  - `enter_matched_section` starts a fresh accumulator whenever a matching heading is encountered, closing out any previous section.  
  - `exit_matched_section` flushes the current accumulator when a higher/equal-depth heading is seen.
- `lib.rs` orchestrates the scan:  
  - Tracks fenced code blocks by flipping a boolean when a line starts with triple backticks (`line.starts_with("```")`).  
  - Only attempts heading parsing when not inside a code block.  
  - Handles hierarchical boundaries by comparing heading depth; entering a new heading of the same or higher depth ends the current match.

Notable behaviors
~~~~~~~~~~~~~~~~~
- Code blocks: simple heuristic—any line starting with ``` toggles the flag. Does not account for indented code blocks or different fencing languages.
- Matching: relies on the caller-provided `Regex`; the library neither normalizes nor pre-processes heading text beyond trimming whitespace.
- Output ordering: sections are emitted in the order encountered. Each section contains exact source lines (no newline normalization).
- Error handling: `extract_from_reader` never returns an error; callers must ensure the reader supplies valid UTF-8 lines.

CLI crate (`crates/markdown-extract-cli`)
-----------------------------------------

Command-line surface
~~~~~~~~~~~~~~~~~~~~
- Built with `clap`. Important flags:  
  - `--all / -a`: print every matching section instead of stopping after the first match.  
  - `--case-sensitive / -s`: disable case-insensitive matching (default is case-insensitive).  
  - `--no-print-matched-heading / -n`: omit the heading line from the output.  
  - Positional `PATTERN` (regex) and `FILE` path.
- Regex builder: enforces a 100 KB compiled-regex size limit and flips case sensitivity based on the flag.
- Execution flow: parse CLI → build regex → call `extract_from_path` → bail with `anyhow` error if no matches → print sections.
- Printing logic:  
  - Writes each line using a locked `stdout`.  
  - Suppresses errors caused by broken pipes (`ErrorKind::BrokenPipe`/`WouldBlock`) to support piping into `head`/`less`.

Testing
-------
- Library integration tests (`crates/markdown-extract/tests/extract.rs`):  
  - Verifies multiple matches are captured and ordered correctly.  
  - Ensures headings inside fenced code blocks are ignored.
- CLI test (`crates/markdown-extract-cli/tests/broken_pipe.rs`): confirms graceful handling when downstream consumers close the pipe early.
- Unit tests in `heading.rs` cover heading detection for positive and negative cases.

Opportunities & reuse for `markdown-edit`
-----------------------------------------
- Heading parsing: `try_parse_heading` already captures depth and trimmed content; this will be useful for locating insertion points and enforcing heading-level invariants.
- Section span logic: `State`’s depth tracking models Markdown hierarchy; we can extend it to capture byte offsets or ranges required for edits.
- Code block awareness: reuse the fenced-block toggle to avoid editing sections inside code blocks inadvertently.
- CLI scaffolding: existing `clap` usage and error handling (especially pipe-friendly output, regex options) provides a template for the future `markdown-edit` binary.
- Testing patterns: fixtures and integration tests demonstrate how to structure Markdown samples; can be expanded to cover edit operations.
- TODO for `markdown-edit`:  
  - Extend the state machine to record start/end indices for sections.  
  - Introduce newline normalization and duplicate guards per the spec.  
  - Build content writers that leverage atomic file updates and optional backups.  
  - Add validation layers for payload escapes, heading collisions, and `--max-matches`.

Known gaps to consider
----------------------
- No abstraction for Setext headings or inline-markdown normalization yet—`markdown-edit` will need richer parsing to meet the spec.  
- Code block detection doesn’t handle nested fences or indentation rules.  
- Memory usage is in-memory accumulation of full sections; for very large sections we may need streaming write support.

These notes should help agents quickly understand the existing extraction pipeline and identify components to repurpose while designing `markdown-edit`.

Planned deltas for `markdown-edit`
----------------------------------
- Parsing upgrades  
  - Add Setext (`===` / `---`) heading support and normalize inline markdown to plain text for pattern matching per spec.  
  - Track byte offsets (start/end) for each section so edits can splice content without re-reading the file.  
  - Improve code-block awareness (respect language fences with info strings, ignore indented blocks) and skip front matter if present.
- Operation engine  
  - Build an editor module layering on top of the extractor state machine to support replace/append/prepend/insert/delete.  
  - Implement newline normalization and duplicate guards before writes.  
  - Enforce heading-level validation, sibling collision checks, and single-section payload constraints.  
  - Surface dry-run unified diffs using `similar` and add atomic write + backup handling (temp file → rename).
- CLI/UX  
  - Scaffold a new `markdown-edit` binary mirroring existing Clap patterns; add flags from the spec (`--body-only`, `--max-matches`, `--allow-duplicate`, etc.).  
  - Extend error handling to map to the spec’s exit codes and render helpful “candidate headings” messages when patterns miss.
- Content sources  
  - Support `--with` file input (including `-` for stdin) and `--with-string` escape parsing; validate unsupported sequences.  
  - Optional markdown lint flag (roadmap) and future hooks for JSON/YAML export or rename operations.
- Quality + security hygiene  
  - Add comprehensive unit and integration tests for each operation, duplicate guard behavior, escape validation, and large-file scenarios.  
  - Sanitize file paths for managed environments and document size/performance envelopes.  
  - Ensure coverage for broken pipe behavior during diff/dry-run output similar to the extractor CLI.

Agent Task: Parser + State Enhancements
---------------------------------------
**Goal:** Upgrade the `markdown-extract` library so it can power `markdown-edit`’s richer section model without breaking current CLI behavior.

**Scope**
- Extend heading detection to support both ATX (`#`) and Setext (`===`/`---`) styles.
- Normalize captured heading text to plaintext: strip inline markdown (links, emphasis, code spans) and collapse whitespace so pattern matching aligns with the spec.
- Track byte offsets (start and end) for every section as it’s scanned; expose this metadata in a new struct (`SectionSpan` or similar) alongside the raw lines.
- Enhance code-block detection to handle fenced blocks with language labels (```rust`), tilde fences, and indented code blocks; section matching must ignore headings inside any code block.
- Detect YAML front matter (`---` at file start) and skip it when evaluating headings.
- Preserve backwards compatibility for existing public functions (`extract_from_path`, `extract_from_reader`) but introduce new APIs that return the richer section metadata without forcing callers to re-read the file.

**Implementation notes**
- Update `heading.rs` to parse both ATX and Setext headings, returning depth + normalized text + span start.
- Rework the state machine (`state.rs`) so it can emit structs that carry heading info, offsets, and accumulated lines; you may wrap the original `Vec<String>` in a new type.
- Use byte counters while reading to compute offsets; remember that `BufRead::lines()` strips newlines, so account for line length + 1 (or actual delimiter length) when advancing.
- For normalization, consider leveraging `pulldown_cmark` or implement a lightweight sanitizer; ensure no dependencies blow up compile times.
- Keep `extract_from_reader` compiling by delegating to the new machinery and mapping results back to plain `Vec<String>` collections.

**Testing requirements**
- Expand unit tests to cover Setext heading parsing, inline markdown normalization, and front-matter skipping.
- Add integration tests with fixtures that exercise ATX/Setext mixes, headings inside code blocks (fenced, indented, language-tagged), and byte-offset verification (assert offsets align with known positions).
- Ensure existing tests pass; add new ones under `tests/markdown/`.
- Run `cargo fmt`, `cargo clippy --all-targets --all-features`, and `cargo test --all` before handing the task back.

**Deliverables**
- Updated source files with inline comments only where logic is non-obvious.
- New/updated tests demonstrating the enhanced behavior.
- Summary of changes, test runs, and any follow-up risks or TODOs surfaced during the work.

Agent Task: Edit Engine
-----------------------
**Goal:** Build the core `markdown-edit` engine (library crate/module) that consumes `SectionSpan`s and performs the spec’d operations with full validation and write safety.

**Scope**
- Create a new crate (e.g., `crates/markdown-edit-core`) or module that depends on `markdown-extract` for span discovery.
- Implement operation handlers for `replace`, `delete`, `append-to`, `prepend-to`, `insert-after`, and `insert-before`.
- Accept payloads from either `--with` (file/stdin) or `--with-string`, interpreting escapes (`\n`, `\t`, `\\`, `\"`) and failing on unsupported sequences.
- Enforce validation rules:
  - Replacement payload must include exactly one heading unless `--keep-heading`/`--body-only` is set.
  - Heading level must match (or be child level for insert-before/after).
  - Duplicate guard: detect when the payload already exists at the insertion boundary and no-op unless `--allow-duplicate` is set.
  - `--max-matches` limits batch operations; surface exit code 2 if matches exceed the limit.
- Normalize trailing newlines: ensure section bodies end with a single newline and that sections remain separated by exactly one blank line (preserve indentation for child sections).
- Produce dry-run output as unified diffs via the `similar` crate and implement atomic writes with optional backups.
- Map all error conditions to the spec’s exit codes (0–6).

**Implementation notes**
- Build a `SectionEdit` struct that carries the original content, the edited content, byte ranges, and metadata for diffing.
- Reuse `SectionSpan` offsets to splice new content into the original buffer without multiple reads.
- Design an extensible validation error enum that can be surfaced via the CLI verbatim.
- For atomic writes: write to `file.tmp`, optionally create `file.bak`, then rename over the original.
- Keep the engine library CLI-agnostic; return structured results that higher layers can format.

**Testing requirements**
- Unit tests for each operation cover success paths, duplicate guard behavior, heading-level mismatches, multi-heading payload rejection, escape parsing, and newline normalization.
- Integration tests under `tests/` that run the engine against fixture files, verifying the diff output and final file contents.
- Simulate dry-run and real writes; ensure backups (`.bak`) are created/omitted correctly.
- Confirm that non-ASCII headings remain intact after normalization and editing.
- Run `cargo fmt`, `cargo clippy --all-targets --all-features`, and `cargo test --all` (workspace) before handing the task back.

**Deliverables**
- New core editing module/crate source files.
- Supporting unit + integration tests and fixtures.
- Summary (change log, validation coverage, diff examples, commands run).

Agent Task: CLI Integration (completed)
---------------------------------------
- Implemented the `markdown-edit` binary under `crates/markdown-edit-cli`, wiring Clap arguments to the core engine and mirroring the spec’s flag surface (`--with`, `--with-string`, `--keep-heading/--body-only`, `--all`, `--max-matches`, `--allow-duplicate`, `--dry-run`, `--backup/--no-backup`, `--quiet`, `--case-sensitive`).
- Default behaviour: prints unified diff on `--dry-run`, reports duplicate-guard no-ops, and emits structured error messages with candidate headings when patterns miss.
- Exit codes are sourced directly from `markdown-edit-core::ExitCode`, and regex compilation failures are reported as invalid arguments.
- Integration tests (`crates/markdown-edit-cli/tests/cli.rs`) exercise dry-run diffs, argument validation, and not-found messaging; relies on fixture `tests/fixtures/sample.md`.
- Updated README with a companion CLI section and flag reference so contributors can discover and operate `markdown-edit`.
- Added stdin support to `markdown-extract` (pass `-` as FILE) and covered it with an integration test.

AGENT NOTES: markdown-doc Toolkit
=================================

Phase 1 Foundations
-------------------
- **Configuration (`crates/markdown-doc-config`)**: Loads layered `.markdown-doc.toml` files (override → local → git root → defaults) into typed settings. Validates lint rules, glob patterns, ignore entries, and exposes provenance metadata (`ConfigSources`).
- **Parser (`crates/markdown-doc-parser`)**: `ParserContext` reuses `markdown-extract` to emit `DocumentSection`s with normalized headings, anchors, byte ranges, and per-line content while honoring include/exclude patterns, front matter, and code blocks.
- **Operations & CLI**:
  - `markdown-doc-ops` implements `catalog` and `lint` (broken-links) using `ScanOptions`, git staged detection, and atomic writes.
  - `markdown-doc-format` renders catalog (Markdown/JSON) and lint reports (plain/JSON/SARIF).
  - `markdown-doc-cli` exposes `catalog`/`lint` via `clap`, supporting selective scanning and structured output formats.
- **Benchmark harness**: `tools/markdown-doc-bench` times catalog + lint runs against fixtures and powers CI benchmarks.

CI & Benchmarks
---------------
- `.github/workflows/build_and_test.yml` (push/PR):
  ```bash
  cargo fmt --all -- --check
  cargo clippy --all-targets --all-features -- -D warnings
  cargo test --all --all-features
  ```
  Docker/action tests remain push-only.
- `.github/workflows/bench.yml` (nightly/manual) runs:
  ```bash
  cargo run -p markdown-doc-bench --release -- --iterations 5 --warmup 1
  ```
  Uploads `benchmark-results.txt` and appends a job summary for historical tracking.

Developer Notes
---------------
- Catalog default output is `DOC_CATALOG.md`; JSON available via `--format json`.
- Broken-links lint honors severity overrides and ignore globs; SARIF output supports CI ingestion.
- Benchmark target defaults to `tests/markdown-doc/wepppy`; override with `--path` when profiling other trees.

Phase 2 – TOC Command & Severity (Agent 7)
------------------------------------------
- Prompt refreshed with current implementation snapshot and explicit deliverables (`docs/work-packages/20251025_markdown_doc_toolkit/prompts/active/agent7_phase2_toc_severity.md`).
- Workspace tests currently fail because `load_ignore_filter` calls `GitignoreBuilder::add(&path).is_err()`; the API now returns `Result` (`crates/markdown-doc-ops/src/lib.rs:409`).
- `toc::locate_block` triggers unused-assignment warnings at EOF and needs tighter marker handling before publishing (`crates/markdown-doc-ops/src/toc.rs:75`).
- TOC command lacks end-to-end coverage for diff/update flows, newline preservation, and `.markdown-doc-ignore`/`--no-ignore` CLI combinations; extend tests in `crates/markdown-doc-ops/tests/toc.rs` and add new fixtures.
- README and `docs/markdown-doc/README.md` need TOC/ignore/severity documentation, plus tracker entry for Agent 7 once work completes.
