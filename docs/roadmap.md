# Ralph Roadmap

Last updated: 2026-03-23

This is the canonical near-term roadmap for active follow-up work.

## Active roadmap

### 1. Refresh ship-gate timing baselines after RalphMac stabilization

Why first:
- Current profiling artifacts predate the latest Run Control and parallel-status cutover.
- Timing decisions are only useful after app verification stops moving.

Primary outcome:
- Ship-gate tuning decisions cite fresh local timings and a short written summary.

Detailed execution plan:

#### 1.1 Refresh current baselines under `target/profiling/`
- Re-run `make agent-ci`, doctests, targeted operator-path nextest suites, `macos-build`, `macos-test`, and `macos-test-contracts`.
- Keep the workflow local and headless.

#### 1.2 Split Rust and Xcode costs
- Measure Rust and macOS surfaces independently.
- Compare capped versus uncapped `RALPH_XCODE_JOBS` before proposing default changes.

#### 1.3 Change defaults only with evidence
- Do not relax xcodebuild serialization or job caps unless refreshed timings and contract coverage show a safe win.

Exit criteria for item 1:
- Fresh artifacts replace stale timing baselines.
- Any tuning proposal names the measured win and its guardrail.

## Sequencing rules

- Keep completed roadmap items out of this file; replace them with the next active work only.
- Keep shared `machine run parallel-status` decoding and version checks in RalphCore; keep Run Control presentation-only.
- Keep Run Control's initial `.task` refresh on the status-only path; use full refresh only when queue or task data must change.
- Refresh ship-gate timings only after RalphMac verification stops moving.
- Prefer current measurement artifacts over anecdotal gate-tuning claims.
- Preserve the hardened runtime split boundaries (`runutil/execution`, `runutil/retry`, `runutil/shell`, queue prune, fsutil, eta_calculator, undo, and contracts/task) while refactoring adjacent modules.
- Do not reopen completed serial recovery alignment, queue-lock recovery alignment, macOS test-defaults isolation, macOS Settings/workspace-routing cutovers, or git/init/app split work unless a new regression appears.
