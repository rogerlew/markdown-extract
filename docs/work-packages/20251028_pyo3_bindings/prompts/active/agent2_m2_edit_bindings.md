# Agent Prompt – M2: markdown-edit & doc.toc PyO3 Bindings

**Agent ID**: Agent 2  
**Milestone**: M2 – Edit/Doc Bindings  
**Target Date**: 2025-11-11  
**Estimated Effort**: 4-5 days

---

## Objective

Expose the high-value operations from `markdown-edit` and `markdown-doc` (TOC command only) through PyO3 bindings so Python agents can edit Markdown safely and manage table-of-contents updates without shelling out to the CLI.

---

## Scope

### In Scope
1. **New crates**
   - `crates/markdown_edit_py`
   - `crates/markdown_doc_py`
2. **markdown-edit bindings**
   - Functions (keyword-only args):
     - `replace(file, pattern, replacement, **options)`
     - `delete(file, pattern, **options)`
     - `append_to(file, pattern, payload, **options)`
     - `prepend_to(file, pattern, payload, **options)`
     - `insert_before(file, pattern, payload, **options)`
     - `insert_after(file, pattern, payload, **options)`
   - Options surface (align with CLI / core API):
     - `case_sensitive`, `all_matches`, `body_only`, `keep_heading`, `allow_duplicate`, `max_matches`, `dry_run`, `backup`, `with_path` (optional payload file), `with_string` (escape sequences)
   - Return structured `EditResult` (pyclass) containing:
     - `applied` (bool), `exit_code` (int), `diff` (str | None when not dry-run), `messages` (list[str]), `written_path` (str | None)
   - Dry-run should return the diff text (reuse `markdown-edit-core` dry-run machinery)
   - All error cases raise `MarkdownEditError`
3. **markdown-doc bindings**
   - TOC-only API:
     - `toc(path: str, *, mode: Literal["check","update","diff"]="check", no_ignore: bool=False, quiet: bool=False) -> TocResult`
   - `TocResult` pyclass: `mode`, `status` (`"clean" | "changed" | "error"`), `diff` (optional str), `messages` (list[str])
   - Defer catalog/lint/validate to a later milestone
4. **Shared binding utilities**
   - Reuse regex builder defaults from M1 (`case_insensitive`, `unicode`, size limit)
   - Centralize error helpers so Python sees consistent exception types
5. **Python packages**
   - `python/markdown_edit_py/`
   - `python/markdown_doc_py/`
   - Type stubs (`__init__.pyi`) for both packages
   - README snippets + quick-start instructions mirroring the extract binding style
6. **Tests**
   - Rust `#[cfg(test)]` smoke tests ensuring PyO3 signatures compile (use `Python::with_gil`)
   - Python pytest suites:
     - Edit operations (success + dry-run diff + duplicate guard + error cases)
     - TOC command (check/update/diff modes, `--no-ignore` toggle)

### Out of Scope
- MCP server integration (Phase 2)
- Wheel/CI automation (Phase 3)
- Extending markdown-doc bindings beyond `toc`
- Async/await wrappers (document sync usage instead)

---

## Technical Requirements

### Crate Setup & Reuse
- Follow crate naming with underscores (`markdown_edit_py`, `markdown_doc_py`)
- Depend on existing libraries:
  ```toml
  markdown-edit-core = { path = "../markdown-edit-core" }
  markdown-doc-ops = { path = "../markdown-doc-ops", features = ["toc"] }
  pyo3 = { version = "0.20", features = ["extension-module", "abi3-py38"] }
  ```
- Reuse the M1 helpers: consider moving shared regex/error utilities into a small module (e.g., `bindings_common`) so `markdown_edit_py`/`markdown_doc_py` match the `markdown_extract_py` behaviour.
- Ensure `cdylib` crate type and add each crate to the workspace member list

### API Design Notes
- Mirror CLI defaults:
  - Case-insensitive regex by default
  - `all_matches=False` (stop after first applied edit unless requested)
  - `backup=True` maps to `.bak` creation; allow `backup=False`
  - `dry_run=True` → no writes, return diff string, `applied=False`
  - `max_matches` should raise `MarkdownEditError` if limit exceeded
  - `with_path` / `with_string` are mutually exclusive; parse escapes (`\n`, `\t`, `\\`, `\"`)
- For TOC:
  - `mode="check"` returns `status="clean"` when no changes, `"changed"` when updates needed, `"error"` on failure
  - `mode="update"` writes changes (respecting `backup` in future release; for now mirror CLI behavior)
  - `mode="diff"` returns diff output without writing
  - Honor `.markdown-doc-ignore` unless `no_ignore=True`

### Error Handling
- Define per-crate exception types:
  - `MarkdownEditError`
  - `MarkdownDocError`
- Map internal errors:
  - I/O → descriptive messages (`File not found: …`)
  - Regex compilation → `PyValueError`
  - Validation (duplicate guard, heading mismatch, payload errors) → `MarkdownEditError`
  - TOC check failures → `MarkdownDocError`
- Ensure Python callers never see raw Rust errors (wrap everything)

### Structured Results
- `EditResult` pyclass:
  ```rust
  #[pyclass(module = "markdown_edit_py")]
  pub struct EditResult {
      #[pyo3(get)] applied: bool,
      #[pyo3(get)] exit_code: i32,
      #[pyo3(get)] diff: Option<String>,
      #[pyo3(get)] messages: Vec<String>,
      #[pyo3(get)] written_path: Option<String>,
  }
  ```
- `TocResult` pyclass analogous, with `status`, `mode`, `diff`, `messages`
- Provide helper constructors to convert from Rust results (`markdown_edit_core::EditOutcome`, `markdown_doc_ops::toc::Outcome`)

### Python Packaging
- Create package skeletons (following M1 layout):
  ```
  python/markdown_edit_py/__init__.py
  python/markdown_edit_py/__init__.pyi
  python/markdown_doc_py/__init__.py
  python/markdown_doc_py/__init__.pyi
  python/tests/test_edit.py
  python/tests/test_doc_toc.py
  python/pyproject.toml  # update to include both modules
  ```
- Update `python/pyproject.toml` so maturin can build both bindings (e.g., separate `[tool.maturin]` tables or documented `--manifest-path` invocation).
- Update README sections with quick-start examples (install + basic usage)

---

## Implementation Steps (Suggested)
1. **Scaffold crates** (`cargo new --lib`) and wire into workspace
2. **Implement shared error/regex helper module** (consider re-export from `markdown_extract_py`)
3. **Implement markdown-edit bindings**:
   - Wrap `markdown_edit_core::Editor` APIs
   - Handle payload sourcing (`with_string` decode, `with_path` read)
   - Convert `EditReport` into `EditResult`
4. **Implement markdown-doc TOC binding**:
   - Call `markdown_doc_ops::toc::run(...)`
   - Convert outcome to `TocResult`
5. **Author Python packages + stubs**
6. **Write pytest coverage**
7. **Run format/check/tests**:
   - `cargo fmt`
   - `cargo check -p markdown_edit_py -p markdown_doc_py`
   - `maturin develop --manifest-path crates/markdown_edit_py/Cargo.toml`
   - `maturin develop --manifest-path crates/markdown_doc_py/Cargo.toml`
   - `pytest python/tests/test_edit.py python/tests/test_doc_toc.py`
8. **Update tracker** with progress summary + validation report

---

## Testing Expectations
- Rust: minimal smoke tests verifying functions return `PyResult`
- Python (pytest):
  - Replace with dry-run returning diff
  - Append/Prepend to existing headings
  - Duplicate guard blocks edit unless `allow_duplicate=True`
  - Max matches limit triggers error
  - Payload parsing from `with_string`
  - TOC `check` reports clean/dirty scenarios (use temp files)
  - TOC `update` writes changes and diff matches expected
  - Verify `.markdown-doc-ignore` respected unless `no_ignore=True`
- Optional: add quick perf sanity (ensure single edit under ~50 ms) if feasible

---

## Acceptance Criteria
- [ ] `markdown_edit_py` and `markdown_doc_py` compile with PyO3, exposed functions work in REPL
- [ ] All new pytest suites pass after running `maturin develop` per crate
- [ ] Exceptions map cleanly (no raw Rust errors)
- [ ] Documentation and type stubs provided for both packages
- [ ] Tracker updated with M2 progress notes and outstanding risks

---

## Resources
- `crates/markdown-edit-core` source (for API reference)
- `crates/markdown-doc-ops/src/toc.rs`
- `docs/work-packages/20251025_markdown_doc_toolkit/tracker.md` (TOC behavior details)
- PyO3 docs: <https://pyo3.rs/v0.20.0>
- maturin docs: <https://www.maturin.rs/>

---

## Handoff Notes
- M1 already ships shared regex/error guidance—reuse helpers to keep behavior consistent.
- Focus on TOC only for `markdown_doc_py`; catalog/lint/validate will arrive in a later milestone.
- Document any shortcuts or TODOs in the tracker so M3 agents can plan around them.
