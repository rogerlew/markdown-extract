# Agent Prompt – Schema Matcher & `markdown-doc validate`

## Objective
Build the schema-matching infrastructure and introduce the `markdown-doc validate` command for deep template conformance checks, per Phase 2 scope.

## Deliverables
- Extend `markdown-doc-config` to model schema definitions (existing `[schemas]` table) with pattern matching, required section lists, ordering constraints, and options like `allow_additional`, `min/max heading level`, etc.
- Implement a reusable schema matcher service (likely inside `markdown-doc-ops` or a dedicated module) that:
  - Resolves which schema applies to each file based on glob patterns.
  - Uses parser spans to verify required headings, ordering, and depth constraints.
  - Exposes results for both lint (required-sections rule) and the new `validate` command.
- Add the `markdown-doc validate` CLI command with options matching the spec (e.g., `--schema <name>`, `--path`, `--staged`, `--format` plain/json, exit codes distinct from lint).
- Tests:
  - Unit tests for schema resolution, matching logic, and ordering checks.
  - Integration tests covering READMEs, AGENTS.md, work-package templates, etc., using fixtures (create new sample files under `tests/markdown-doc/schemas/` as needed).
  - CLI tests ensuring structured output and exit-code mapping (e.g., exit code 1 for conformance failures, 2 for schema not found).
- Documentation updates: README quickstart, architecture primer, and config reference sections describing schemas and validate usage.

## Key Behaviors
- Default schema applies when no specific pattern matches.
- Multiple patterns may target the same file; adopt deterministic precedence (e.g., most specific wins) and document it.
- Required sections should integrate with Agent 5’s lint rule but also produce detailed validate output (e.g., “Missing section ‘Core Directives’ expected after ‘Authorship’”).
- Support `--format json` so automation/agents can process results; align schema with lint JSON style (summary + findings array).

## Constraints & Notes
- Reuse parser-normalized heading text and anchors; avoid re-parsing files.
- Ensure concurrency safety and performance remain within expectations (reuse existing parallel traversal when possible).
- Coordinate field naming with existing config loader; validation errors in config (bad schemas) must surface clearly at load time.
- Update work package tracker with progress logs and decisions.

## Acceptance Criteria
- `cargo test --all` (or targeted packages) passes with new tests.
- `markdown-doc validate` functions per spec with accurate exit codes and output formats.
- README/architecture docs describe schemas, CLI usage, and example command outputs.
- Required-sections lint rule (Agent 5) consumes the new matcher without duplicate logic.
