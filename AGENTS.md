# AGENTS.md

<!-- AGENTS ONLY: concise repository guidance for autonomous coding agents. -->

## Purpose

CueLoop is a local-first Rust CLI plus SwiftUI macOS app for running AI agent loops against a repo-local JSONC task queue. This file gives agents the durable repository rules needed to work safely and verify changes. Product docs start at `README.md` and `docs/index.md`; operating rules are in `docs/guides/project-operating-constitution.md`.

## Repository map

- `crates/cueloop/` — primary Rust 2024 crate; package and executable are `cueloop`.
- `crates/cueloop/src/` — CLI, queue, runner, config, migration, plugin, redaction, and support code. Extra guidance: `crates/cueloop/src/AGENTS.md`.
- `crates/cueloop/assets/prompts/` — embedded prompt templates; `.cueloop/prompts/*.md` are project overrides.
- `crates/cueloop/tests/` — Rust integration tests and shared test support.
- `apps/CueLoopMac/` — SwiftUI app that shells out to the bundled CLI. Extra guidance: `apps/AGENTS.md`.
- `schemas/*.schema.json` — committed generated schemas; regenerate with `make generate`.
- `docs/` — canonical docs. Key sources: `docs/configuration.md`, `docs/cli.md`, `docs/machine-contract.md`, `docs/guides/ci-strategy.md`, `docs/decisions.md`.
- `.cueloop/queue.jsonc`, `.cueloop/done.jsonc`, `.cueloop/config.jsonc` — repo-local active queue, archive, and config. `.cueloop/cache/`, `.cueloop/logs/`, `.cueloop/workspaces/`, `.cueloop/undo/`, `.cueloop/webhooks/`, and trust files are runtime artifacts.
- `Makefile` with fragments in `mk/` — canonical local build/test/release entrypoint.
- `scripts/` — maintenance and release helpers. User-facing scripts need useful `-h/--help` output.
- `.github/workflows/cursor-finish-line-ready.yml.disabled` — disabled demo sequencing glue, not CI.

## Operating rules

- Start from the user request and the nearest source of truth. Read only enough surrounding code/docs to identify the canonical path, affected surfaces, and validation.
- Proceed without asking for routine edits, tests, docs sync, refactors needed for correctness, or harmless cleanup. Ask the user before changing product scope, deleting active workflows, adding dependencies, changing release/security posture, or running destructive/interactive commands.
- Stop discovery once the canonical path, impacted files, and validation are clear. Do not keep searching for alternative designs unless evidence conflicts.
- Prefer the simplest complete change. Do not add new abstractions, config knobs, compatibility shims, or parallel workflows without a current requirement.
- Keep one source of truth. When replacing behavior, remove or archive obsolete references instead of leaving duplicate active paths.
- For user-visible CLI/app/config/task-format changes, update help text, docs, schemas, tests, and app parity together or record the explicit block in `crates/cueloop/src/cli/app_parity.rs`.
- Significant architecture, workflow, dependency, release, or policy decisions belong in `docs/decisions.md`.

## Setup and commands

Prerequisites: Rust is pinned by `rust-toolchain.toml`; GNU Make >= 4 is required (`gmake` on macOS if Apple `make` is first on `PATH`). Run commands from the repository root.

| Need | Command |
| --- | --- |
| See supported targets | `make help` |
| Required local gate for normal completion | `make agent-ci` |
| Cheap docs/community gate | `make ci-docs` |
| Fast Rust/CLI gate | `make ci-fast` |
| Full Rust release-shaped gate | `make ci` |
| macOS app ship gate | `make macos-ci` |
| Final release/platform gate | `make release-gate` |
| Public-readiness audit | `make pre-public-check` |
| Format | `make format` |
| Format check | `make format-check` |
| Clippy lint (`-D warnings`) | `make lint` |
| Type check | `make type-check` |
| Tests | `make test` |
| Build release CLI | `make build` |
| Regenerate schemas | `make generate` |
| Install locally | `make install` |
| Refresh compatible Rust dependency lockfile entries | `make update` |
| RustSec advisory audit | `make security-audit` |
| Quick CLI iteration | `cargo run -p cueloop -- <command>` |
| Quick crate tests | `cargo test -p cueloop` |
| Queue validation | `cargo run -p cueloop -- queue validate` |

Use `CUELOOP_CI_JOBS=4` or `CUELOOP_XCODE_JOBS=4` on shared workstations. `make agent-ci` routes by the current uncommitted diff; with no local changes it exits as a no-op. If validation cannot run, report the exact command, reason, and safest next validation step; do not claim it passed.

## Coding conventions

- Rust code uses edition 2024, `cargo fmt`, and Clippy with warnings denied.
- Keep APIs private by default; prefer `pub(crate)` over `pub` unless cross-crate export is required.
- Every new or changed Rust source file starts with `//!` module docs covering responsibility, non-scope, and invariants/assumptions.
- Use `anyhow` for propagation and `thiserror` for matchable domain errors. Add context at IO/process/config boundaries.
- Keep facade files thin; move dense logic into focused companion modules rather than growing monoliths.
- File-size policy: soft advisory above 1,500 LOC, review advisory above 3,000 LOC, and blocking failure above 5,000 LOC unless allowlisted in `scripts/file-size-allowlist.txt`.
- Integration tests live in `crates/cueloop/tests/`; use existing test-support helpers and isolated temp repos.
- Update committed generated outputs when their sources change: `schemas/*.schema.json`, `Cargo.lock`, intentional `insta` snapshots, migration docs/registry entries, CLI docs, and machine-contract docs.
- Snapshot updates must be intentional and reviewed; use `INSTA_UPDATE=always cargo test -p cueloop` only when expected output changed.
- Dependency changes go through Cargo/Make, not hand-edited lockfiles. After dependency refreshes, run the routed gate and consider `make security-audit` for release/public-facing work.

## Validation and done criteria

Done means the requested outcome works, the canonical path is updated, downstream docs/tests/generated files are synchronized, and validation evidence is recorded.

- Default completion gate: `make agent-ci`.
- Release tagging/public launch: `make release-gate` and/or `make pre-public-check` as appropriate.
- App-affecting changes: `make macos-ci` or `CUELOOP_AGENT_CI_MIN_TIER=macos-ci make agent-ci`.
- Docs-only agent-guidance edits may use `make ci-docs` plus readability checks when the routed `make agent-ci` tier would be disproportionate; state that explicitly.
- If a test fails, triage the first failing command. Fix failures you caused. If the failure is pre-existing or out of scope, include evidence and the next owner/action instead of hiding it.
- If only documentation changed and a heavier gate is skipped, say what was skipped and why.

## Planning and large changes

Use a short written plan for multi-file, cross-surface, migration, release, dependency, or app/CLI parity work. Stop planning after you know: goal, non-goals, affected source of truth, implementation path, validation, and rollback/recovery concern. Do not create a large `PLANS.md`; durable roadmap changes belong in `docs/roadmap.md`, and decisions belong in `docs/decisions.md`.

## Security and side effects

- NEVER commit, print, or copy secrets. Keep `.env`, `.env.*`, `.envrc`, raw runner logs, raw dumps, and `.cueloop/logs/` out of tracked files and reports.
- Runner output may contain sensitive data. Use redaction helpers before logging or copying output into queue notes/docs.
- Allowed local side effects from normal validation: `target/`, ignored app build/derived-data paths, temporary directories, `.cueloop/cache/`, `.cueloop/lock/`, and `.cueloop/logs/`. Normal validation must not install or replace user-global binaries; use explicit `make install` for local CLI/app installation. Do not commit generated runtime artifacts.
- Do not broaden app/CLI workspace access, bypass queue locks, disable confirmations/previews for destructive actions, or enable automatic publish behavior without explicit maintainer direction.
- Do not add or enable GitHub Actions as CI. Local Make targets are the validation source of truth; the disabled Cursor finish-line workflow is demo sequencing glue only.

## Progress updates and handoff

For multi-step or tool-heavy work, send brief progress updates after major phases: discovery complete, implementation complete, validation started/failed/passed. Keep updates factual and name current blockers.

Final handoff format:

- Files changed
- What changed and why
- Validation run and result
- Risks, skipped checks, or `[VERIFY]` items
- Next steps only when a real follow-up remains

## Cursor Cloud specific instructions

This is a Rust-only CLI project on Linux (the SwiftUI macOS app cannot build here). All `macos-*` Make targets are out of scope.

- **Rust toolchain**: Pinned to 1.95.0 via `rust-toolchain.toml`; `rustup` auto-resolves it. Components: `rustfmt`, `clippy`.
- **cargo-nextest**: Required for `make test` (it falls back to `cargo test` without it, but nextest is preferred). Installed via `cargo install cargo-nextest --locked`.
- **git default branch**: Must be `main` (`git config --global init.defaultBranch main`), otherwise several integration tests that create temp repos will fail with `fatal: 'origin/main' is not a commit`.
- **Pre-existing test failures**: Three `doctor_contract_test` tests (`doctor_auto_fix_repairs_invalid_queue`, `doctor_passes_in_clean_env`, and `doctor_warns_on_missing_upstream`) fail because they expect the `pi` runner binary on PATH. These are environment-dependent and do not indicate a code regression.
- **sccache**: Not needed on Linux. The Makefile auto-detects sccache via `which`; when absent, agent mode skips `RUSTC_WRAPPER` gracefully. No action needed.
- **`make agent-ci` vs `target/debug`**: `agent-ci` sets `CUELOOP_CARGO_MODE=agent` for the nested gate, so Cargo writes under `target/agents/$(AGENT_ID)/` (default `manual`), not `target/debug/`. Use `cargo build -p cueloop` or direct `make ci-fast` for the usual `target/` layout; see [`docs/troubleshooting.md`](docs/troubleshooting.md) and [`docs/guides/ci-strategy.md`](docs/guides/ci-strategy.md).
- **Python 3 on PATH**: `scripts/agent-ci-surface.sh` invokes `python3` (stdlib only) to merge, dedupe, and sort pathnames from `git diff` / `git ls-files` before routing; without it, classification fails before any tier runs. See [`docs/troubleshooting.md`](docs/troubleshooting.md) and [`docs/guides/ci-strategy.md`](docs/guides/ci-strategy.md#agent-ci-classifier-path-list).
- **lld**: Not used or configured by this project. Not needed.
- **No external services**: No Docker, databases, or network services needed. The entire build/test/lint workflow is self-contained.
- **Key commands**: See the `## Setup and commands` table above. For day-to-day work: `make lint`, `make format-check`, `make test`, `cargo build -p cueloop`, `make ci-fast`, and `make agent-ci` (the default completion gate).
- **Quick CLI iteration**: `cargo run -p cueloop -- <command>` from the repo root, or build once with `cargo build -p cueloop` and use `target/debug/cueloop`.

## Updating this file

Keep this file concise and repo-specific. Update it when commands, paths, invariants, or source-of-truth docs change. Put materially different subdirectory guidance in a nested `AGENTS.md`/`AGENTS.override.md`; do not bloat the root. Remove stale instructions rather than adding exceptions. Do not store model choice, reasoning effort, sandbox, or approval policy here.
