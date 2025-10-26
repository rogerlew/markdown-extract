# Agent Prompt – Phase 3 `markdown-doc refs` & Refactor Stress Tests

## Objective
Implement the `markdown-doc refs` command for discovering references to Markdown files/anchors, and build out Phase 3 stress-test fixtures + docs covering refactoring workflows.

## Responsibilities
- **`refs` CLI command**
  - Syntax: `markdown-doc refs <PATTERN>` where `<PATTERN>` may be a file path, glob, or anchor (e.g., `docs/guide.md#setup`).
  - Options:
    - `--path`: restrict search scope (reuses scan options).
    - `--staged`: limit to staged files.
    - `--format json`: structured output for automation.
    - `--anchor-only`: when supplied, treat pattern as anchor slug.
    - `--no-ignore`: bypass `.markdown-doc-ignore` filters.
  - Output should list referencing files, line numbers, contextual snippet (first heading or line), and anchor target when relevant.
  - Exit code 0 when matches found (or 1 when none found). Additional errors (bad pattern) should map to config/IO exit codes.

- **Integration with link graph**
  - Reuse the graph built by Agent 8 to answer queries quickly without re-parsing everything.
  - Provide a text report plus JSON in the shape:
    ```json
    {
      "query": "docs/guide.md#setup",
      "matches": [
        {"file": "docs/faq.md", "line": 42, "display": "See [Guide](docs/guide.md#setup)"}
      ]
    }
    ```
  - Decide whether to include anchor-only results when pattern omits path; document behaviour.

- **Stress-test fixtures**
  - Create new fixtures under `tests/markdown-doc/refactor/complex/` representing nested directories, mixed link styles, and large documents.
  - Ensure tests cover edge cases relevant to `mv` + future operations (e.g., relative links within nested directories, anchors with spaces/punctuation).

- **Documentation**
  - Add README + `docs/markdown-doc/README.md` sections showing how to use `refs` for discovery (include JSON example, agent automation use cases).
  - Document stress-test fixture purpose for future contributors.

- **Testing**
  - Integration tests verifying `refs` outputs for file-only, anchor-only, and combined queries.
  - CLI tests for JSON output and empty results.
  - Confirm commands honour ignore settings and staged filters.

## Deliverables
- `markdown-doc refs` implemented in CLI + ops modules.
- Test suite covering queries, ignore behaviour, and JSON output.
- Expanded fixtures for complex refactoring scenarios.
- Documentation updates & tracker note summarising findings + any follow-up ideas.

## Constraints
- Keep command read-only (no file mutations).
- Ensure performance remains acceptable on WEPPpy-scale repos (reuse cached graph when possible).
- If new helper APIs are required in the link graph, coordinate with Agent 8’s module (or add there with tests).
