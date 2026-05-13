# Autoresearch: make agent-ci routing overhead

## Objective
Reduce wall-clock time of CueLoop's diff-routing/classification path for small non-doc local diffs by optimizing the real classifier used by agents: `scripts/agent-ci-surface.sh --target` on a stable tiny tracked change under `scripts/` so routing resolves to `ci-fast`. The current benchmark workload intentionally touches two CI-router scripts (`scripts/agent-ci-surface.sh` and `scripts/pre-public-check.sh`) to reduce overfitting to one exact file path while staying on the same routed surface.

## Metrics
- **Primary**: `classifier_ms` (ms, lower is better) — median wall time of repeated `scripts/agent-ci-surface.sh --target` runs on the stable tiny-diff workload
- **Secondary**: `agent_ci_ms`, `surface_target_code`, `stdout_bytes`, `status_code` — one-shot end-to-end `make agent-ci` wall time as a guardrail, routing tier code (`0=noop,1=ci-docs,2=ci-fast,3=ci,4=macos-ci`), output volume, and command success status

## How to Run
`./autoresearch.sh` — prepares a stable tiny tracked diff across two CI-router scripts, runs `make agent-ci`, and emits `METRIC` lines.

## Files in Scope
- `scripts/agent-ci-surface.sh` — current diff classifier and likely optimization hotspot
- `scripts/lib/release_policy.sh` — path classification helpers used by the classifier
- `mk/ci.mk` — `agent-ci` orchestration target
- `Makefile` — shared env/reset and routing-adjacent vars if needed
- `autoresearch.sh` — benchmark harness for this session
- `autoresearch.checks.sh` — correctness backpressure for touched surfaces
- `autoresearch.ideas.md` — deferred ideas backlog

## Off Limits
- `apps/CueLoopMac/**` — macOS app is out of scope on this Linux workflow
- Product behavior changes beyond preserving current routing semantics
- New dependencies
- Generated files or schemas unless required by a kept change

## Constraints
- Preserve `make agent-ci` routing semantics and CLI-visible behavior
- Keep checks green for kept runs
- No new dependencies
- Prefer simpler code when gains are similar
- Benchmark should keep the same tiny-diff workload across runs for comparability

## What's Been Tried
- Baseline discovery showed `make agent-ci` is ~0.20s with no local diff, which is too cheap and routes to `noop`; the benchmark harness must force a stable tracked diff.
- To reduce overfitting risk, the benchmark workload now touches two real CI-router scripts instead of only `scripts/agent-ci-surface.sh`, while still routing to `ci-fast`.
- The likely hot path is repeated shelling + git diff inspection in `scripts/agent-ci-surface.sh`, with helper policy functions in `scripts/lib/release_policy.sh`.
- End-to-end `make agent-ci` proved too noisy for this goal: unchanged controls varied from ~62s to ~101s while classifier time stayed ~70-80ms. That means the prior primary metric was dominated by unrelated `lint`/`test` variance and could not honestly measure routing work.
- A failed attempt to add a script-surface fast lane in `ci-fast` broke the documented no-git fallback contract; any future optimization must preserve source-snapshot behavior.
