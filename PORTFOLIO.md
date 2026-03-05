# Ralph Portfolio Guide

Purpose: give skeptical reviewers a fast, high-signal validation path.

## Reviewer Personas

- Adopter evaluator: "Can I clone this and get value quickly?"
- Senior engineer reviewer: "Is the architecture intentional, reliable, and maintainable?"
- Security-minded reviewer: "Are defaults safe and publication hygiene enforced?"
- Maintainer/operator: "Are CI gates deterministic and resource-conscious?"

## If You Only Read 3 Things

1. [README.md](README.md) — product scope, quickstart, known limitations
2. [docs/architecture.md](docs/architecture.md) — boundaries, flows, recovery model
3. [docs/guides/release-readiness-report.md](docs/guides/release-readiness-report.md) — current evidence snapshot

## Claim → Evidence

- Claim: local clone to working is straightforward
  - Evidence: [README quick start](README.md#quick-start)
  - Verify: `ralph init && ralph task "smoke" && ralph queue list`

- Claim: PR gate is deterministic and local-first
  - Evidence: [Makefile gates](Makefile), [CI strategy](docs/guides/ci-strategy.md)
  - Verify: `make agent-ci`

- Claim: public-release hygiene is enforced
  - Evidence: [pre-public check script](scripts/pre-public-check.sh), [public checklist](docs/guides/public-readiness.md)
  - Verify: `make pre-public-check`

- Claim: architecture is explainable and recoverable
  - Evidence: [architecture overview](docs/architecture.md)
  - Verify: read trust boundaries + failure/recovery sections

- Claim: security expectations are explicit
  - Evidence: [SECURITY.md](SECURITY.md), [security model](docs/security-model.md)
  - Verify: run secret/runtime checks via `scripts/pre-public-check.sh --skip-ci --skip-links --skip-clean`

## Suggested Reviewer Walkthrough (10 minutes)

```bash
# install from source
make install

# no external runner required for this smoke
ralph init
ralph --help
ralph run one --help
ralph scan --help
ralph queue list
ralph queue graph
ralph queue validate
ralph doctor

# required quality gate
RALPH_CI_JOBS=4 make agent-ci
```

## Where the Interesting Engineering Lives

- `crates/ralph/src/main.rs` — startup path and command wiring
- `crates/ralph/src/sanity/mod.rs` — preflight checks and guardrails
- `crates/ralph/src/commands/run/` — supervision, phases, resume/recovery
- `apps/RalphMac/RalphCore/RalphCLIClient.swift` — app ↔ CLI bridge
- `scripts/pre-public-check.sh` — publication safety gates

## Related Evidence

- [Public Readiness Checklist](docs/guides/public-readiness.md)
- [Reviewer Smoke Test](docs/guides/reviewer-smoke-test.md)
- [Role Evidence Index](docs/role-evidence/index.md)
