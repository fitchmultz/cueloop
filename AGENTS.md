# Repository Guidelines (CueLoop)

<!-- AGENTS ONLY: This file is exclusively for AI agents, not humans -->

**Keep this file updated** as you learn project patterns. Follow: concise, index-style, no duplication.

---

## Goal

CueLoop is a Rust CLI for running AI agent loops against a structured JSON task queue.

---

## Where to Find Things

| Topic | Location |
|-------|----------|
| Architecture & design | `docs/index.md` |
| Project operating constitution | `docs/guides/project-operating-constitution.md` |
| Decision log | `docs/decisions.md` |
| Contributing guide | `CONTRIBUTING.md` |
| Configuration | `docs/configuration.md` |
| CLI reference | `docs/cli.md` |
| Public launch checklist | `docs/guides/public-readiness.md` |
| Local verification guide | `docs/guides/local-smoke-test.md` |
| Public audit automation | `scripts/pre-public-check.sh` |
| GitHub Actions (single allowed workflow) | `.github/workflows/cursor-finish-line-ready.yml` — not canonical CI; see below |
| Core source | `crates/cueloop/src/` |
| Tests | `crates/cueloop/tests/` |
| macOS app | `apps/CueLoopMac/` |

---

## User Preferences

- **CI-first**: Run `make agent-ci` before claiming completion (current uncommitted local diff routes to `ci-docs`, `ci-fast`, `ci`, or `macos-ci`; clean tree is a no-op)
- **Release gate**: Run `make release-gate` before release tagging/public launch windows
- **Public-readiness gate**: Use `make pre-public-check` before making broad visibility changes
- **Resource controls**: Prefer `CUELOOP_CI_JOBS` / `CUELOOP_XCODE_JOBS` caps on shared workstations
- **Minimal APIs**: Default to private; prefer `pub(crate)` over `pub`
- **Small files**: Prefer cohesive files; file-size gate is advisory at 1,500 LOC, review advisory at 3,000 LOC, and blocking only above 5,000 LOC unless reasoned allowlisted
- **Explicit over implicit**: Prefer explicit, minimal usage patterns
- **Verify before done**: Test coverage required for all new/changed behavior
- **CI source of truth**: Local `make agent-ci` / `make release-gate` is canonical; do not treat GitHub Actions as a substitute gate

### GitHub Actions (explicit repo exception)

Global Cursor agent rules for this workspace class default to **no GitHub Actions**. **Maintainers have granted a narrow exception for the current minimal workflow only** so agents should not delete it or “fix” the repo by removing `.github/workflows/`.

- **Allowed**: `.github/workflows/cursor-finish-line-ready.yml` — triggers on completed `check_run`, uses `actions/github-script@v7` with `checks: write`, `issues: write`, `pull-requests: write`, and `contents: read`, polls until three named **Cursor Automation** checks succeed on the current PR head SHA, suppresses superseded stale-SHA results, mirrors that readiness onto a dedicated PR-head `Cursor Finish Line Ready` check run, and applies/removes the `cursor-finish-line-ready` PR label for downstream PR Finish Line sequencing. It is **not** build or test CI; the workflow file’s own header states it is demo automation sequencing only.
- **Not allowed without a new maintainer decision**: additional workflows, matrices, caching layers, release automation, or moving `make agent-ci` / `make release-gate` logic into Actions.

---

## Non-Obvious Patterns

### Error Handling Strategy
Two-tier approach: `anyhow` for propagation, `thiserror` for domain errors.

| Scenario | Pattern |
|----------|---------|
| Propagating | `anyhow::Result<T>` |
| Quick return | `bail!("msg")` |
| Add context | `.context("...")` |
| Domain errors | `thiserror` enums like `RunnerError` |

### Session ID Format
`{task_id}-p{phase}-{timestamp}` (Unix epoch seconds). No `cueloop-` prefix. Passed via `--session` flag.

### Configuration Precedence
Derived summary; `docs/configuration.md` is the configuration source of truth.

1. CLI flags
2. `.cueloop/config.jsonc`
3. `~/.config/cueloop/config.jsonc`
4. Schema defaults
