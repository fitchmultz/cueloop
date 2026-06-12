# Hot-Spot Refactor: machine task, doctor runner, agent command

Status: completed

## Outcome

The hot-spot refactor split the three largest non-test Rust source files into thin facades with focused companion modules while preserving the CLI and machine-contract behavior. The active implementation now lives in:

- `crates/cueloop/src/cli/machine/task.rs` plus `crates/cueloop/src/cli/machine/task/`
- `crates/cueloop/src/commands/doctor/runner/mod.rs` plus `crates/cueloop/src/commands/doctor/runner/`
- `crates/cueloop/src/commands/agent.rs` plus `crates/cueloop/src/commands/agent/`
- Shared lifecycle helpers in `crates/cueloop/src/queue/operations/lifecycle.rs`

This document is retained as a completed plan record, not as active roadmap guidance.

## Resolved decisions

- Scope: all three hot spots were split in the same effort so the facade pattern, module docs, and validation gate could be applied consistently.
- Doctor runner binary probing: doctor now uses `commands::runner::detection` as the single runner binary probe source of truth.
- Agent and machine task lifecycle overlap: common active/done lookup, lifecycle field updates, active-task disk mutation, and terminal archive reload behavior are centralized under `queue::operations::lifecycle`; command modules retain their lock policy, output documents, and CLI-specific rendering.
- Machine task follow-ups: task submodules return machine documents; the router owns JSON printing.
- Agent documents: agent JSON uses agent-local document enums at the boundary instead of serializing internal queue helper enums directly.
- Task ID matching: the shared lifecycle helpers follow the existing queue-operation contract of trimming surrounding whitespace and preserving case sensitivity.

## Validation evidence

- The split preserves the established facade convention: routers remain thin, companion modules have `//!` module docs, and shared queue behavior is centralized below command layers.
- Regression coverage was added for the shared lifecycle helpers and doctor runner binary behavior, including fail-closed handling for project runner overrides when repo trust or project config cannot be positively loaded.
- Final landing validation for the remediation diff is tracked in the agent handoff/final report for this change.

## References

- `docs/audits/codebase-audit-2026-03-31.md`
- `docs/audits/thermo-nuclear-code-quality-review-2026-05-21.md`
- Prior split commits: `8d7ab371` (queue.rs split), `88659663` (runner execution split)
