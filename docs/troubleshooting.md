# Troubleshooting
Status: Active
Owner: Maintainers
Source of truth: this document for its stated scope
Parent: [CueLoop Documentation](index.md)


Purpose: provide fast resolution paths for common setup and CI failures.

## GNU Make Errors on macOS

Symptom: Makefile errors about GNU Make version.

Fix:

```bash
brew install make
gmake agent-ci
```

## `make agent-ci` Fails on Env Safety

Symptom: tracked env file detected.

Fix:

```bash
git rm --cached <env-file>
# keep only .env.example tracked
```

## `make pre-public-check` Fails on Runtime Artifacts

Symptom: tracked `.cueloop/...` runtime paths or build outputs detected.

Fix:

```bash
git rm --cached -r apps/CueLoopMac/build .cueloop/cache .cueloop/logs .cueloop/lock .cueloop/workspaces .cueloop/undo .cueloop/webhooks
```

Then rerun:

```bash
make pre-public-check
```

## Test Failures in Temporary Directory Logic

Symptom: flaky integration tests around temp paths or queue fixtures.

Fixes:

- ensure `cueloop init --non-interactive` is used in tests
- rerun with `make test` to use the project harness
- inspect `crates/cueloop/tests/test_support.rs` helpers for deterministic setup

## macOS App Build/Test Failures

Symptom: xcodebuild failures or UI test runner signing/quarantine issues.

Fixes:

```bash
make macos-build
make macos-test
# for interactive UI runs only
make macos-ui-build-for-testing
make macos-ui-retest
```

If macOS prompts for password/Touch ID before a UI run, that is the system approving Accessibility/Automation for a rebuilt test bundle. Reduce repeated prompts by building once and then iterating with `make macos-ui-retest` instead of rebuilding every run.

If an interrupted run strands `target/tmp/locks/xcodebuild.lock`, rerun the same target. CueLoop now removes stale project-owned Xcode build locks automatically once the recorded owner PID is gone, and it keeps waiting only for live holders.

Symptom: Xcode or `make macos-build` looks stuck, shows a stale bundled CLI, or you suspect corrupted DerivedData under `target/tmp/xcode-deriveddata`.

Fixes:

- Default Make targets **reuse** Xcode DerivedData for faster incremental builds. To wipe the tree for that lane before running it, either set `CUELOOP_XCODE_CLEAN_DERIVED_DATA=1` for one invocation or use an explicit clean wrapper: `make macos-build-clean`, `make macos-test-clean`, `make macos-ci-clean`, `make macos-ui-build-for-testing-clean`, or `make macos-test-window-shortcuts-clean`.
- For a broader cold reset of temp outputs (stamp + all default DerivedData locations), use `make clean-temp` (see [`docs/guides/ci-strategy.md`](guides/ci-strategy.md#cleaning-targettmp)).
- To inspect Cargo and default Xcode cache paths (sizes and entry counts), run `make build-cache-doctor`.
- When iterating **only** in Xcode, the “Build and Bundle CueLoop” run script consults `apps/CueLoopMac/CueLoopCLIInputs.xcfilelist` so Swift-only edits do not re-bundle the CLI; if Rust or bundling scripts changed and Xcode did not pick it up, run a clean build lane above or touch the inputs the xcfilelist tracks.

For gate choice, shared-workstation caps, and preserved UI evidence capture, use [`docs/guides/ci-strategy.md`](guides/ci-strategy.md).

## `make coverage` Fails (Missing Tools)

Symptom: `make coverage` exits with `cargo-llvm-cov not found` (or missing `jq` only affects formatted terminal summaries).

Fix:

```bash
cargo install cargo-llvm-cov --locked
# On macOS you may also need:
rustup component add llvm-tools-preview
```

Coverage is optional and outside the default `make agent-ci` graph. HTML is written under `target/coverage/html/`; the recipe prints the path to open manually (see `mk/coverage.mk` and [`CONTRIBUTING.md`](../CONTRIBUTING.md)). To drop generated coverage outputs and profile scraps, run `make coverage-clean`.

## Need Visual Evidence from UI Tests

Symptom: UI run appears noisy/flaky but tests still pass, and you need inspectable visuals.

Use `make macos-test-ui-artifacts` for preserved `.xcresult` output, or use `CUELOOP_UI_ONLY_TESTING=... make macos-ui-retest` for focused reruns. Keep the full workflow in [`docs/guides/ci-strategy.md`](guides/ci-strategy.md).
