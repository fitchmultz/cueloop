# Public Readiness
Status: Active
Owner: Maintainers
Source of truth: this document for its stated scope
Parent: [CueLoop Documentation](../index.md)


Use this checklist before any public release window.

## Fast Safety Gate

Run the lightweight repo safety audit at least once while iterating:

```bash
make check-repo-safety
```

That delegates to:

```bash
scripts/pre-public-check.sh --skip-ci --skip-links --skip-clean --allow-no-git
```

In source-snapshot mode, the check still rejects local/runtime artifacts such as `target/`, unallowlisted `.cueloop/*` content (for example `.cueloop/cache/`, `.cueloop/plugins/`, `.cueloop/trust.json`, `.cueloop/trust.jsonc`), repo-local env files (`.env`, `.env.*`, `.envrc` except `.env.example`), local notes like `.scratchpad.md` / `.FIX_TRACKING.md`, and `apps/CueLoopMac/build/`.

## Full Public-Readiness Audit

Before the real release mutation:

```bash
make security-audit
make pre-public-check
```

`make security-audit` requires `cargo-audit` (`cargo install cargo-audit --locked`) and checks `Cargo.lock` against RustSec advisories with warnings denied. It is listed separately because `make pre-public-check` focuses on repository safety, links, secrets, and local ship gates rather than fetching the advisory database on every run.

`make pre-public-check` runs:

- required public-file checks
- tracked runtime/build artifact checks
- tracked local-only file checks (`.env`, `.env.*`, `.envrc`, `.scratchpad.md`, `.FIX_TRACKING.md`)
- repo-wide working-tree high-confidence secret-pattern scan
- repo-wide working-tree markdown link checks
- `make release-gate`

A real `make pre-public-check` run still requires a git worktree.

## Release-Context Audit

After `versioning.sh sync` has intentionally dirtied release metadata, use release-context mode instead of forcing a clean tree:

```bash
scripts/pre-public-check.sh --skip-ci --release-context
```

`--release-context` allows only the canonical release metadata paths to be dirty.

## Suggested Sequence

1. `make agent-ci`
2. `make security-audit`
3. `make pre-public-check`
4. `make release-verify VERSION=<x.y.z>`
5. `make release VERSION=<x.y.z>`

For validation gate definitions and macOS-specific verification behavior, use [ci-strategy.md](ci-strategy.md).

## Notes

- `make release-verify` is the canonical preflight for real releases and now prepares the exact local snapshot that `make release` publishes.
- Public-readiness scans the repo working tree, excluding explicit local/runtime/build directories only; allowlisted tracked `.ralph/README.md`, `.ralph/queue.jsonc`, `.ralph/done.jsonc`, and `.ralph/config.jsonc` remain in scope for secret/link checks.
