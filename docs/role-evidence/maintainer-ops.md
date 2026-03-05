# Maintainer Ops Evidence

- Claim: CI gate strategy is resource-aware and operationally disciplined
- Evidence link: `Makefile`, `docs/guides/ci-strategy.md`
- Verification command: `RALPH_CI_JOBS=4 RALPH_XCODE_JOBS=4 make agent-ci`
- Expected result: deterministic gate completes without saturating host resources
- Last verified: March 5, 2026 (pre-commit working tree)
