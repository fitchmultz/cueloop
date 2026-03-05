# Release Manager Evidence

- Claim: release workflow is deterministic and documented
- Evidence link: `Makefile`, `docs/guides/release-runbook.md`, `docs/guides/public-readiness.md`
- Verification command: `make ci && make pre-public-check`
- Expected result: all required gates pass with clean worktree
- Last verified: March 5, 2026 (pre-commit working tree)
