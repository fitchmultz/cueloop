# Release Readiness Report

Purpose: capture an evidence-based publication snapshot for Ralph.

## Snapshot Metadata

- Date: March 5, 2026
- Base commit before this hardening pass: `caa59e79`
- Environment: local macOS workstation (GNU Make + pinned Rust toolchain)
- Scope: public-release hardening pass (CI/safety/docs/reviewer evidence)

## Gate Results

Status values: `PASS`, `FAIL`, or `PENDING`.

- `make agent-ci`: `PASS`
- `make ci`: `PASS`
- `make pre-public-check`: `PASS` on clean temporary snapshot of current changes
- Reviewer deterministic smoke sequence (CLI help + focused contract tests): `PASS`

Evidence commands run:

```bash
make check-repo-safety
make agent-ci
make ci
scripts/pre-public-check.sh --skip-clean

# clean-snapshot verification of full strict gate
make pre-public-check

# skeptical reviewer deterministic sequence
cargo run -p ralph -- --help
cargo run -p ralph -- run one --help
cargo run -p ralph -- scan --help
cargo test -p ralph --bin ralph normalize_repo_prompt_args -- --nocapture
cargo test -p ralph cli_parses_run_ -- --nocapture
cargo test -p ralph cli_rejects_run_loop_with_id_flag -- --nocapture
cargo test -p ralph --test run_cli_overrides_contract_test -- --nocapture
```

Notes:

- Running `make pre-public-check` in an in-progress working tree correctly fails the clean-worktree check by design.
- Running `make pre-public-check` on a clean temporary snapshot of these exact changes passed end-to-end.

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
- Improved reviewer-facing docs:
  - README cold-start clarity (scope, non-goals, limitations, security/data handling)
  - Architecture trust boundaries + failure/recovery details
  - Public checklist + portfolio evidence alignment
  - Added support/version/security/troubleshooting/runbook/smoke-test docs
  - Added role evidence pack under `docs/role-evidence/`
- Determinism upgrade:
  - Added `rust-toolchain.toml`
  - Added crate-level `rust-version = "1.93"`

## Open Risks

- Risk: Full gate runtimes may exceed target on constrained machines
  - Severity: Medium
  - Owner: Maintainer
  - Mitigation: use `RALPH_CI_JOBS` / `RALPH_XCODE_JOBS` caps; track runtime trends
  - Due: before first public release tag

- Risk: Optional history rewrite not yet executed
  - Severity: Low (unless noisy/sensitive history remains)
  - Owner: Repo owner
  - Mitigation: follow `history-cleanup-execution-plan.md` only if still private and safe
  - Due: before making repository broadly public

## Go / No-Go Decision

- Current decision: `CONDITIONAL GO`
- Condition: publish from a clean tree with the same gate outcomes recorded above.

## Sign-off

- Technical sign-off: ready pending final clean-tree commit
- Publication sign-off: pending maintainer approval and (optional) private-history cleanup decision
