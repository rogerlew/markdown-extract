# Agent Prompt – CI & Benchmark Harness

## Objective
Establish automation around formatting/linting/testing and build the benchmarking harness that measures `catalog`/`lint` performance on the wepppy fixtures.

## Deliverables
- Update/add GitHub Actions (or equivalent CI) workflows to run:
  - `cargo fmt --all -- --check`
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `cargo test --all`
- Introduce benchmark job (nightly or on demand) executing a binary/script that measures `catalog` and `lint --format json` over `tests/markdown-doc/wepppy`, capturing duration (<5s target for catalog).
- Provide benchmarking harness (Rust binary or cargo bench) under `benches/` or `tools/` directory with repeatable metrics output.
- Document CI/benchmark setup in `docs/markdown-doc/README.md` or a dedicated CI guide.
- Ensure workflows handle non-sandboxed benches gracefully (skip benchmarks on PRs if needed).

## Constraints & Notes
- Keep workflows compatible with existing repo CI setup; avoid breaking current pipelines.
- Bench harness should be scriptable locally (e.g., `cargo run --bin md-doc-bench` or `cargo bench` task).
- Store benchmark results as artifacts or log summaries; consistent format for future comparisons.
- Update work package tracker upon milestones.

## Acceptance Criteria
- CI pipeline runs fmt/clippy/test for markdown-doc workspace members.
- Benchmark harness executes locally and reports timings.
- Documentation outlines how to run/interpret benchmarks.
- Workflows account for skip conditions where appropriate.
