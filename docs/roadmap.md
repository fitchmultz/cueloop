# Ralph Roadmap

Last updated: 2026-03-20

This is the canonical near-term roadmap for active follow-up work.

## Active roadmap

### 1. Make Ralph feel like an operator console for nondeterministic agent runs, not a generic agent wrapper

Why first:
- Ralph's biggest product risk is not missing another model integration; it is leaving operators unsure what happened, what Ralph is doing now, and what the safest next action is after a nondeterministic run goes sideways.
- The highest-traffic product surfaces are already the run, recover, inspect, and repair loops: `ralph run one`, `ralph run loop`, `ralph run resume`, queue inspection commands, `ralph undo`, and the app's Queue, Run Control, Quick Actions, and task-detail flows.
- First run, resume, retry, and fresh re-invocation can diverge; Ralph has to narrate state, confidence, and operator choices explicitly instead of acting like a deterministic build tool.

Scope:
- First, make interrupted-run state obvious in both CLI and app surfaces: when `.ralph/cache/session.json` says there was an interrupted run, `ralph run one`, `ralph run loop`, `ralph run resume`, `--resume`, `--force`, and the Run Control surface should clearly say whether Ralph is resuming, falling back to a fresh invocation, or stopping for operator confirmation because session safety is unclear.
- Next, make blocked, waiting, and partial-progress states legible: `ralph run loop --wait-when-blocked`, `ralph queue stats`, `ralph queue aging`, `ralph queue burndown`, and the app's Queue, List, Kanban, and Dependency Graph views should help the operator see whether the system is waiting on dependencies, schedule, lock cleanup, CI fallout, or a genuinely stalled run.
- Then, make supervision failures actionable instead of noisy: when CI retried twice, when `git_revert_mode` starts to matter, or when runner/session capability problems are the real issue, the CLI, `ralph doctor`, `ralph runner list`, `ralph runner capabilities`, completion checklists, and the app's Run Control / Quick Actions surfaces should make the decision path explicit.
- After that, make recovery tools part of the normal workflow instead of a last-resort escape hatch: operators should be able to keep partial value and recover cleanly with `ralph task mutate`, `ralph task decompose`, `ralph queue validate`, `ralph queue repair`, `ralph undo`, and the app's task-detail editing / command-palette flows without dropping into manual JSON surgery.
- Only after that, tighten the experimental parallel path: `ralph run parallel`, lock/stale-lock handling, and post-run bookkeeping visibility should improve only after the serial run/resume/recovery path is boring and trustworthy.

### 2. Keep maintenance and structural hygiene active, but only when it helps the operator-facing roadmap move faster

Why second:
- Cleanup matters when it makes the run/recover/supervise surfaces safer to change, easier to validate, and easier to explain.
- Ralph already has the baseline architecture it needs; additional hygiene work should now be selective, evidence-driven, and tied to product-facing iteration speed.
- A small maintenance lane keeps test and fixture debt from slowing the UX work above without letting maintenance become the default priority again.

Scope:
- Split or reorganize oversized suites and fixtures only when that improves failure locality or iteration speed for operator-critical paths such as run/resume, supervision, undo/recovery, queue repair, task editing, or app run-control flows; `task_template_commands_test.rs` remains the clearest currently-known structural candidate, not the default next task.
- Re-profile before and after focused test work; use target-specific nextest runs and `target/profiling/nextest*.jsonl` artifacts instead of reopening broad "test speed" churn without evidence.
- Keep environment locking narrow: tests that mutate shared PATH or shared process environment should serialize on `env_lock()`, but isolated tests using explicit runner overrides should not.
- Keep real contract coverage for operator-critical semantics explicit: use real `ralph init`, queue/undo/recovery coverage, and real supervision behavior where correctness matters; keep cached scaffolding limited to pure setup speed.
- Keep CLI, app, and machine surfaces behaviorally aligned when operator UX changes land, especially around `ralph machine queue read`, `ralph machine task mutate`, and `ralph machine run one`.

### 3. Add durable local timing visibility, then use it to tune the headless ship gate with evidence

Why third:
- Operator-facing improvements are easier to sustain when local validation speed regressions are visible instead of discovered by feel.
- Durable measurement keeps maintenance work and macOS gate tuning grounded in evidence instead of vibes.
- `macos-ci` is still expensive enough that any relaxation of serialization or job caps should be justified with data and protected by contract coverage.

Scope:
- Add or document an opt-in local profiling entrypoint that writes timing artifacts under `target/profiling/`.
- Capture timings for `make agent-ci`, targeted nextest suites that cover run/resume/supervision/undo/operator flows, doctests, `macos-build`, `macos-test`, and `macos-test-contracts`.
- Keep outputs machine-readable so the slowest targets and trend deltas are obvious.
- Measure Xcode targets separately under current defaults and under capped vs unbounded `RALPH_XCODE_JOBS`.
- Keep the profiling path headless and opt-in; do not widen `macos-ci` to include interactive UI automation.
- Only relax Xcode serialization or default caps after measurement and contract coverage support the change.

## Sequencing rules

- Keep completed roadmap items out of this file; replace them with the next active work only.
- Prefer run/resume/recovery clarity and operator control over maintenance-only churn when both are plausible next steps.
- Preserve the recently hardened runtime split boundaries (`runutil/execution`, `runutil/retry`, `runutil/shell`, queue prune, fsutil, eta_calculator, undo, and contracts/task) while refactoring adjacent modules.
- Prefer infrastructure and fixture stabilization before broader feature churn only when that stabilization clearly supports the active operator UX roadmap.
- Do not reopen the completed macOS Settings/workspace-routing or the completed git/init/app split cutovers unless a new regression appears.
