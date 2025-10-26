# Agent Prompt – Phase 3 Documentation Refresh

## Objective
Perform a human-friendly documentation pass after the link graph + `markdown-doc mv` rollout. Ensure README and supporting docs explain the new refactoring features clearly and accessibly.

## Scope
1. **Primary targets**
   - `README.md` – update the markdown-doc section with:
     - Overview of the link graph foundations.
     - `markdown-doc mv` command (usage, flags, exit codes, dry-run/backups).
     - Cross-reference to upcoming `refs` command placeholder if needed.
   - `docs/markdown-doc/README.md` – expand architecture details for link graph, rewrite planner, and refactoring commands.
2. **Content quality**
   - Emphasise readability for humans (structured headings, tables, short paragraphs).
   - Include step-by-step examples (before/after diff snippet for `mv`, JSON output sample, etc.).
   - Align terminology/flags with `--help` output; verify behavior with spot-check commands if necessary.
3. **Consistency checks**
   - Ensure existing sections (catalog, lint, validate, toc) still read smoothly after new additions; adjust navigation/table-of-contents if required.
   - Update any cross-links, quickstart bullet lists, and agent workflow notes to mention refactoring support.
4. **Report back**
   - Add a tracker note summarising documentation updates, commands run, and any follow-up suggestions.
   - Call out TODOs (e.g., when `refs` lands) so future agents know where to extend.

## Constraints & Expectations
- Documentation-only changes—no Rust implementation edits.
- Keep style consistent with rest of README (emoji headings, tables) but feel free to improve clarity.
- Use real command output where helpful (trim timestamps, maintain accuracy).
- If gaps remain (e.g., missing screenshots or future features), note them in the tracker.

## Suggested verification commands
```bash
markdown-doc mv docs/example.md docs/example-new.md --dry-run
markdown-doc mv docs/example.md docs/example-new.md --format json --dry-run
markdown-doc validate --format json --path docs/example-new.md
```
