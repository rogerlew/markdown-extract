# Agent Prompt – Parser & Section Spans

## Objective
Enhance `markdown-doc-parser` to deliver enriched section/span data required for cataloging, linting, and future editing operations.

## Deliverables
- Parsing engine leveraging `pulldown-cmark` (or equivalent) that:
  - Supports ATX and Setext headings with normalized plain-text content (strip inline markdown, collapse whitespace).
  - Tracks byte offsets (start/end) for sections, skipping YAML front matter and ignoring headings inside fenced/indented code blocks.
  - Provides iterator/API returning structured `SectionSpan` data (path, heading depth, normalized text, anchor, byte range, raw lines reference).
- Integration with existing `markdown-extract` spans where appropriate; avoid duplicate logic.
- Unit/integration tests with fixtures covering ATX/Setext, code fences (```lang, ~~~), indented blocks, front matter, unicode headings.

## Constraints & Notes
- Coordinate with config loader outputs for include/exclude patterns.
- Ensure parser is streaming-friendly (avoid loading entire file when not required) but can provide offsets; reading into a buffer is acceptable if necessary.
- Provide normalization utility reusable by lint rules and catalog.
- Update documentation (crate README or module docs) with behavior summary.
- Log progress in work package tracker.

## Acceptance Criteria
- `cargo test -p markdown-doc-parser` passes with new tests.
- Section spans align with spec requirements and existing fixtures.
- Normalization and code-block rules validated via tests.
- API ready to be consumed by ops/lint layers.
