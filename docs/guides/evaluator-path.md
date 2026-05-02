# Evaluator Path
Status: Active
Owner: Maintainers
Source of truth: this document for its stated scope
Parent: [CueLoop Documentation](../index.md)


Use this guide when you want a fast, high-signal evaluation of CueLoop without first wiring up an external runner.

## Goal

Validate four things quickly:

1. the CLI installs and runs cleanly
2. the repository-local task queue model is easy to inspect
3. the docs point to a sane workflow
4. the local quality gate is green

## Fastest Path

From a fresh clone:

```bash
# install locally from source
make install
# macOS/Homebrew GNU Make users: gmake install

# initialize repo-local runtime files
cueloop init

# inspect the command surface
cueloop --help
cueloop queue list
cueloop queue graph
cueloop doctor

# run the required local gate
make agent-ci
```

If you prefer the full step-by-step version, use [local-smoke-test.md](local-smoke-test.md).

## What To Look For

- `cueloop --help` should make the main command groups easy to discover.
- `cueloop queue list` and `cueloop queue graph` should show the repo's structured queue model without any remote setup.
- `cueloop doctor` should explain environment readiness in plain language.
- `make agent-ci` should give you confidence that local verification is the real gate.
- On source snapshots without `.git/`, `make agent-ci` should fall back to `make release-gate` instead of forcing the macOS-only path.
- That source snapshot still needs to be export-clean: `target/`, unallowlisted `.cueloop/*` content, repo-local env files (`.env`, `.env.*`, `.envrc` except `.env.example`), local notes (`.scratchpad.md`, `.FIX_TRACKING.md`), and app build outputs should be absent.

## If You Want One Real Workflow

After the basic smoke test, try one lightweight end-to-end repo-local flow:

```bash
cueloop task "Document the evaluator quick path"
cueloop queue list
cueloop queue show RQ-0001
cueloop run one --dry-run
```

That demonstrates task creation, queue inspection, and runnable-task selection without requiring a configured model runner.

## If You Want Runner-Aware Validation

Only after the smoke test:

```bash
cueloop runner list
cueloop runner capabilities claude
cueloop run one --phases 3
```

Use that path if you specifically want to evaluate supervised execution rather than repo-local ergonomics.
