# Ralph Roadmap

Last updated: 2026-03-22

This is the canonical near-term roadmap for active follow-up work.

## Active roadmap

### 1. Finish the remaining parallel operator-state gaps through shared runtime surfaces

Why first:
- The serial run/resume/recovery path is aligned enough for now; the remaining operator gap is that RalphMac still lacks a shared parallel-status data path.
- The next work is app-side load-and-render adoption of `MachineParallelStatusDocument`.
- These fixes should keep using shared operator-state builders and continuation documents instead of creating more parallel-only wording paths.

Primary outcome:
- RalphMac should load and render the same shared parallel operator-state model already used by CLI and machine surfaces.

Detailed execution plan:

#### 1.1 Add app-side loading for shared parallel status
- Add the RalphMac client/load path for `machine run parallel-status` and decode `MachineParallelStatusDocument` into app-side models.
- Reuse existing continuation/blocking projections instead of inventing a parallel-only app contract.

#### 1.2 Render shared parallel status in app surfaces
- Feed Run Control and recovery/diagnostics views from the loaded shared parallel status instead of parallel-only app wording.
- Keep CLI, machine, and app status/recovery narration aligned from the same source.

Exit criteria for item 1:
- RalphMac loads and renders the same shared parallel-status model already used by CLI and machine output.
- New wording paths are shared-first, not parallel-only forks.

### 2. Capture real local timing baselines, then tune the ship gate only if the data justifies it

Why second:
- The profiling workflow is already documented; the missing step is collecting fresh baseline artifacts and using them to make decisions.
- Gate tuning without current measurements would create churn without confidence.
- Timing work is safer after the remaining shared parallel-status work stops moving operator-facing surfaces.

Primary outcome:
- Ship-gate tuning discussions should point to current local artifacts, not anecdotes.

Detailed execution plan:

#### 2.1 Record fresh baseline artifacts under `target/profiling/`
- Capture current timings for `make agent-ci`, targeted operator-path nextest suites, doctests, `macos-build`, `macos-test`, and `macos-test-contracts`.
- Keep the workflow headless and local-first.

#### 2.2 Compare Rust and Xcode costs separately
- Measure Rust/CLI and Xcode surfaces independently.
- Compare capped versus uncapped `RALPH_XCODE_JOBS` before changing defaults.

#### 2.3 Change concurrency or serialization only with evidence
- Do not relax xcodebuild serialization or default job caps unless profiling plus contract coverage show the tradeoff is safe.

Exit criteria for item 2:
- Timing artifacts exist for the gates that drive iteration speed.
- Any proposed ship-gate tuning is backed by fresh local data.

## Sequencing rules

- Keep completed roadmap items out of this file; replace them with the next active work only.
- Prefer low-churn shared-runtime fixes before broader prompt, doc, or suite churn.
- Finish shared Rust/machine operator-state builders before app-only follow-ups on the same path.
- For app adoption of shared machine documents, land load/decode paths before view-level rendering changes.
- Prefer operator-state clarity over maintenance-only cleanup when both are plausible next steps.
- Preserve the hardened runtime split boundaries (`runutil/execution`, `runutil/retry`, `runutil/shell`, queue prune, fsutil, eta_calculator, undo, and contracts/task) while refactoring adjacent modules.
- Do not reopen completed serial recovery alignment, queue-lock recovery alignment, macOS test-defaults isolation, macOS Settings/workspace-routing cutovers, or git/init/app split work unless a new regression appears.
