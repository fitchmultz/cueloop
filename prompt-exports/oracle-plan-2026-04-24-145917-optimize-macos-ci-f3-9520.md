## Final Prompt
<taskname="Optimize macos-ci"/>
<task>Propose exactly one next optimization to reduce total wall-clock for `make macos-ci` (single attributed change only, not a list). The output must include: (1) the specific change, (2) why it should move wall-clock, (3) risks to behavior/correctness/visibility, and (4) an exact verification plan proving no meaningful checks regressed.</task>

<architecture>`make macos-ci` is orchestrated in `Makefile` and composes two major surfaces: Rust CI (`ci` → `ci-fast` → `check-env-safety`/format/type/lint/test/build/generate/install-verify) and macOS validation (`macos-build`, `macos-test`, `macos-test-contracts`). `macos-ci` forces shared derived-data reuse within a run (`RALPH_XCODE_REUSE_SHIP_DERIVED_DATA=1`, `RALPH_XCODE_KEEP_DERIVED_DATA=1`) but by default clears `target/tmp/xcode-deriveddata/ship` at start/end unless outer `RALPH_XCODE_KEEP_DERIVED_DATA=1` is set.

`check-env-safety` delegates to `scripts/pre-public-check.sh --skip-ci --skip-links --skip-clean --allow-no-git`, which still performs required-file checks plus tracked-path and secret scans (via `scripts/lib/public_readiness_scan.sh` → `public_readiness_scan.py`).

macOS deterministic contract checks are explicit scripts (`scripts/macos-settings-smoke.sh`, `scripts/macos-workspace-routing-contract.sh`) invoked via `macos-test-contracts`; Xcode invocations are serialized through `scripts/lib/xcodebuild-lock.sh`. Release CLI build dedup is stamp-driven (`RALPH_RELEASE_BUILD_STAMP`) and implemented through `scripts/ralph-cli-bundle.sh`.</architecture>

<selected_context>
/Users/mitchfultz/Projects/AI/ralph/prompt-exports/optimize-macos-ci-runs.md: Baseline and phase timing scoreboard (316.450s total; top phases include Xcode non-UI tests 107.649s, pre-public readiness checks 73.715s, Rust tests 73.573s, Xcode Release build 53.367s).
/Users/mitchfultz/Projects/AI/ralph/Makefile: Full gate graph and timing-critical targets (`ci-fast`, `ci`, `check-env-safety`, `macos-build`, `macos-test`, `macos-test-contracts`, `macos-ci`), plus knobs (`RALPH_CI_JOBS`, `RALPH_XCODE_JOBS`, `RALPH_XCODE_KEEP_DERIVED_DATA`).
/Users/mitchfultz/Projects/AI/ralph/scripts/pre-public-check.sh: Public-readiness orchestration, skip flags, and exact checks executed by `check-env-safety`.
/Users/mitchfultz/Projects/AI/ralph/scripts/lib/public_readiness_scan.sh: Wrapper for focused `links|secrets|session-paths` scans.
/Users/mitchfultz/Projects/AI/ralph/scripts/lib/public_readiness_scan.py: Actual repo-wide scan implementation and secret/link/session-path logic.
/Users/mitchfultz/Projects/AI/ralph/scripts/lib/release_policy.sh: Policy constants and path-tiering helpers used by public-readiness checks.
/Users/mitchfultz/Projects/AI/ralph/scripts/profile-ship-gate.sh: Existing profiling harness and per-phase timing bundle generation.
/Users/mitchfultz/Projects/AI/ralph/scripts/ralph-cli-bundle.sh: Canonical release CLI build path used by stamp-based build/generate/install flow.
/Users/mitchfultz/Projects/AI/ralph/scripts/lib/ralph-shell.sh: Shared shell helpers (repo root, toolchain activation, make resolution).
/Users/mitchfultz/Projects/AI/ralph/scripts/lib/xcodebuild-lock.sh: Xcode lock acquisition/release and stale lock behavior.
/Users/mitchfultz/Projects/AI/ralph/scripts/macos-settings-smoke.sh: Deterministic Settings contract coverage invoked in `macos-test-contracts`.
/Users/mitchfultz/Projects/AI/ralph/scripts/macos-workspace-routing-contract.sh: Deterministic workspace routing contract coverage invoked in `macos-test-contracts`.
/Users/mitchfultz/Projects/AI/ralph/AGENTS.md: Repo-specific constraints/preferences for CI and validation expectations.
/Users/mitchfultz/Projects/AI/ralph/apps/AGENTS.md: App-scoped guidance confirming `make macos-ci` as canonical ship gate.
</selected_context>

<relationships>
- `macos-ci` → `ci` (Rust release gate) + `macos-build` + `macos-test` + `macos-test-contracts`.
- `ci` → `ci-fast` + `build` + `generate` + `install-verify`.
- `ci-fast` → `check-env-safety` + `check-backup-artifacts` + `deps` + `format-check` + `type-check` + `lint` + `test`.
- `check-env-safety` → `scripts/pre-public-check.sh --skip-ci --skip-links --skip-clean --allow-no-git`.
- `pre-public-check.sh` secret scan path → `public_readiness_scan.sh secrets` → `public_readiness_scan.py scan_secrets()`.
- `macos-test-contracts` → `macos-test-settings-smoke` + `macos-test-workspace-routing-contract` (both depend on `macos-build`).
- `macos-build`/`macos-test`/UI targets share Xcode lock helper (`xcodebuild-lock.sh`) and derived-data policy from `Makefile` variables.
</relationships>

<ambiguities>
- The baseline table labels a 73.715s phase as “Pre-public readiness checks.” In current `macos-ci` wiring, this likely corresponds to `check-env-safety` (a skip-flag subset of `pre-public-check.sh`), but the timing artifact naming may be broader than that single target.
- No external global AGENTS file outside the repo root could be added in this workspace; only repo-local guidance files are included.
</ambiguities>

## Selection
- Files: 14 total (14 full)
- Total tokens: 42308 (Auto view)
- Token breakdown: full 42308

### Files
### Selected Files
/Users/mitchfultz/Projects/AI/ralph/
├── apps/
│   └── AGENTS.md — 1,549 tokens (full)
├── prompt-exports/
│   └── optimize-macos-ci-runs.md — 507 tokens (full)
├── scripts/
│   ├── lib/
│   │   ├── public_readiness_scan.py — 2,916 tokens (full)
│   │   ├── public_readiness_scan.sh — 891 tokens (full)
│   │   ├── ralph-shell.sh — 1,134 tokens (full)
│   │   ├── release_policy.sh — 2,940 tokens (full)
│   │   └── xcodebuild-lock.sh — 1,296 tokens (full)
│   ├── macos-settings-smoke.sh — 1,802 tokens (full)
│   ├── macos-workspace-routing-contract.sh — 2,116 tokens (full)
│   ├── pre-public-check.sh — 2,734 tokens (full)
│   ├── profile-ship-gate.sh — 1,532 tokens (full)
│   └── ralph-cli-bundle.sh — 1,196 tokens (full)
├── AGENTS.md — 11,939 tokens (full)
└── Makefile — 9,756 tokens (full)


---

## Generated Plan

## Chat Send ✅
- **Chat**: `optimize-macos-ci-F3B4E6` | **Mode**: plan

### Response
## Summary

Propose one optimization: change `Makefile`’s `macos-ci` orchestration from fully serial execution to a two-lane gate that prebuilds the shared release CLI stamp once, then runs Rust `ci` and macOS validation concurrently. This preserves the same checks while reducing total wall-clock from roughly the sum of Rust CI + macOS validation to roughly `release-build + max(Rust CI, macOS validation)`.

## Current-state analysis

`make macos-ci` currently runs:

1. `macos-preflight`
2. shared derived-data cleanup/setup
3. `make ci`
   - `ci-fast`
   - `build`
   - `generate`
   - `install-verify`
4. `make macos-build macos-test macos-test-contracts`
   - uses shared `target/tmp/xcode-deriveddata/ship`
   - serializes Xcode via `scripts/lib/xcodebuild-lock.sh`
   - deterministic contract scripts depend on `macos-build`

The baseline shows large independent surfaces being serialized:

- Pre-public readiness checks: `73.715s`
- Rust tests: `73.573s`
- Xcode non-UI tests: `107.649s`
- Xcode Release build: `53.367s`

The key blocker to naïve parallelism is `RALPH_RELEASE_BUILD_STAMP`: both `ci` and `macos-build` can require the release CLI binary. The optimization must therefore build/verify that stamp once before launching parallel lanes.

## Specific change

Modify only the `macos-ci` target in `Makefile` so it:

1. Keeps the existing derived-data cleanup/trap behavior.
2. Runs `$(MAKE) --no-print-directory build` once before parallel work to materialize `$(RALPH_RELEASE_BUILD_STAMP)`.
3. Starts two background lanes:
   - Rust lane: `$(MAKE) --no-print-directory ci`
   - macOS lane: existing `$(MAKE) --no-print-directory macos-build macos-test macos-test-contracts RALPH_XCODE_REUSE_SHIP_DERIVED_DATA=1 RALPH_XCODE_KEEP_DERIVED_DATA=1`
4. Waits for both lanes.
5. Fails `macos-ci` if either lane fails.
6. Preserves final success messaging only when both lanes pass.

Illustrative shape only:

```sh
# inside macos-ci recipe
$(MAKE) --no-print-directory build

run_rust_ci_in_background
run_macos_validation_in_background

wait_for_both_lanes
if rust_failed || macos_failed; then
  exit 1
fi
```

This is a targeted orchestration change, not a broader refactor, because all existing target definitions and scripts already express the required checks correctly.

## Why it should move wall-clock

Today, Rust CI and macOS validation are serialized even though most work is independent after the release binary exists. Prebuilding the release stamp removes the shared mutable dependency, allowing:

```text
current:  release/build prerequisite + Rust CI + macOS validation
after:    release/build prerequisite + max(Rust CI, macOS validation)
```

Given the baseline, the overlapable work is substantial: Rust CI includes the `73.715s` readiness phase and `73.573s` Rust test phase, while macOS validation includes `107.649s` Xcode tests and `53.367s` Xcode build. Even with CPU/IO contention, overlapping these surfaces should reduce total wall-clock materially because the current critical path is mostly serialized independent work.

## Risks to behavior, correctness, and visibility

- **Resource contention:** Cargo/nextest and Xcode may compete for CPU, disk, and memory. Existing knobs `RALPH_CI_JOBS` and `RALPH_XCODE_JOBS` remain the mitigation.
- **Failure ordering changes:** macOS validation may continue running after Rust CI has already failed. Correctness is preserved because final status is nonzero if either lane fails.
- **Log readability:** Parallel output can interleave. The implementation should either prefix lane output or capture per-lane logs and print a clear final failure summary.
- **Release-stamp race:** If the stamp is not built before parallel lanes, both lanes could try to build or inspect it concurrently. The required prebuild step avoids this.
- **Interrupt cleanup:** Parent `macos-ci` must terminate/wait for both child lanes on `INT`/`TERM` so Xcode locks and derived-data cleanup still happen.

## Exact verification plan

1. **Baseline before change**
   - Run:
     ```sh
     /usr/bin/time -p make macos-ci > /tmp/macos-ci-before.log 2>&1
     ```
   - Save:
     - total wall-clock
     - Rust test summary/count
     - Xcode test summary/count
     - presence of readiness, schema generation, install verification, settings smoke, and workspace routing contract output.

2. **Functional verification after change**
   - Run:
     ```sh
     /usr/bin/time -p make macos-ci > /tmp/macos-ci-after.log 2>&1
     ```
   - Require exit code `0`.
   - Verify `/tmp/macos-ci-after.log` contains evidence of all meaningful checks:
     - `Pre-public readiness checks`
     - `No tracked runtime/build artifacts detected`
     - `No tracked local-only files detected`
     - `No high-confidence secret patterns found`
     - `Formatting OK`
     - `Type-checking complete`
     - `Linting complete`
     - `Tests passed`
     - `Schemas generated`
     - `CI completed`
     - Xcode `build` success
     - Xcode `test` success
     - `Settings smoke contract passed`
     - `Workspace routing contract passed`
     - `macOS CI completed`

3. **Regression comparison**
   - Compare before/after Rust and Xcode test counts from the logs.
   - Require no reduction in executed Rust tests or Xcode non-UI tests unless explained by an unrelated intentional test change.
   - Require deterministic contract scripts to run exactly once each.

4. **Failure propagation check**
   - Force the Rust lane to fail with an existing override such as:
     ```sh
     RALPH_PUBLIC_READINESS_SCAN_PY=/bin/false make macos-ci
     ```
   - Require final `make macos-ci` exit code nonzero.
   - Require the failure is attributed to the Rust/readiness lane.

5. **Wall-clock proof**
   - Run three warm-cache post-change samples:
     ```sh
     /usr/bin/time -p make macos-ci
     ```
   - Compare median post-change wall-clock against the saved baseline.
   - Keep the change only if median total wall-clock decreases and all check-presence assertions above still pass.


> 💡 Continue this plan conversation with ask_oracle(chat_id: "optimize-macos-ci-F3B4E6", new_chat: false)