# Ralph Roadmap

Last updated: 2026-03-14

This is the canonical near-term roadmap for active follow-up work.

## Active roadmap

### 1. Continue consolidating macOS workspace background-task ownership

Why first:
- The default macOS CI gates are now noninteractive again, so the next highest-leverage churn reducer is finishing workspace task ownership while app checks are cheap to rerun.
- The teardown-race cutover removed the noisy failures, but more workspace entrypoints still launch ad hoc background tasks.
- Completing ownership cleanup before broadening more macOS contract coverage reduces the chance of encoding flaky lifecycle behavior into new harnesses.

Scope:
- Audit remaining fire-and-forget workspace/bootstrap tasks for explicit ownership and cancellation.
- Prefer workspace-owned task slots over detached lifecycle work where repository context matters.
- Keep close/retarget/shutdown semantics deterministic across app and tests.

### 2. Broaden noninteractive macOS contract coverage beyond the Settings cutover

Why second:
- The Settings smoke now runs offscreen in-process, and the remaining CI-path foreground launch regression has been cut over to the shared presentation runtime.
- The next best leverage is moving more fragile window/bootstrap/URL lifecycle checks onto the same noninteractive contract path before they drift behind headed-only coverage.
- Doing this after the ownership audit avoids baking transient lifecycle races into new contract assertions.

Scope:
- Identify other app flows that still depend on headed-only verification and migrate the highest-value ones to deterministic in-process contracts.
- Reuse the offscreen presentation/runtime helpers instead of adding new one-off harnesses.
- Keep contract reports machine-readable so `make macos-ci` can stay noninteractive by default.

### 3. Split the remaining oversized macOS persistence and parsing suites after the lifecycle audit settles

Why third:
- `WindowStateTests.swift` remains above the file-size target and still mixes multiple persistence behaviors.
- `ANSIParserTests.swift` is near the soft limit and is a good candidate for behavior-focused decomposition once lifecycle churn subsides.
- Deferring this until after the ownership audit and contract broadening avoids re-splitting files that may still absorb lifecycle-driven test changes.

Scope:
- Break large persistence/parsing suites into behavior-focused files without changing coverage.
- Keep suite-level facade files thin and move reusable support into focused companions only when duplication is real.
- Preserve the current deterministic test-support entrypoints introduced by the recent cutovers.

### 4. Extend supervision hardening to parallel-worker and revert-mode edge cases

Why fourth:
- Standard post-run supervision now has broader lifecycle regression coverage, so the remaining higher-risk seams are worker-specific restore flows and revert/error branches.
- This should follow the macOS lifecycle audit so app/runtime churn does not mask supervision regressions during verification.
- Keeping this after the current Rust test cutover avoids mixing queue/git behavior expansions with the just-finished standard supervision coverage pass.

Scope:
- Add targeted coverage for parallel-worker bookkeeping restore failures, revert-mode inconsistency paths, and adjacent publish-mode/rebase surfaces not exercised by standard post-run tests.
- Keep runtime test modules behavior-grouped and thin at the root.
- Preserve the current cutover semantics; do not reintroduce compatibility branches.

## Sequencing rules

- Keep completed roadmap items out of this file; replace them with the next active work only.
- Prefer infrastructure and fixture stabilization before broader feature churn.
- Do not reopen the completed Settings window cutover unless a new regression appears.
