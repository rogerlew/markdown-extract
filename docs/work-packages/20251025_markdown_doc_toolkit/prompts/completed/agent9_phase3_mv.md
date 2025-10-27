# Agent Prompt – Phase 3 `markdown-doc mv`

## Objective
Implement the `markdown-doc mv` command that moves/renames Markdown files while updating internal references safely. This command builds on the link graph engine (Agent 8) and must ensure atomic, transactional updates with optional backups and dry-run previews.

## Responsibilities
- **Starting context**
  - Link graph + rewrite planning are available via `markdown_doc_ops::refactor` (`graph.rs`, `rewrite.rs`). `Operations::link_graph` builds graphs honoring `ScanOptions` (ignore filters, staged/path targeting).
  - `plan_file_moves` currently handles single/multi-file renames, producing `RewritePlan` + `FileEdit` with byte offsets and updated contents. It assumes callers will perform filesystem moves/atomic writes.
  - `markdown-doc-cli` already exposes `catalog`, `lint`, `validate`, `toc`. No `mv` command exists yet.
  - Tests exist for graph/rewrite behavior (`crates/markdown-doc-ops/tests/graph.rs`, `crates/markdown-doc-ops/tests/rewrite.rs`); use these as building blocks for integration coverage.

- **CLI surface**
  - Command: `markdown-doc mv <SOURCE> <DEST>` with options:
    - `--dry-run`: print planned changes + unified diffs without writing.
    - `--stdin-plan`: (future expansion) consider docstring; for now note as TODO if out of scope.
    - `--force`: allow destination overwrite when safe (otherwise fail if dest exists).
    - `--no-backup`: opt out of `.bak` creation (default is to create backups).
    - `--quiet`: suppress per-file logs (errors still printed).
    - `--json`: optional machine-readable summary (list of updated files + status).
    - `--no-ignore`: bypass `.markdown-doc-ignore` filtering.
  - Return exit codes consistent with spec (0 success, 1 validation failure, 4 I/O error, etc.).

- **Operation logic**
  - Validate source exists & is a Markdown file (or directory for future expansion – OK to start with files, but structure code to extend later).
  - Use link graph APIs to identify all referencing files + anchors that need rewrites.
  - Update relative links, image paths, and anchor fragments pointing at the source.
  - Apply file move (rename) atomically: write to temp, optionally copy `.bak`, rename in final step.
  - Handle scenarios where source/destination differ only by case, or cross directories.

- **Safety & reporting**
  - Present diff output in dry-run mode (similar to `markdown-edit` dry-run). Consider reusing diff utilities.
  - In real runs, emit human-readable summary (files updated, references corrected, backups created). Provide JSON output when requested.
  - Honour `.markdown-doc-ignore` unless `--no-ignore` flag (optional) is provided.
  - Abort gracefully when conflicts occur (e.g., link rewrite failure) and leave workspace unchanged.
  - Surface clear errors for unsupported scenarios (directory moves until implemented, cross-root moves, non-Markdown sources).

- **Testing**
  - Unit + integration tests under `tests/markdown-doc/refactor/` verifying:
    - Basic file rename with link updates.
    - Links across directories with relative path adjustments (`../` levels).
    - Dry-run diff output matches expectations.
    - Backup creation & rollback on error.
    - Ignored paths unaffected when using `.markdown-doc-ignore` (and vice versa when `--no-ignore`).

## Deliverables
- Updated CLI + ops modules implementing `mv`.
- Tests demonstrating safe rename behaviour and coverage for error cases.
- README + docs update describing `mv` usage, flags, exit codes, and sample workflows (include agent-focused guidance).
- Tracker entry summarising work, test commands, and performance observations.

## Constraints
- Reuse link graph + writer utilities; avoid duplicating graph logic.
- Keep operations transactional—if any update fails, restore prior state.
- For now, scope to single-file moves; document TODOs for directory moves if noted.
 - Ensure `cargo fmt`, `cargo clippy --all-targets --all-features`, and `cargo test --all` pass before handoff.
