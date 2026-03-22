# Ralph Roadmap

Last updated: 2026-03-22

This is the canonical near-term roadmap for active follow-up work.

## Active roadmap

### 1. Finish the remaining parallel operator-state gaps through shared runtime surfaces

Why first:
- The serial run/resume/recovery path is aligned enough for now; the biggest remaining operator confusion is in parallel-mode recovery and bookkeeping.
- The remaining parallel gaps are retained workspaces/bookkeeping and post-run integration outcomes.
- These fixes should stay on shared operator-state builders and continuation documents instead of creating more parallel-only wording paths.

Primary outcome:
- Parallel runs should explain what was retained, what was integrated, and what the operator should do next without source diving.

Detailed execution plan:

#### 1.1 Expose retained workspace and bookkeeping state explicitly
- Show when a worker workspace or bookkeeping artifact was intentionally retained.
- Distinguish retained-for-inspection from failed-to-clean-up states.

#### 1.2 Clarify post-run integration outcomes
- Make merge/rebase/push/integration outcomes read like operator summaries, not internal plumbing.
- Keep success, retryable failure, and operator-action-required cases structurally distinct.

Exit criteria for item 1:
- Parallel mode narrates retained-workspace and integration outcomes clearly across CLI, machine, and app surfaces.
- New wording paths are shared-first, not parallel-only forks.

### 2. Remove macOS test-state pollution that is now showing up in the ship gate

Why second:
- `make agent-ci` currently surfaces CFPreferences/NSUserDefaults bloat warnings from accumulated app defaults during Xcode test runs.
- This is noisy validation debt on the active ship gate, not hypothetical cleanup.
- Fixing it is narrower and lower-churn than broader profiling or suite reshaping.

Primary outcome:
- macOS validation should run without app-defaults growth warnings or runaway test-owned persisted state.

Detailed execution plan:

#### 2.1 Prune test-owned defaults aggressively
- Ensure test and contract runs do not leave production-domain workspace snapshots and restoration state behind.
- Keep cleanup deterministic and test-owned rather than relying on incidental process teardown.

#### 2.2 Keep UI/app persistence isolation explicit
- Preserve the existing separation between production defaults and UI-test/contract defaults.
- Close any remaining paths that still accumulate state in the production domain during automated runs.

Exit criteria for item 2:
- `make agent-ci` and `macos-ci` no longer emit CFPreferences size warnings from Ralph-owned defaults.
- Repeated local app-test runs do not grow persistent defaults unboundedly.

### 3. Capture real local timing baselines, then tune the ship gate only if the data justifies it

Why third:
- The profiling workflow is already documented; the missing step is collecting fresh baseline artifacts and using them to make decisions.
- Gate tuning without current measurements would create churn without confidence.
- Timing work is safer after the active validation-noise issue above is removed.

Primary outcome:
- Ship-gate tuning discussions should point to current local artifacts, not anecdotes.

Detailed execution plan:

#### 3.1 Record fresh baseline artifacts under `target/profiling/`
- Capture current timings for `make agent-ci`, targeted operator-path nextest suites, doctests, `macos-build`, `macos-test`, and `macos-test-contracts`.
- Keep the workflow headless and local-first.

#### 3.2 Compare Rust and Xcode costs separately
- Measure Rust/CLI and Xcode surfaces independently.
- Compare capped versus uncapped `RALPH_XCODE_JOBS` before changing defaults.

#### 3.3 Change concurrency or serialization only with evidence
- Do not relax xcodebuild serialization or default job caps unless profiling plus contract coverage show the tradeoff is safe.

Exit criteria for item 3:
- Timing artifacts exist for the gates that drive iteration speed.
- Any proposed ship-gate tuning is backed by fresh local data.

## Sequencing rules

- Keep completed roadmap items out of this file; replace them with the next active work only.
- Prefer low-churn shared-runtime fixes before broader prompt, doc, or suite churn.
- Prefer operator-state clarity over maintenance-only cleanup when both are plausible next steps.
- Preserve the hardened runtime split boundaries (`runutil/execution`, `runutil/retry`, `runutil/shell`, queue prune, fsutil, eta_calculator, undo, and contracts/task) while refactoring adjacent modules.
- Do not reopen completed serial recovery alignment, macOS Settings/workspace-routing cutovers, or git/init/app split work unless a new regression appears.
