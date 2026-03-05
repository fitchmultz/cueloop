# Investigation: Public Release Hardening (Deep Investigation)

Date: March 4, 2026

## Summary
The repository is structurally strong (local CI contracts, release flow, docs hub), but had reviewer-visible friction in policy clarity, security/contact ergonomics, pre-public guardrails, and minor packaging/documentation drift. This pass hardened those gaps and re-validated CI-equivalent gates.

## Symptoms
- Public-readiness checks were improving, but still had avoidable friction for iterative use and potential blind spots.
- Reviewer-facing docs had minor policy/onboarding inconsistencies.
- Publish-readiness dry-runs surfaced lockfile quality warnings.

## Investigation Log

### Phase 1 - Baseline and inventory
**Hypothesis:** Existing hardening landed most critical work; remaining issues are cross-file drift and edge-case friction.

**Findings:**
- Main branch had active in-flight hardening changes.
- Required local gates passed (`make agent-ci`, `make ci`), but policy/docs and pre-public details still needed scrutiny.

**Evidence:**
- `git status`: 23 modified + 4 untracked at investigation start.
- `Makefile` gate model and contract tests in `crates/ralph/tests/makefile_ci_contract_test.rs`.

**Conclusion:** Confirmed. Move to targeted cross-file audit.

---

### Phase 2 - Oracle synthesis + direct verification
**Hypothesis:** Remaining high-impact risks are policy/reporting clarity, security scan ergonomics, docs parity, and publish hygiene.

**Findings:**
1. README policy links were incomplete.
2. Security/CoC reporting channels were too implicit for external reporters.
3. CLI docs missed `--debug` coverage despite real support in command help.
4. Runtime lock artifacts were not ignored by default.
5. Pre-public script artifact checks could be stricter and cleaner.
6. Release notes checksum docs were macOS-biased.
7. Lockfile contained yanked transitive wasm packages flagged by publish dry-run.

**Evidence:**
- README policy section includes CONTRIBUTING/SECURITY/CHANGELOG and now includes CoC: `README.md:147-153`.
- Security reporting flow explicitly documented: `SECURITY.md:9-14`, `SECURITY.md:123`.
- CoC enforcement reporting path clarified: `CODE_OF_CONDUCT.md:39-45`.
- CLI docs now include debug flag note + workflow example: `docs/cli.md:8-14`, `docs/cli.md:53-58`.
- `.ralph/lock/` now ignored: `.gitignore:28-33`.
- Pre-public runtime artifact checks include cache/lock/log paths + skip-clean mode + allowlist logic: `scripts/pre-public-check.sh:133-171`, `scripts/pre-public-check.sh:181-239`.
- Release-notes checksum instructions include macOS and Linux commands: `.github/release-notes-template.md:46-53`.
- Rust host target detection now has fallback path in both release scripts:
  - `scripts/build-release-artifacts.sh:73-88`
  - `scripts/release.sh:137-154`, usage at `scripts/release.sh:554-555`.
- Publish dry-run initially warned about yanked `js-sys/wasm-bindgen`; after lock update, warning removed:
  - Command run: `cargo publish -p ralph --dry-run --allow-dirty` (warning removed after `cargo update -p js-sys -p wasm-bindgen`).

**Conclusion:** Confirmed. Issues were real and fixable with low-to-medium code churn.

---

### Phase 3 - Eliminated hypotheses
**Hypothesis:** `rustc --print host-tuple` is unsupported and currently breaking artifact scripts.

**Findings:**
- On this toolchain, `rustc --print host-tuple` succeeds.

**Evidence:**
- Command run: `rustc --print host-tuple` returned `aarch64-apple-darwin`.

**Conclusion:** Eliminated as an immediate breakage. Still hardened scripts with a fallback parser for broader compatibility.

## Root Cause
The remaining friction came from **contract coverage asymmetry**:
- CI pipeline parity had strong enforcement (Makefile + contract tests),
- but adjacent areas (security/community reporting text, quick-start caveats, release artifact UX, pre-public ergonomics, lockfile hygiene) relied more on manual alignment.

This created small but reviewer-visible inconsistencies despite otherwise strong engineering controls.

## Recommendations
1. Keep CI/docs contracts explicit and test-backed where feasible (especially release/public-readiness docs).
2. Add optional secret scanner integration (e.g., gitleaks) as a non-blocking pre-public check enhancement.
3. Track gate runtime benchmarks over time to catch regressions (`ci-fast`, `ci`, `macos-ci`).
4. Add architecture sequence diagrams for parallel/session-recovery flows.

## Preventive Measures
- Continue using `scripts/pre-public-check.sh` for pre-public windows.
- Run `make ci` and `cargo publish --dry-run --allow-dirty` before release prep.
- Keep policy docs (README/SECURITY/CODE_OF_CONDUCT) updated alongside workflow/script changes.
- Keep lockfile healthy by addressing yanked package warnings when they appear.
