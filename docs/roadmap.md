# Ralph Roadmap

Last updated: 2026-03-23

This is the canonical near-term roadmap for active follow-up work.

## Active roadmap

### 1. Refresh ship-gate baselines and settle local concurrency defaults

Why first:
- `target/profiling/` still reflects pre-stabilization runs from 2026-03-16/17.
- `RALPH_XCODE_JOBS` should only move after one fresh local baseline pass.

Primary outcome:
- One current profiling set exists for the CLI + RalphMac ship gate, and local concurrency defaults are either updated from that data or explicitly kept.

Implementation steps:
- Re-run `make agent-ci`, doctests, targeted operator-path nextest suites, `macos-build`, `macos-test`, and `macos-test-contracts` under comparable local conditions.
- Replace stale timing outputs in `target/profiling/` with one current naming scheme and a short summary of the slowest surfaces.
- Compare capped versus uncapped `RALPH_XCODE_JOBS` runs and change defaults only if the win is material and contract coverage stays stable.

Exit criteria:
- `target/profiling/` contains one current baseline set instead of mixed cutover history.
- Any default change is justified by fresh measurements, or the current defaults are explicitly reaffirmed.

### 2. Make macOS local-validation cleanup resilient to hard interruption

Why second:
- Normal success/failure cleanup is hardened, but interrupted runs can still strand `target/tmp/locks/xcodebuild.lock` and related project-owned artifacts.
- This is the remaining local-churn source for macOS validation.

Primary outcome:
- Interrupted macOS validation recovers cleanly without manual lock/temp cleanup.

Implementation steps:
- Audit project-owned lock, temp, and derived-data artifacts created by Makefile macOS targets and contract scripts.
- Add stale-lock detection and recovery for `target/tmp/locks/xcodebuild.lock` before wait paths block indefinitely.
- Keep cleanup scoped to clearly project-owned artifacts and preserve the existing loud-failure contract for lingering app processes.
- Add deterministic contract coverage for interrupted-run cleanup and stale-lock recovery.

Exit criteria:
- Manual deletion of stranded macOS validation artifacts is no longer needed after interrupted local runs.
- Contract coverage catches regressions in stale-lock handling and cleanup recovery.

## Sequencing rules

- Keep completed work out of this file.
- Roadmap items must be chunky, dependency-aware work packages; combine adjacent evidence, cleanup, and tuning work instead of splitting follow-ups into trivial single-step tasks.
- Refresh measurements before revisiting local concurrency defaults.
- Keep shared `machine run parallel-status` decoding and version checks in RalphCore; keep Run Control presentation-only.
- Keep Run Control's initial `.task` refresh on the status-only path; use full refresh only when queue or task data must change.
- Prefer current measurement artifacts over anecdotal gate-tuning claims.
- Preserve the hardened runtime split boundaries (`runutil/execution`, `runutil/retry`, `runutil/shell`, queue prune, fsutil, eta_calculator, undo, and contracts/task) while refactoring adjacent modules.
- Do not reopen completed serial recovery alignment, queue-lock recovery alignment, macOS test-defaults isolation, macOS Settings/workspace-routing cutovers, git/init/app split work, macOS test-cleanup hardening, or the removed xcresult-attachment export path unless a new regression appears.
