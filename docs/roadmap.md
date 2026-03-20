# Ralph Roadmap

Last updated: 2026-03-20

This is the canonical near-term roadmap for active follow-up work.

## Active roadmap

### 1. Prioritize user-facing UI/UX so Ralph clearly solves an operator problem, not just an agent-wrapper problem

Why first:
- Ralph has enough structural foundation now that the biggest product risk is being perceived as a generic wrapper around model CLIs instead of a tool that helps users reliably manage nondeterministic agent work.
- AI agents are not deterministic tools, so the product experience must make uncertainty, progress, recovery, and operator control first-class instead of assuming repeatable single-shot execution.
- Improving the app and CLI experience around the real operator loop is higher leverage than continuing maintenance-first cleanup in isolation.

Scope:
- Prioritize workflows that help users understand what Ralph is doing, what happened, what is likely to happen next, and what they should do when an agent run stalls, drifts, fails, or produces partial value.
- Improve the highest-traffic user surfaces first: core CLI flows (`init`, `run`, `run --loop`, task selection, task mutation, resume/retry, status inspection) and the corresponding macOS app surfaces that expose queue state, run state, failures, and recovery.
- Treat agent nondeterminism as a product constraint: prefer explicit state, observable checkpoints, durable session/history context, actionable error and retry messaging, and clear recovery paths over implicit "it should just work" assumptions.
- Favor end-to-end user scenarios and smokeable operator workflows over infrastructure-only churn when choosing between similarly sized tasks.
- Validate UX changes with realistic local workflows, including interrupted runs, resume paths, partial failures, blocked tasks, CI-gate failures, and multi-step operator recovery.

### 2. Keep maintenance and structural hygiene active, but in service of the user-facing roadmap

Why second:
- Structural cleanup is still valuable, but it should support product clarity, iteration speed, and reduced UX churn rather than becoming the default priority.
- The remaining production-facing foundational split work is complete, so additional cleanup should now be more selective and evidence-driven.
- Keeping a small, explicit maintenance lane prevents test and fixture debt from compounding while preserving focus on user-visible improvements.

Scope:
- Break remaining oversized Rust test and fixture files into thin suite roots plus behavior-grouped companions/directories when that cleanup is worthwhile on its own or unblocks near-term product work.
- Current highest-value remaining structural suite candidate is `task_template_commands_test.rs`; revisit subsequent structural targets only if fresh profiling or nearby product work points there.
- Keep shared test support centralized only where duplication is real; otherwise prefer adjacent grouped helpers.
- Use target-specific nextest profiles and `target/profiling/nextest*.jsonl` artifacts as the source of truth before reopening any test-speed pass.
- Re-profile a candidate suite immediately before editing it and again immediately after any focused fixture/lock cutover instead of repeating whole-workspace sweeps.
- Narrow global-environment locks to only the tests that mutate PATH or shared process env; tests using explicit runner binary overrides should not serialize on `env_lock()`.
- Keep real `ralph init` contract tests explicit while continuing to route pure fixture setup through cached seeded scaffolding.
- Do not reopen `run_parallel_test`, `parallel_direct_push_test`, `parallel_done_json_safety_test`, or `doctor_contract_test` unless fresh measurements show they have re-emerged as worthwhile optimization targets.

### 3. Add durable local timing visibility, then use it to tighten the headless macOS ship gate

Why third:
- One-off profiling is still useful, but the repo needs a repeatable way to spot when `make agent-ci`, `make test`, or specific nextest/macOS targets get slower again.
- Adding that measurement path makes later maintenance and gate tuning easier to justify, repeat, and roll back.
- After the completed foundational split work, `macos-ci` remains the most expensive default gate for app-surface changes and should be tuned only with durable evidence.

Scope:
- Add a documented local profiling entrypoint that records `make agent-ci`, nextest, doctest, and macOS gate timings under `target/profiling/`.
- Keep the profiling path headless and opt-in; it should not slow the default CI gate.
- Prefer machine-readable summaries that make the slowest targets and trend deltas obvious.
- Measure `macos-build`, `macos-test`, and `macos-test-contracts` separately with current defaults and capped/unbounded Xcode parallelism.
- Preserve headless defaults and keep interactive UI automation outside `macos-ci`.
- Only relax Xcode serialization or default caps when the measured regression risk is understood and covered by contract tests.

## Sequencing rules

- Keep completed roadmap items out of this file; replace them with the next active work only.
- Prefer user-facing workflow clarity and operator control over maintenance-only churn when both are plausible next steps.
- Preserve the recently hardened runtime split boundaries (`runutil/execution`, `runutil/retry`, `runutil/shell`, queue prune, fsutil, eta_calculator, undo, and contracts/task) while refactoring adjacent modules.
- Prefer infrastructure and fixture stabilization before broader feature churn only when that stabilization clearly supports the active UX roadmap.
- Do not reopen the completed macOS Settings/workspace-routing or the completed git/init/app split cutovers unless a new regression appears.
