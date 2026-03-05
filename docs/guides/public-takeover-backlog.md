# Public Takeover Backlog

Purpose: track prioritized release-hardening work for public readiness.

## P0 (Release blockers)

- [x] Fix pre-public secret-scan false positive in allowlist handling
- [x] Expand tracked runtime artifact checks (`.ralph/workspaces`, `undo`, `webhooks`)
- [x] Enforce tracked `.ralph` allowlist policy in publication audit
- [x] Tighten tracked env-file detection (`.env*`, excluding `.env.example`)
- [x] Ensure required local gates run safety checks by default (`check-env-safety` delegation)

## P1 (High priority confidence)

- [x] Add deterministic toolchain policy (`rust-toolchain.toml`, crate `rust-version`)
- [x] Refresh README/docs for cold-start reviewer confidence
- [x] Add trust-boundary + failure/recovery architecture documentation
- [x] Add reviewer smoke-test and role-evidence pack
- [ ] Track gate runtime baselines on representative hardware profiles

## P2 (Polish / longer-tail)

- [ ] Optional dedicated secret scanner integration (for example, gitleaks)
- [ ] Optional architecture deep-dive diagrams for additional run paths
- [ ] Optional private-history cleanup execution (only if still safe and private)
