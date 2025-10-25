# Work Package: markdown-doc Toolkit

**Status**: Open (2025-10-25)

## Overview
Deliver the `markdown-doc` Rust toolkit to automate documentation cataloging, linting, and refactoring workflows for the WEPPpy ecosystem. The package coordinates multi-phase development across new crates, shared utilities, and CLI experiences that build on `markdown-extract`/`markdown-edit`.

## Objectives
- Stand up the new `markdown-doc` workspace crates with shared utilities and fixtures.
- Ship the Phase 1 MVP: configuration loader, `catalog`, and `lint broken-links` command surfaces.
- Expand quality gates (Phase 2) with additional lint rules, `validate`, and `toc` automation.
- Deliver refactoring and intelligence capabilities (Phase 3–4) including safe `mv`, `refs`, and `search`.

## Scope

### Included
- Workspace scaffolding, architecture documentation, and CI wiring for the toolkit.
- Implementation of commands (`catalog`, `lint`, `validate`, `toc`, `mv`, `refs`, `search`) per phased roadmap.
- Shared markdown parsing, formatting, configuration, and persistence utilities.
- Integration tests, benchmarking harnesses, and agent-facing documentation.

### Explicitly Out of Scope
- External link checking for Phase 1 (future enhancement).
- Live preview servers, PDF/HTML export workflows.
- Git metadata extraction beyond what commands require.
- Non-markdown documentation tooling or non-Rust implementations.

## Stakeholders
- **Primary**: WEPPpy documentation maintainers & automation agents.
- **Reviewers**: Roger Lew (architecture), markdown tooling maintainers.
- **Informed**: DevOps/CI owners, broader agent community relying on docs.

## Success Criteria
- [ ] Workspace foundations merged (crates compile, fixtures present, docs updated).
- [ ] Phase 1 commands (`catalog`, `lint broken-links`) operational with config precedence, JSON/SARIF outputs, and <5s catalog generation for 388 files.
- [ ] Phase 2 quality gates implemented with schemas, severity tuning, and TOC automation.
- [ ] Phase 3 refactoring support (`mv`, `refs`) provides atomic writes, dry-run diffs, and backups by default.
- [ ] Phase 4 intelligence (`search`, watch mode) meets agreed latency and usability targets.

## Dependencies

### Prerequisites
- `markdown-extract` span enhancements and `markdown-edit` engine availability.
- Access to representative documentation corpus (mirrored in `tests/markdown-doc/`).

### Blocks
- Subsequent doc automation tooling (`wctl` integrations) depend on this package.
- Future AI-assisted documentation flows will reference search/index capabilities.

## Related Packages
- **Depends on**: N/A (scaffolds from existing markdown toolchain).
- **Related**: Future WEPPpy documentation quality initiatives (TBD).
- **Follow-up**: Potential package for external link validation backlog.

## Timeline Estimate
- **Expected duration**: 8–10 weeks across four phases.
- **Complexity**: High (cross-cutting, multi-crate coordination).
- **Risk level**: Medium-High (performance targets, link rewriting accuracy).

## References
- `markdown-doc.spec.md` – Full product specification.
- `markdown-doc.plan.nd` – Phase checklists and immediate actions.
- `docs/markdown-doc/README.md` – Architecture primer.
- `tests/markdown-doc/wepppy/` – Realistic fixture corpus.

## Deliverables
- Phase-by-phase command implementations with integration tests and benchmarks.
- Updated documentation (README, quickstarts, CI examples).
- Benchmark artifacts demonstrating performance targets.
- Work package closure notes summarizing outcomes and follow-ups.

## Follow-up Work
- Extend lint suite to cover external links and additional doc heuristics.
- Evaluate cross-repo documentation tooling reuse.
- Consider publishing `markdown-doc` as standalone crate for community use.

## Closure Notes
*Pending future completion.*
