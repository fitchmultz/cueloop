# Autoresearch: make agent-ci routing overhead

## Objective
Reduce wall-clock time of `make agent-ci` for small non-doc local diffs by optimizing the diff-routing/classification path before the selected gate runs. The main workload is the real repository classifier path used by agents: `make agent-ci` with a tiny tracked change under `scripts/` so routing resolves to `ci-fast`.

## Metrics
- **Primary**: `agent_ci_ms` (ms, lower is better) — end-to-end wall time for `make agent-ci` on the synthetic tiny-diff workload
- **Secondary**: `classifier_ms`, `surface_target_code`, `stdout_bytes`, `status_code` — classifier-only wall time, routing tier code (`0=noop,1=ci-docs,2=ci-fast,3=ci,4=macos-ci`), output volume, and command success status

## How to Run
`./autoresearch.sh` — prepares a stable tiny tracked diff, runs `make agent-ci`, and emits `METRIC` lines.

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
- The likely hot path is repeated shelling + git diff inspection in `scripts/agent-ci-surface.sh`, with helper policy functions in `scripts/lib/release_policy.sh`.
