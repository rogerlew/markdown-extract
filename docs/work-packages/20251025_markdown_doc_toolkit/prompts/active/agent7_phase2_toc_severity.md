# Agent Prompt – TOC Command & Severity Tuning

## Objective
Implement the `markdown-doc toc` command and Phase 2 configuration enhancements for severity tuning, per-path ignores, and `.markdown-doc-ignore` support.

## Deliverables
- `markdown-doc toc` CLI command capable of:
  - Scanning target files (supports `--path`, `--staged`, direct file arguments).
  - Reading/writing TOC blocks delimited by configurable markers (default `<!-- toc -->` … `<!-- tocstop -->`).
  - Providing `--check`, `--update`, and `--diff` (or equivalent) modes per the spec so users can validate without modifying files.
  - Sharing TOC parsing logic with the lint `toc-sync` rule (Agent 5) to avoid duplication.
- Configuration upgrades:
  - Extend severity tuning to support wildcard entries (e.g., `"*"`) and per-path overrides that compose with existing rule maps.
  - Add `.markdown-doc-ignore` file support (gitignore semantics) merged into scan filters, with CLI override to disable.
  - Document any new config keys/defaults introduced.
- Tests:
  - Unit coverage for TOC parsing/rendering, ignore file parsing, severity precedence, and CLI option handling.
  - Integration/CLI tests demonstrating TOC update/check flows, ignore precedence, and atomic writes.
  - Ensure broken pipe behavior remains resilient when piping TOC output.
- Documentation updates: README quickstart + `docs/markdown-doc/README.md` sections describing TOC usage, ignore mechanics, and severity configuration, as well as tracker notes.

## Key Behaviors
- Leverage parser spans for constructing TOCs; respect include/exclude patterns and max heading depth configuration.
- Maintain atomic write guarantees with optional backups; `--check`/`--diff` must not mutate files.
- Ignore resolution order: CLI overrides > `.markdown-doc-ignore` > config exclude patterns.
- Severity tuning should merge defaults + per-path entries deterministically and expose the final severity map for rendering.

## Constraints & Notes
- Reuse existing utilities (atomic writer, glob helpers, rayon executor) rather than re-inventing infrastructure.
- Keep performance aligned with catalog/lint expectations (<5 s on fixture corpus); profile if TOC updates introduce noticeable overhead.
- Coordinate with Agents 5/6 so shared helpers (e.g., heading caches, ignore resolution) remain consistent.
- Log progress and decisions in the work package tracker.

## Acceptance Criteria
- `cargo test --all` (or targeted subsets) passes with new coverage.
- `markdown-doc toc` supports `--check`, `--update`, `--diff`, selective scanning, ignore overrides, and surfaces actionable exit codes.
- Severity tuning + `.markdown-doc-ignore` behavior verified by automated tests and documented examples.
- README and architecture docs updated to reflect the new command and configuration options.
