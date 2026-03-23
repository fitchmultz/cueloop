# Ralph Roadmap

Last updated: 2026-03-23

This is the canonical near-term roadmap for active follow-up work.

## Active roadmap

### 1. Refresh local ship-gate timing baselines

Why first:
- `target/profiling/` still reflects pre-stabilization measurements from 2026-03-16/17.
- Timing data is only worth refreshing after teardown leaks stop polluting local runs.

Primary outcome:
- Fresh local timing artifacts exist for the current CLI + RalphMac ship gate.

Implementation steps:
- Re-run `make agent-ci`, doctests, targeted operator-path nextest suites, `macos-build`, `macos-test`, and `macos-test-contracts`.
- Record wall-clock outputs under `target/profiling/` with current timestamps and command labels.
- Keep the workflow local and headless.

Exit criteria:
- Current artifacts replace stale baselines.
- A short measurement summary names the slowest surfaces and their current cost.

### 2. Decide ship-gate concurrency defaults from evidence

Why second:
- `RALPH_XCODE_JOBS` and related gate choices should move only after fresh numbers exist.
- This is the only worthwhile follow-up that can affect day-to-day churn after measurement refresh.

Primary outcome:
- Either the current defaults are reaffirmed or one evidence-backed cutover lands.

Implementation steps:
- Compare Rust-only versus Xcode-heavy costs from the refreshed baseline set.
- Measure capped versus uncapped `RALPH_XCODE_JOBS` runs on the same workstation conditions.
- Change defaults only if the win is material and contract coverage stays stable.

Exit criteria:
- Any default change cites the measured win and the guardrail.
- If no safe win exists, the current defaults are explicitly kept.

### 3. Prune profiling clutter after the timing refresh

Why third:
- `target/profiling/` currently mixes old cutover before/after files with the baselines that should guide future tuning.
- Cleanup is lowest value until the new baseline set is written.

Primary outcome:
- Profiling artifacts are easy to read and only the current useful set remains prominent.

Implementation steps:
- Remove or archive superseded before/after timing files that no longer inform active decisions.
- Keep one current naming scheme for baseline outputs and one short summary note.
- Avoid doc churn unless the measurement workflow itself changes.

Exit criteria:
- `target/profiling/` highlights the current baseline set without stale cutover noise.
- Documentation remains minimal and matches the retained workflow.

## Sequencing rules

- Keep completed work out of this file.
- Measure first; tune defaults second; prune profiling clutter last.
- Keep shared `machine run parallel-status` decoding and version checks in RalphCore; keep Run Control presentation-only.
- Keep Run Control's initial `.task` refresh on the status-only path; use full refresh only when queue or task data must change.
- Prefer current measurement artifacts over anecdotal gate-tuning claims.
- Preserve the hardened runtime split boundaries (`runutil/execution`, `runutil/retry`, `runutil/shell`, queue prune, fsutil, eta_calculator, undo, and contracts/task) while refactoring adjacent modules.
- Do not reopen completed serial recovery alignment, queue-lock recovery alignment, macOS test-defaults isolation, macOS Settings/workspace-routing cutovers, or git/init/app split work unless a new regression appears.
