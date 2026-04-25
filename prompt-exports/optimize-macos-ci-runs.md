# Optimize: make macos-ci total wall-clock

**Metric:** `make macos-ci` total wall-clock latency, seconds.
**Stop criterion:** Oracle-satisfied diminishing returns; no fixed threshold.
**Scope:** Top-level `macos-ci` gate and directly invoked phase targets/scripts: `ci`, `pre-public-check`, Rust build/test/check/lint/schema/install verification, `macos-build`, `macos-test`, and deterministic macOS contract smoke tests. Preserve meaningful breakage visibility; removing no-value tests/checks is allowed only with explicit justification.
**Baseline environment:** macOS local warm-cache run, date source-of-truth `Fri Apr 24 14:52:03 MDT 2026`; user-provided raw timing artifacts `/tmp/ralph-macos-ci-timing-summary.tsv` and `/tmp/ralph-macos-ci-timing.log`.

## Phase breakdown from baseline sample

| Phase | Duration |
|---|---:|
| Total `make macos-ci` | 316.450s |
| macOS tests, Xcode non-UI | 107.649s |
| Pre-public readiness checks | 73.715s |
| Rust test phase | 73.573s |
| macOS build, Xcode Release | 53.367s |
| macOS workspace routing contract | 2.645s |
| macOS Settings smoke contract | 1.772s |
| Format check | 1.423s |
| Fetch deps | 0.592s |
| macOS ship-gate final message | 0.538s |
| Clippy | 0.436s |
| Type-check | 0.383s |
| Full CI final message | 0.107s |
| Schema generation | 0.081s |
| Fast CI final message | 0.014s |

## Runs

| # | Change | Total | Median | p95 | Notes |
|---|---|---:|---:|---:|---|
| baseline-1 | — | 316.450s | n/a | n/a | Warm-cache single sample from prior timing artifact; Xcode tests: 394 tests, 0 failures, suite runtime 52.611s; CoreSimulator warning appeared but gate passed. Need additional samples if a candidate delta is near noise. |

## Candidate/attempt notes

- Preserve valuable visibility across Rust CI, Xcode non-UI tests, and deterministic macOS contract checks.
- One attributed change per iteration; append every measurement row instead of overwriting.
