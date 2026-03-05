# Ralph Documentation

Ralph is a Rust CLI for running AI agent loops against a structured JSON task queue.

## Start Here

- [Architecture Overview](architecture.md): components, data flow, and design trade-offs
- [Quick Start](quick-start.md): install, initialize, create first task, run it
- [CLI Reference](cli.md): command map + high-value command patterns
- [Configuration](configuration.md): full config schema and precedence
- [Queue and Tasks](queue-and-tasks.md): task model and queue semantics

## Command Areas

- `ralph run`: supervised execution (`one`, `loop`, `resume`, `parallel`)
- `ralph task`: task creation, lifecycle, relations, templates, batch ops
- `ralph queue`: queue inspection, validation, analytics, import/export
- `ralph scan`: repository scanning and task discovery
- `ralph prompt`: prompt rendering/export/sync/diff for auditing and customization
- `ralph plugin`: plugin lifecycle (list/validate/install/uninstall/init)
- `ralph daemon` + `ralph watch`: background automation and TODO-comment ingestion
- `ralph doctor`: readiness diagnostics
- `ralph webhook`: test/status/replay for event delivery

## Feature Guides

- [Features Index](features/README.md)
- [Runners](features/runners.md)
- [Phases](features/phases.md)
- [Parallel](features/parallel.md)
- [Session Management](features/session-management.md)
- [Webhooks](features/webhooks.md)
- [Plugins](features/plugins.md)

## Additional Guides

- [Getting Started (extended)](guides/getting-started.md)
- [Advanced Usage](guides/advanced.md)
- [CI and Test Strategy](guides/ci-strategy.md)
- [Public Readiness Checklist](guides/public-readiness.md)
- [Release Readiness Report](guides/release-readiness-report.md)
- [Public Release Hardening Investigation](guides/public-release-hardening-investigation.md)
- [History Cleanup Execution Plan](guides/history-cleanup-execution-plan.md)
- [Plugin Development](plugin-development.md)
- [Workflow](workflow.md)

## Runtime Paths (Defaults)

- Queue: `.ralph/queue.jsonc` (`.json` fallback)
- Done archive: `.ralph/done.jsonc` (`.json` fallback)
- Project config: `.ralph/config.jsonc` (`.json` fallback)
- Prompt overrides: `.ralph/prompts/`

## Validation and CI

> GNU Make >= 4 is required for project targets. On macOS, install with `brew install make` and use `gmake` unless your PATH already exposes GNU Make as `make`.

Before merging documentation or code changes:

```bash
make ci-fast
make agent-ci
# Optional shared-workstation caps: RALPH_CI_JOBS=4 make agent-ci
```

Full Rust release gate:

```bash
make ci
```

Ship gate (when macOS app changes are in scope):

```bash
make macos-ci
# Optional caps: RALPH_CI_JOBS=4 RALPH_XCODE_JOBS=4 make macos-ci
```

## Notes on Historical Docs

Some files in `docs/` are design artifacts or migration notes kept for context (for example tactical plans or runner research snapshots). Treat current CLI help and the reference docs above as the source of truth for active behavior.
