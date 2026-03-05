# Current State Baseline (March 5, 2026)

Purpose: one-page snapshot of Ralph’s stack, primary flows, and top publication risks.

## Stack and Structure

- Core runtime: Rust CLI (`crates/ralph/`)
- Optional app: SwiftUI macOS app (`apps/RalphMac/`)
- State model: repo-local `.ralph/queue.jsonc` + `.ralph/done.jsonc` (+ optional `.ralph/config.jsonc`)
- Local CI gates: Makefile targets (`agent-ci`, `ci`, `macos-ci`, `pre-public-check`)

## Primary Flows

Install and setup:

```bash
make install
ralph init
```

Core queue/task workflow:

```bash
ralph task "<work item>"
ralph queue list
ralph run one
```

Public release hygiene:

```bash
make agent-ci
make ci
make pre-public-check
```

## Known Failure Classes (Before Hardening Pass)

- Secret-scan false positives due to allowlist pattern shape in pre-public script
- Incomplete tracked artifact checks for some `.ralph` runtime directories
- Weaker tracked env-file detection (`.env` only)
- Docs gaps for trust boundaries, limitations, version/support policy, and reviewer evidence mapping

## Top Risks

P0:

- Secret leakage and runtime artifact tracking regressions without strong always-on gates

P1:

- Reviewer friction due to docs drift or unclear verification pathways
- Determinism drift without explicit toolchain policy

P2:

- Historical docs/noisy history reducing confidence even when code quality is strong

## Exit Criteria for Public-Ready State

- Required gates are green and reproducible (`make agent-ci`, `make ci`, `make pre-public-check`)
- No tracked runtime/build artifacts outside policy allowlist
- No tracked env files except `.env.example`
- Docs provide cold-start onboarding, architecture clarity, and reviewer evidence path
