# Release Readiness Report (March 4, 2026)

Purpose: capture the current public-release hardening status, major risks addressed, and remaining follow-ups.

## Top Risks Found and Mitigations Applied

1. **Duplicate release builds during CI/macOS gates**
   - Mitigation: added a release-build stamp target in `Makefile` to deduplicate release build work within a single invocation.

2. **Unbounded cargo/nextest/xcodebuild parallelism impacting workstation responsiveness**
   - Mitigation: added `RALPH_CI_JOBS` and `RALPH_XCODE_JOBS` knobs with conservative defaults and documented usage.

3. **CI contract drift after introducing `ci-fast` factoring**
   - Mitigation: updated `makefile_ci_contract_test.rs` to validate semantic `ci` expansion, assert `ci-fast` contract explicitly, and align `agent-ci` routing expectations.

4. **Release workflow brittleness (changelog links, multiline notes templating, and dry-run blind spots)**
   - Mitigation: hardened `scripts/release.sh` with robust changelog base detection, safe template rendering, and dry-run validation of key transforms.

5. **UI test runner Gatekeeper failures (“damaged” runner app)**
   - Mitigation: Makefile UI-test flows now clear quarantine metadata and perform ad-hoc re-signing before test execution.

6. **Pre-public scan noise and limited runtime-artifact checks**
   - Mitigation: `scripts/pre-public-check.sh` now supports fixture allowlisting, iterative `--skip-clean` audits, and explicit checks for tracked `.ralph/cache`, `.ralph/lock`, and `.ralph/logs` artifacts.

7. **Runtime lock artifacts were not ignored by default**
   - Mitigation: added `.ralph/lock/` to `.gitignore`.

8. **Docs drift in reviewer-facing onboarding/policy references**
   - Mitigation: added `CODE_OF_CONDUCT.md` link in README, GNU Make caveat in quick-start, and `--debug` usage coverage in CLI docs.

9. **Security/CoC reporting paths were too implicit**
   - Mitigation: updated `SECURITY.md`, `CODE_OF_CONDUCT.md`, and security feature docs to point to explicit reporting flows (GitHub private vulnerability reporting + maintainer profile contact path).

10. **Publish-readiness warning: yanked transitive dependencies in lockfile**
   - Mitigation: updated `Cargo.lock` (`js-sys`/`wasm-bindgen` family) and re-verified `cargo publish --dry-run --allow-dirty` without yanked warnings.

## Remaining Known Issues / Follow-ups

- Final publish rehearsal still requires a clean tree run of `scripts/pre-public-check.sh` and `RELEASE_DRY_RUN=1 scripts/release.sh <version>` (currently blocked by active in-flight changes).
- Decide long-term policy for committed `.ralph/done.jsonc` history (keep as sanitized dogfooding evidence vs reduce committed runtime state surface).
- Expand dedicated architecture documentation with sequence diagrams for parallel-worker integration and session recovery internals.
- Add a tracked benchmark log for gate runtimes across representative hardware profiles.
- Add optional secret scanning integration with a dedicated tool (for example, gitleaks) in pre-public checks.

## Private-History Cleanup Plan (Safe While Repo Is Private)

An execution-ready, hash-specific rebase plan is now tracked in:

- [`docs/guides/history-cleanup-execution-plan.md`](history-cleanup-execution-plan.md)

That plan includes:

- exact commit grouping map from current `main` log (`HEAD~50` as of `f96ab95b`)
- a ready-to-paste `git rebase -i` todo list (`pick/reword/fixup/drop` actions)
- explicit backup/rollback commands and post-rewrite verification gates

If collaborators already depend on this history, avoid force-push and use forward cleanup commits instead.

## Before/After DX Summary

### Before

- CI and macOS checks performed redundant release-build work.
- Resource usage could spike during routine validation.
- Public docs did not consistently explain GNU Make expectations.

### After

- Fast/regular/full validation paths are clearer and more intentional.
- Default gate execution is less disruptive on shared developer machines.
- Release and public-readiness workflows are better documented and less brittle.
- Iterative audits can run without stashing work (`--skip-clean`) while final publish gates remain strict.
- Security and conduct reporting paths are explicit enough for external reporters.
- Publish-readiness checks no longer surface yanked-lockfile warnings.
