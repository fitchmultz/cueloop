# CI and Test Strategy

Purpose: canonical operator guide for local validation gates, profiling, and macOS UI evidence capture.

## Principles

- Keep default contributor checks fast and deterministic.
- Keep heavy/interactive checks opt-in and clearly labeled.
- Keep shared-workstation resource caps explicit and opt-in.

## Required for Day-to-Day Development (PR-equivalent)

Run:

```bash
make agent-ci
```

Behavior:

- **Tier A — `make ci-docs`**: all changed paths are docs/community-only (see classifier allowlist in `scripts/lib/release_policy.sh`).
- **Tier B — `make ci-fast`**: any non-docs path that is not a Rust crate path and not a macOS ship-surface path (for example repo-only metadata like `.gitignore`).
- **Tier C — `make ci`**: any change under `crates/**` when no macOS ship-surface path fired (release-shaped Rust: `ci-fast` + release build + schema generation + install checks).
- **Tier D — `make macos-ci`**: any path that affects the app bundle, committed schemas, scripts, toolchain, or build orchestration (`apps/RalphMac/**`, `apps/AGENTS.md`, `schemas/**`, `scripts/**`, `VERSION`, `Cargo.toml`, `Cargo.lock`, `Makefile`, `rust-toolchain.toml`, `.cargo/**`).
- Tier C **does not** run Xcode or Swift tests; it can miss Swift-side integration drift until a tier D run. Use `RALPH_AGENT_CI_MIN_TIER=macos-ci` or run `make macos-ci` before merge when that risk matters (see below).
- On source snapshots without `.git/`, falls back to `make release-gate` so verification stays platform-aware instead of assuming macOS-only tooling.
- The source-snapshot path still fails closed on local/runtime artifacts such as `target/`, unallowlisted `.ralph/*` content, repo-local env files (`.env`, `.env.*`, `.envrc` except `.env.example`), local notes (`.scratchpad.md`, `.FIX_TRACKING.md`), and `apps/RalphMac/build/`.

Optional environment (see `make help`):

- `RALPH_AGENT_CI_FORCE_MACOS=1` — always run `macos-ci` from `agent-ci`.
- `RALPH_AGENT_CI_MIN_TIER=ci-fast|ci|macos-ci` — raise the selected gate to at least that tier (for example `macos-ci` before merge).
- `RALPH_XCODE_KEEP_DERIVED_DATA=1` — skip deleting Xcode derived data under `target/tmp/xcode-deriveddata` for `macos-build` / default `macos-test` (faster local iteration; default remains clean derived data per run).

### `make ci` on macOS and `macos-ci` dependency graph

On Darwin, `make ci` runs `install`, which invokes `macos-install-app` when Xcode is available; that target depends on `macos-build`. So a full Rust release gate already builds the Release app bundle once before `macos-ci` runs its own `macos-build` prerequisite (GNU Make deduplicates the shared `macos-build` node in one invocation).

### Release build stamp and bundling

- The release stamp `target/tmp/stamps/ralph-release-build.stamp` is updated when `Cargo.toml`, `Cargo.lock`, `VERSION`, `rust-toolchain.toml`, `scripts/ralph-cli-bundle.sh`, or tracked Rust sources under `crates/**` are newer than the stamp (no unconditional `FORCE` rebuild).
- `install` copies from `target/release/ralph` after the stamp recipe runs, avoiding a second `ralph-cli-bundle.sh` invocation in the same gate.
- Xcode’s “Build and Bundle ralph” phase copies `target/release/ralph` into the app bundle for **Release** when that binary already exists; otherwise it falls back to `ralph-cli-bundle.sh` (for example Debug builds or cold Xcode-only builds).

### Cleaning `target/tmp`

`make clean-temp` removes `target/tmp`, which holds the release stamp and Xcode derived data defaults. The next gate run will behave like a cold build.

Docs/community-only gate is `make ci-docs`:

- `check-env-safety` (runs required-file + secret checks everywhere, and adds tracked runtime/local-only file validation when git metadata is available)
- `check-backup-artifacts`
- repo-wide markdown link scan

Fast Rust/CLI gate is `make ci-fast`:

- `check-env-safety` (runs required-file + secret checks everywhere, and adds tracked runtime/local-only file validation when git metadata is available)
- `check-backup-artifacts`
- `deps`
- `format-check`
- `type-check`
- `lint`
- `test`

## Full Rust Release Gate

Run:

```bash
make ci
```

Includes `ci-fast` plus release-shape checks:

- release build
- schema generation
- install verification

Use this before release tagging and public-readiness checks.

## macOS Ship Gate (App in Scope)

Run:

```bash
make macos-ci
```

Includes:

- `ci` (full Rust gate)
- macOS app build
- macOS app non-UI tests
- deterministic macOS contract smoke (`make macos-test-contracts`, currently the Settings open-path + workspace-routing contracts)

Interactive XCTest UI automation is intentionally excluded from `macos-ci` by default.

## Canonical Release Gate

Run:

```bash
make release-gate
```

Behavior:

- runs `macos-ci` on macOS when Xcode is available
- otherwise runs `ci`
- is the shared gate used by `make release-verify` and `scripts/pre-public-check.sh`

## Heavy / Interactive / On-Demand Checks

Run only when needed (manual or scheduled in your own automation):

```bash
make macos-test-ui
make macos-test-ui-artifacts
make macos-test-window-shortcuts
make coverage
```

Use `make macos-ui-retest` for interactive iteration. Use `make macos-test-ui-artifacts` when you need a preserved `.xcresult` bundle plus `summary.txt` under a timestamped artifact directory.

After review, clean preserved UI artifacts:

```bash
make macos-ui-artifacts-clean
```

## Resource Controls

Ralph’s make targets support resource caps:

```bash
RALPH_CI_JOBS=4 make agent-ci
RALPH_XCODE_JOBS=4 make macos-test-window-shortcuts
RALPH_CI_JOBS=4 RALPH_XCODE_JOBS=4 make pre-public-check
```

Defaults:

- `RALPH_CI_JOBS=0` lets cargo/nextest use tool-managed parallelism for fastest local iteration.
- `RALPH_XCODE_JOBS=0` keeps xcodebuild on tool-managed parallelism by default; set `RALPH_XCODE_JOBS=4` on shared workstations when you need a cap.
- `RALPH_XCODE_KEEP_DERIVED_DATA=0` deletes Xcode derived data for `macos-build` / default `macos-test` before building (reproducible; slower when iterating).
- Set either value explicitly (for example `RALPH_CI_JOBS=4`) on shared workstations.

## Suggested Cadence

- On every branch update: `make agent-ci`
- Before merge to release-ready branch: `make ci`
- For app-heavy changes: `make macos-ci`
- Overnight/manual quality sweep: UI tests + coverage

## Headless Profiling Loop

When CI speed regresses, use the supported profiling entrypoint:

```bash
make profile-ship-gate
```

This writes one timestamped bundle under `target/profiling/<timestamp>-ship-gate/` and times the same ship-gate phases every run (`make ci`, `make macos-build`, `make macos-test`, `make macos-test-contracts`), plus:

- `timings.tsv`
- `summary.md`
- `nextest.run_parallel_test.jsonl`
- `nextest.parallel_direct_push_test.jsonl`

Timestamped bundles are retained until explicit cleanup:

```bash
make profile-ship-gate-clean
```

Prefer headless paths first; interactive UI automation remains opt-in and out of the default gate.

Optimization rules for Rust integration tests:
- Hold `env_lock()` only when mutating `PATH` or other process-global env vars.
- If a fake runner is configured via an explicit `*_bin` path in `.ralph/config.jsonc`, do not also mutate `PATH`.
- Prefer `seed_ralph_dir()` over `ralph_init()` when the test only needs cached `.ralph/` scaffolding and is not asserting real init behavior.

## Expected Runtime Profile (guidance)

Actual times vary by machine and cache warmth.

- `make agent-ci` should be the fastest stable gate for the current change surface.
- `make ci` is heavier due to release build/schema/install steps.
- `make macos-ci` is heaviest among non-UI defaults.
- UI suites and coverage are intentionally separated to protect everyday DX.
