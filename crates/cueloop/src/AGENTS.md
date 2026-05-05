# AGENTS.md

<!-- AGENTS ONLY: Rust source guidance for crates/cueloop/src/**. Repo-wide rules live in ../../../AGENTS.md. -->

## Purpose

This file applies to `crates/cueloop/src/**`. The Rust crate implements the `cueloop` CLI, library contracts, runner orchestration, queue management, migrations, plugins, redaction, and app machine contracts. Follow `../../../AGENTS.md` for repo-wide rules.

## Repository map

- `bin/cueloop.rs` — executable entrypoint.
- `cli/` — clap argument definitions, command routing, machine/app surfaces, and app-parity registry.
- `commands/` — command implementations for run, queue, task, scan, prompt, plugin, doctor, daemon/watch, app, init, context, and PRD flows.
- `contracts/` — queue, task, config, runner, and serialized data contracts.
- `config/`, `queue/`, `lock/`, `session/` — core runtime state handling.
- `runner/`, `runutil/` — runner execution, retries, CI gates, subprocess helpers, and phase/session behavior.
- `migration/` — config/file/runtime migration registry and implementations.
- `redaction/`, `output/`, `webhook/`, `plugins/`, `template/`, `prompts_internal/` — focused support domains.
- `testsupport/` — reusable helpers for Rust unit/integration tests.
- Integration tests are outside this scope at `crates/cueloop/tests/`.

## Operating rules

- Identify the owning module before editing. Do not add logic to a facade when a focused companion module already owns that concern.
- Preserve CLI contracts: flags/help, machine JSON, schemas, queue/task formats, config precedence, and app parity.
- Keep source files cohesive. Split only when it improves ownership or reviewability; do not create generic frameworks for one-off behavior.
- When changing user-visible CLI/config/queue/machine behavior, update tests, docs, schemas, and app parity in the same change or record the explicit blocker.
- Stop source exploration after the owning module, callers, contract tests, and validation command are clear.

## Setup and commands

Run from the repository root.

| Need | Command |
| --- | --- |
| Required routed gate | `make agent-ci` |
| Fast Rust gate | `make ci-fast` |
| Full Rust release-shaped gate | `make ci` |
| Format | `make format` |
| Format check | `make format-check` |
| Clippy lint | `make lint` |
| Type check | `make type-check` |
| Full Rust tests | `make test` |
| Quick crate tests | `cargo test -p cueloop` |
| Run local CLI | `cargo run -p cueloop -- <command>` |
| Validate queue | `cargo run -p cueloop -- queue validate` |
| Regenerate schemas | `make generate` |
| Keep temp test dirs | `CUELOOP_CI_KEEP_TMP=1 make test` |
| Update snapshots intentionally | `INSTA_UPDATE=always cargo test -p cueloop` |

`make test` uses cargo-nextest when available and falls back to cargo test, then runs doc tests. Use `CUELOOP_CI_JOBS=4` on shared workstations.

## Coding conventions

- New or changed Rust source files MUST start with `//!` module docs stating responsibility, explicit non-scope, and invariants/assumptions.
- Default to private APIs; prefer `pub(crate)` over `pub` unless the library contract needs export.
- Use `anyhow::Result` for propagation and `thiserror` enums for matchable domain errors. Use `bail!` for quick returns and `.context(...)` at IO/process/config boundaries.
- Avoid panics in runtime paths. Prefer typed validation or contextual errors.
- Keep clap help, examples, and `docs/cli.md` in sync for user-facing commands/flags.
- Configuration precedence is: CLI flags, `.cueloop/config.jsonc`, `~/.config/cueloop/config.jsonc`, schema defaults. `docs/configuration.md` is authoritative.
- Session IDs are `{task_id}-p{phase}-{timestamp}` using Unix epoch seconds. Do not add a `cueloop-` prefix or pid suffix; phase resumes reuse the same session ID.
- Queue order is execution order. Draft tasks are skipped unless `--include-draft` is set.
- Prompt defaults live in `crates/cueloop/assets/prompts/`; project overrides live in `.cueloop/prompts/*.md`.
- Breaking config/file format changes use the migration system under `migration/` and update docs.

## Validation and done criteria

Rust source work is done when changed behavior has success and failure coverage, generated outputs are current, and the routed gate passes or the blocker is reported.

- Prefer unit tests near the changed code for pure logic.
- Use `crates/cueloop/tests/` integration tests for CLI/filesystem/queue/config/runner-contract behavior.
- Use `testsupport` helpers for temp workspaces, `git_init`, `cueloop_init`, queue fixtures, and CLI invocations.
- Tests should isolate state in temp dirs. Use `--non-interactive` for `cueloop init` in tests. Use `serial_test` only for process-global state.
- Regenerate `schemas/*.schema.json` with `make generate` after schema-producing contract changes.
- Review and commit intentional snapshot changes only.
- If validation fails, inspect the first failing command, fix owned regressions, and include exact failure evidence if blocked.

## Planning and large changes

Use a short plan for work touching contracts, runner phases, queue formats, migrations, config, git publish behavior, redaction, app machine APIs, or parallel execution. Define the invariant and failure modes before editing. Do not land compatibility bridges without owner, review date, and cleanup path.

## Security and side effects

- NEVER log, print, or commit raw secrets. Debug logs and raw safeguard dumps can contain unredacted runner output.
- Use `RedactedString` and `redact_text()` for runner output before logging or copying into persisted notes.
- Do not bypass queue locks, trust/config safeguards, destructive confirmations, or CI-gate enforcement without explicit maintainer direction.
- Runtime artifacts under `.cueloop/cache/`, `.cueloop/logs/`, `.cueloop/lock/`, `.cueloop/workspaces/`, `.cueloop/undo/`, and `.cueloop/webhooks/` stay untracked.

## Progress updates and handoff

For source changes, progress updates should name the affected contract/module and current validation state. Handoffs must include changed Rust modules, generated/docs updates, tests run, failed/skipped checks, and any app-parity or migration follow-up.

## Updating this file

Keep this file source-specific and concise. Update it when Rust module ownership, CLI/config/session contracts, validation commands, or migration rules change. Move repo-wide policy to `../../../AGENTS.md` instead of duplicating it here.
