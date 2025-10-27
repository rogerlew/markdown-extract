# Agent Prompt – README Quickstart Review & Test Drive

## Objective
Validate the updated README quickstart and accompanying documentation for the Phase 2 markdown-doc release. Confirm that instructions are accurate, runnable, and clear for both humans and agents.

## Tasks
1. **Content review**
   - Read the `README.md` sections covering `markdown-doc` (overview, commands, configuration, examples, benchmarks).
   - Check for clarity, tone, accuracy, and cross-references (e.g., links to spec/docs).
   - Verify terminology aligns with the CLI (flags, rule names, exit codes).

2. **Hands-on verification**
   - Follow the quickstart steps in the README from a clean workspace: run sample commands (`catalog`, `lint`, `toc`, `validate`) against the provided fixtures.
   - Confirm JSON/SARIF snippets and table examples match actual output (adjusting for timestamps where needed).
   - Re-run representative commands with key flags (`--path`, `--staged` if applicable, `--format` variants, `toc --check/update/diff`).
   - Note any discrepancies between documented behavior and actual CLI output.

3. **Report findings**
   - Add a progress note to `docs/work-packages/20251025_markdown_doc_toolkit/tracker.md` summarizing review results, commands executed, and recommendations.
   - Identify any required edits (typos, clarifications, missing steps). If changes are minor, enumerate them; if substantial, open follow-up tasks.

## Constraints & Expectations
- Run within the existing workspace; do not modify code unless documenting a bug.
- Capture command output snippets or summarize key differences (no need to paste full logs unless illustrating an issue).
- Keep feedback actionable and organized (e.g., bullets grouped by topic).
- If everything looks good, explicitly state PASS and any residual risks or suggested future improvements.

## Deliverables
- Tracker entry with review summary, test commands, and status (PASS/FAIL per area).
- Optional inline suggestions in README if corrections are obvious; otherwise, describe required edits for the PM to apply.
