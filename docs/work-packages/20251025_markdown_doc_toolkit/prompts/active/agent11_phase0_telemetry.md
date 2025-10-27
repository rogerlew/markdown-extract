# Agent Prompt – Phase 0 Telemetry Logging

## Objective
Instrument the docs-quality workflow so lint runtime and error counts are captured as JSON telemetry for at least two weeks ahead of Phase 4. This gate must be complete before search/indexing work begins.

## Deliverables
1. Update docs-quality workflow to:
   - Time the `wctl doc-lint --format json` invocation.
   - Parse the JSON summary (files scanned, errors, warnings).
   - Append a JSONL record with timestamp, commit SHA, duration, error/warning counts to a telemetry file (e.g., `telemetry/docs-quality.jsonl`).
   - Upload the telemetry file as an artifact (or send to existing storage) for later aggregation.
2. Lightweight Node/Bash script committed under `scripts/` to perform the logging (document usage in the workflow).
3. README/work package note describing telemetry format and how to consume it.
4. Tracker update noting telemetry go-live date so Phase 4 can measure “≥2 weeks of data”.

## Constraints & Expectations
- Keep runtime overhead minimal (<200 ms).
- Continue to respect `MARKDOWN_DOC_WORKSPACE` and existing environment assumptions.
- Ensure workflow succeeds even if telemetry write fails (avoid blocking docs-quality runs on logging).
- No additional dependencies beyond Node core/JQ (stay within existing toolchain).

## Acceptance Criteria
- Workflow logs telemetry file with each docs-quality run (visible in artifacts).
- JSON line matches schema: `{"timestamp": "...", "commit": "...", "lint": {"duration_ms": ..., "errors": ..., "warnings": ...}}`.
- Tracker entry updated with start date of telemetry collection.
- README/tooling docs mention telemetry location for analysis.
