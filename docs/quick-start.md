# Quick Start Guide
Status: Active
Owner: Maintainers
Source of truth: this document for its stated scope
Parent: [CueLoop Documentation](index.md)


Get CueLoop running in a repository in a few minutes.

## 1) Install

```bash
cargo install cueloop-agent-loop
```

This installs the primary `cueloop` executable and the legacy `cueloop` compatibility alias.

Or from source:

```bash
git clone https://github.com/fitchmultz/cueloop cueloop
cd cueloop
make install
```

> macOS note: install GNU Make with `brew install make` and use `gmake ...` unless your PATH already points `make` to Homebrew gnubin.

## 2) Initialize a Repository

```bash
cd your-project
cueloop init
```

Non-interactive setup (CI/scripts):

```bash
cueloop init --non-interactive
```

`cueloop init` creates/updates `.cueloop/trust.jsonc` by default for current repos, refreshes the generated `.cueloop/README.md` when CueLoop ships a newer template, and gitignores the trust file so project-local execution settings can work immediately without committing machine-local trust. Interactive init also asks whether queue/done should be shared through git or kept local, and lets you select extra ignored local files for parallel-worker sync. Manual additions use trusted `parallel.ignored_file_allowlist` and the small-file contract in [Ignored local file sync](configuration/queue-and-parallel.md#ignored-local-file-sync). Non-interactive init keeps queue/done tracked and only relies on the default `.env` / `.env.*` parallel sync.

## 3) Create Tasks

```bash
# Freeform task creation
cueloop task "Add regression tests for queue repair"

# Or use task builder explicitly
cueloop task build "Audit webhook retry behavior"
```

## 4) Run Tasks

```bash
# Run one runnable task
cueloop run one

# Run continuously until queue is drained
cueloop run loop
```

Useful run variants:

```bash
# Single-pass mode
cueloop run one --quick

# Explicit 3-phase supervision mode
cueloop run one --phases 3

# Dry-run selection only (no execution)
cueloop run one --dry-run
```

If you have not configured a runner yet, stop at `--dry-run` and use the local smoke test instead of a real execution pass.

Useful readiness checks before a real run:

```bash
cueloop runner list
cueloop runner capabilities claude
cueloop doctor
```

## 5) Inspect Queue State

```bash
cueloop queue list
cueloop queue next --with-title
cueloop queue validate
```

## 6) Verify Environment

```bash
cueloop doctor
cueloop runner list
cueloop runner capabilities claude
```

## 7) Optional Automation

```bash
# Background worker process
cueloop daemon start

# Watch source files for TODO/FIXME/HACK/XXX and create tasks
cueloop watch --auto-queue
```

## 8) macOS App

```bash
cueloop app open
```

## Where Files Live

Default runtime files:

- `.cueloop/queue.jsonc`
- `.cueloop/done.jsonc`
- `.cueloop/config.jsonc`

This repository intentionally keeps sanitized `.cueloop/` state for dogfooding and reproducible demos. Legacy `.cueloop/` runtime directories remain supported during the migration window.

## Next Docs

- [Evaluator Path](guides/evaluator-path.md)
- [Local Smoke Test](guides/local-smoke-test.md)
- [CLI Reference](cli.md)
- [Configuration](configuration.md)
- [Queue and Tasks](queue-and-tasks.md)
