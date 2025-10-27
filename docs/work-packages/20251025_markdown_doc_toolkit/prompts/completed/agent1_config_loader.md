# Agent Prompt – Config Loader Foundations

## Objective
Build the `markdown-doc-config` crate so it loads configuration with the precedence stack (flags → local `.markdown-doc.toml` → git-root `.markdown-doc.toml` → embedded defaults) and produces typed settings with validation errors.

## Deliverables
- Rust implementation that:
  - Accepts an optional config path override and working directory.
  - Finds and merges config files per precedence rules.
  - Applies built-in defaults for unset values.
  - Validates key fields (e.g., known command names, valid glob patterns) and returns descriptive errors.
- Unit tests covering precedence, merges, missing files, validation failures.
- Docs: module-level comment and README note if behavior affects users.

## Constraints & Notes
- Use existing plan in `markdown-doc.plan.nd` (Phase 1).
- Reference spec sections on configuration precedence (`markdown-doc.spec.md`).
- Avoid pulling in heavy dependencies; `toml`, `serde`, `globset` are acceptable.
- Ensure outputs integrate cleanly with downstream crates (return a struct that the parser/CLI can consume).
- Update work package tracker with progress notes and mark tasks when complete.

## Acceptance Criteria
- `cargo test -p markdown-doc-config` passes.
- Precedence logic matches spec examples.
- Validation errors surfaced with actionable messages.
- Documentation updated to reflect config loading behavior.
