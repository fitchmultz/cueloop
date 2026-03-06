# Release Readiness Report

Purpose: capture a point-in-time publication snapshot for Ralph.

## Snapshot Metadata

- Date: March 5, 2026
- Hardening commit: `05a141f1`
- Environment: local macOS workstation (GNU Make + pinned Rust toolchain)
- Scope: public-release hardening pass (CI, safety, and docs)

## Gate Results

Status values: `PASS`, `FAIL`, or `PENDING`.

- `make agent-ci`: `PASS`
- `make ci`: `PASS`
- `make pre-public-check`: `PASS`
- `make macos-ci`: `PASS`
- Local smoke sequence (CLI help + focused contract tests): `PASS`

Evidence commands run:

```bash
make check-repo-safety
make agent-ci
make ci
scripts/pre-public-check.sh --skip-clean

# clean-snapshot verification of full strict gate
make pre-public-check

# local smoke sequence
cargo run -p ralph -- --help
cargo run -p ralph -- run one --help
cargo run -p ralph -- scan --help
cargo test -p ralph --bin ralph normalize_repo_prompt_args -- --nocapture
cargo test -p ralph cli_parses_run_ -- --nocapture
cargo test -p ralph cli_rejects_run_loop_with_id_flag -- --nocapture
cargo test -p ralph --test run_cli_overrides_contract_test -- --nocapture
```

Notes:

- `make pre-public-check` now passes end-to-end on a clean tree for this hardening commit.
- `make macos-ci` also passed (183 app/core tests; UI automation intentionally excluded by default in this target).

## What Changed in This Pass

- Hardened publication guardrails:
  - Fixed secret-scan allowlist false-positive behavior in `scripts/pre-public-check.sh`
  - Expanded tracked runtime artifact checks (`.ralph/workspaces`, `.ralph/undo`, `.ralph/webhooks`)
  - Added `.ralph` tracked-file allowlist policy checks
  - Tightened tracked env-file detection to catch `.env*` patterns (excluding `.env.example`)
- Promoted safety checks into required local gates:
  - Updated `check-env-safety` to delegate to `scripts/pre-public-check.sh --skip-ci --skip-links --skip-clean`
  - Added `check-repo-safety` alias target for explicit safety runs
- Strengthened ignore policy for runtime-only local paths:
  - Added `.ralph/undo/` and `.ralph/webhooks/` to `.gitignore`
- Improved public-facing docs:
  - README cold-start clarity (scope, non-goals, limitations, security/data handling)
  - Architecture trust boundaries + failure/recovery details
  - Public checklist + verification guide alignment
  - Added support/version/security/troubleshooting/runbook/smoke-test docs
- Determinism upgrade:
  - Added `rust-toolchain.toml`
  - Added crate-level `rust-version = "1.93"`

## Open Risks

- Risk: Full gate runtimes may exceed target on constrained machines
  - Severity: Medium
  - Owner: Maintainer
  - Mitigation: use `RALPH_CI_JOBS` / `RALPH_XCODE_JOBS` caps; track runtime trends
  - Due: before first public release tag

## Go / No-Go Decision

- Current decision: `GO`
- Condition: keep release branch clean and rerun gates if any additional changes are introduced.

## Sign-off

- Technical sign-off: complete on hardening commit `05a141f1`
- Publication sign-off: maintainer approval pending
