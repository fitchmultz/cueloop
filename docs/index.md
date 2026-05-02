# CueLoop Documentation
Status: Active
Owner: Maintainers
Source of truth: this document for documentation navigation
Parent: [README](../README.md)


CueLoop is a Rust CLI for running AI agent loops against a structured JSON task queue. The primary executable is `cueloop`; the legacy `cueloop` executable remains available as a compatibility alias during the migration window. The Cargo package is `cueloop-agent-loop`.

## Start Here

- [README](../README.md): product overview, first end-to-end workflow, and verification path
- [Evaluator Path](guides/evaluator-path.md): fastest reviewer-friendly path through install, queue inspection, and local verification
- [Architecture Overview](architecture.md): components, data/control flow, trust boundaries
- [Quick Start](quick-start.md): install, initialize, create first task, run it
- [CLI Reference](cli.md): command map + high-value workflows
- [Machine Contract](machine-contract.md): versioned app/automation JSON API
- [Project Operating Constitution](guides/project-operating-constitution.md): canonical project rules for source of truth, cutover, docs, UX, validation, and drift control
- [Decisions](decisions.md): project-level decision log
- [Roadmap](roadmap.md): canonical near-term execution order for active follow-up work
- [Configuration](configuration.md): hub for config schema, precedence, trust, runners, queues, webhooks, plugins, and profiles
- [PRD Specs](prd/cueloop-task-decompose.md): feature-level product requirements
- [Queue and Tasks](queue-and-tasks.md): task model and queue semantics
- [Local Smoke Test](guides/local-smoke-test.md): deterministic install and verification path
- [CueLoop Dogfood Harness](guides/dogfood-cueloop.md): repeatable end-to-end fixture project with real three-phase runner execution
- [Advanced Usage Guide](guides/advanced.md): power-user workflows, profiles, plugins, automation, and optimization
- [Advanced Troubleshooting and Reference](guides/advanced-troubleshooting.md): complex recovery patterns and quick references
- [Stack Audit (2026-04)](guides/stack-audit-2026-04.md): current toolchain/dependency inventory and Rust 1.95.0 baseline review

## Core Command Areas

- `cueloop run`: supervised execution (`one`, `loop`, `resume`, `parallel`)
- `cueloop task`: task creation, lifecycle, relations, templates, batch ops
- `cueloop queue`: queue inspection, validation, analytics, import/export
- `cueloop scan`: repository scanning and task discovery
- `cueloop prompt`: prompt rendering/export/sync/diff
- `cueloop doctor`: readiness diagnostics
- `cueloop plugin`: plugin lifecycle
- `cueloop daemon` + `cueloop watch`: background automation
- `cueloop webhook`: test/status/replay for event delivery

## Verification and Operations

Use these when you want to validate a clone, understand the operational model, or prepare for a public release:

- [README](../README.md)
- [Evaluator Path](guides/evaluator-path.md)
- [Local Smoke Test](guides/local-smoke-test.md)
- [CueLoop Dogfood Harness](guides/dogfood-cueloop.md)
- [Architecture Overview](architecture.md)
- [Public Readiness Checklist](guides/public-readiness.md)
- [Security Model](security-model.md)

## Reference Docs

- [CLI Reference](cli.md)
- [Configuration](configuration.md)
- [CI and Test Strategy](guides/ci-strategy.md)
- [Project Operating Constitution](guides/project-operating-constitution.md)
- [Decisions](decisions.md)
- [Troubleshooting](troubleshooting.md)
- [Support Policy](support-policy.md)
- [Versioning Policy](versioning-policy.md)

## Maintainer Runbooks

- [Release Runbook](guides/release-runbook.md)
- [Full Release Guide](releasing.md)

## Runtime Paths (Defaults)

- Queue: `.cueloop/queue.jsonc`
- Done archive: `.cueloop/done.jsonc`
- Project config: `.cueloop/config.jsonc`
- Prompt overrides: `.cueloop/prompts/`
- Legacy fallback: `.cueloop/` remains supported; use `cueloop migrate runtime-dir --apply` when ready.

## Validation and CI

> GNU Make >= 4 is required for project targets. On macOS, install with `brew install make` and use `gmake` unless your PATH already exposes GNU Make as `make`.

Use [`docs/guides/ci-strategy.md`](guides/ci-strategy.md) as the canonical validation guide.

Routine branch gate:

```bash
make agent-ci
```

Final ship/release gate:

```bash
make release-gate
```

Lower-level targets such as `ci-docs`, `ci-fast`, `ci`, and `macos-ci` still exist, but most contributors should treat them as internal tiers behind `make agent-ci` rather than commands to choose among day to day.
