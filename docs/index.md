# CueLoop Documentation
Status: Active
Owner: Maintainers
Source of truth: this document for documentation navigation and page ownership
Parent: [README](../README.md)

CueLoop is a Rust CLI plus SwiftUI macOS app for queue-driven, auditable AI coding agent work. Use this page as the map; each linked page owns its own detailed contract.

## Pick Your Path

| If you are... | Start with | Then read |
| --- | --- | --- |
| Evaluating CueLoop quickly | [README](../README.md) | [Evaluator Path](guides/evaluator-path.md), [Local Smoke Test](guides/local-smoke-test.md) |
| Installing CueLoop in a repo | [Quick Start](quick-start.md) | [Configuration](configuration.md), [CLI Reference](cli.md) |
| Learning the task ledger | [Agent Usage Guide](guides/agent-usage.md) | [Queue](features/queue.md), [Task System](features/tasks.md), [Task Schema and Field Reference](features/task-schema.md) |
| Learning runner-backed workflow | [Architecture Overview](architecture.md) | [Phases](features/phases.md), [Runners](features/runners.md), [Supervision](features/supervision.md) |
| Running CueLoop day to day | [CLI Reference](cli.md) | [Troubleshooting](troubleshooting.md), [Feature Guides](features/README.md) |
| Operating or releasing this project | [CI and Test Strategy](guides/ci-strategy.md) | [Project Operating Constitution](guides/project-operating-constitution.md), [Release Runbook](guides/release-runbook.md) |
| Building app or automation integrations | [Machine Contract](machine-contract.md) | [App Feature Guide](features/app.md), [Pi Integration](integrations/pi.md) |
| Using CueLoop as an already-running agent | [Agent Usage Guide](guides/agent-usage.md) | [Machine Contract](machine-contract.md), [Queue](features/queue.md), [Task Schema and Field Reference](features/task-schema.md) |

## Canonical Documentation Ownership

These pages are the active sources of truth. Legacy URLs remain as navigation bridges when useful, but they should not accumulate new reference material.

| Topic | Canonical page |
| --- | --- |
| Product overview and fastest value proof | [README](../README.md) |
| Install, init, and first local checks | [Quick Start](quick-start.md) |
| Commands and flags | [CLI Reference](cli.md) |
| Configuration, precedence, trust, profiles, plugins, and integrations | [Configuration](configuration.md) |
| Runtime architecture and trust boundaries | [Architecture Overview](architecture.md) |
| Queue file operations, ordering, locking, repair, archive, import/export | [Queue](features/queue.md) |
| Task docs index | [Task System](features/tasks.md) |
| Task JSON fields and schema examples | [Task Schema and Field Reference](features/task-schema.md) |
| Status transitions and priority semantics | [Task Lifecycle and Priority](features/task-lifecycle.md) |
| Dependencies, blocking, relations, duplicates, and hierarchy | [Task Relationships](features/task-relationships.md) |
| Creating, editing, templating, cloning, importing, and batching tasks | [Task Operations](features/task-operations.md) |
| Multi-phase execution | [Phases](features/phases.md) |
| Runner orchestration | [Runners](features/runners.md) |
| CI gates, git oversight, and human-in-the-loop supervision | [Supervision](features/supervision.md) |
| Prompt overrides | [Prompts](features/prompts.md) |
| Session recovery | [Session Management](features/session-management.md) |
| Parallel execution | [Parallel](features/parallel.md) |
| Background automation | [Daemon and Watch](features/daemon-and-watch.md) |
| Webhooks and notifications | [Webhooks](features/webhooks.md), [Notifications](features/notifications.md) |
| Security model | [Security Model](security-model.md), [Security Features](features/security.md) |

## Human Onboarding

Use these when you want a guided human-readable path rather than a machine contract or maintainer runbook.

- [Quick Start](quick-start.md): shortest install/init/inspect path.
- [Getting Started](guides/getting-started.md): guided orientation with links to deeper docs.
- [Evaluator Path](guides/evaluator-path.md): reviewer-friendly route through proof and validation.
- [Local Smoke Test](guides/local-smoke-test.md): deterministic no-runner validation.
- [CueLoop Dogfood Harness](guides/dogfood-cueloop.md): repeatable end-to-end fixture project.
- [Advanced Usage Guide](guides/advanced.md): power-user workflows, profiles, plugins, automation, and optimization.

## Feature Guides

The [Feature Guides](features/README.md) page is the feature-area index. Common entry points:

- [Queue](features/queue.md)
- [Task System](features/tasks.md)
- [Phases](features/phases.md)
- [Runners](features/runners.md)
- [Supervision](features/supervision.md)
- [App (macOS)](features/app.md)
- [Scan](features/scan.md)
- [Plugins](features/plugins.md)
- [Daemon and Watch](features/daemon-and-watch.md)

## Reference Docs

- [CLI Reference](cli.md)
- [Configuration](configuration.md)
- [Machine Contract](machine-contract.md)
- [Environment Variables](environment.md)
- [Error Handling Guidelines](error-handling.md)
- [Support Policy](support-policy.md)
- [Versioning Policy](versioning-policy.md)
- [Decisions](decisions.md)
- [Roadmap Archive](roadmap.md)

## Maintainer Runbooks

- [Project Operating Constitution](guides/project-operating-constitution.md)
- [CI and Test Strategy](guides/ci-strategy.md)
- [Public Readiness Checklist](guides/public-readiness.md)
- [Release Runbook](guides/release-runbook.md)
- [Full Release Guide](releasing.md)

## Agent-Facing Docs

These are for coding agents or app/automation clients, not first-time human onboarding.

- [Agent Usage Guide](guides/agent-usage.md)
- [Machine Contract](machine-contract.md)
- [Project Operating Constitution](guides/project-operating-constitution.md)
- [Configuration Trust and Precedence](configuration/trust-and-precedence.md)

## Archive, Audit, and Baseline Material

Archive and audit docs are useful for history, review evidence, and follow-up planning. Most are point-in-time artifacts; active behavior is defined by the canonical docs above and generated schemas. The current stack audit remains an active baseline document until a newer stack audit supersedes it.

- [Archive and audit policy](archive/README.md)
- [Stack Audit (2026-04)](guides/stack-audit-2026-04.md): current toolchain/dependency baseline
- [Thermo-Nuclear Code Quality Review (2026-05-21)](audits/thermo-nuclear-code-quality-review-2026-05-21.md): point-in-time maintainability review
- [Comprehensive Codebase Audit (2026-03-31)](audits/codebase-audit-2026-03-31.md): point-in-time codebase audit
- [CueLoopMac Settings Window Investigation (2026-03-13)](audits/2026-03-13-cueloopmac-settings-window-investigation.md): resolved investigation notes
- [Stack Audit (2026-03)](guides/stack-audit-2026-03.md): older baseline kept for comparison

## Runtime Paths (Defaults)

- Queue: `.cueloop/queue.jsonc`
- Done archive: `.cueloop/done.jsonc`
- Project config: `.cueloop/config.jsonc`
- Prompt overrides: `.cueloop/prompts/`
- Runtime migration: use `cueloop migrate runtime-dir --check` before applying supported old-state migrations.

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

Routing uses only the **current uncommitted** working tree (including untracked paths); commits already on the branch do not change the tier. To debug routing, run `scripts/agent-ci-surface.sh --target` and `--reason` from the repo root. Changes to `scripts/agent-ci-surface.sh` or path allowlists in `scripts/lib/release_policy.sh` should stay aligned with contract coverage in [`crates/cueloop/tests/agent_ci_surface_contract_test.rs`](../crates/cueloop/tests/agent_ci_surface_contract_test.rs) (see [`docs/guides/ci-strategy.md`](guides/ci-strategy.md)).
