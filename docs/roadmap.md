# Ralph Roadmap

Last updated: 2026-03-13

This is the canonical near-term roadmap for active follow-up work.

## Active roadmap

### 1. Reduce macOS test noise from fixture teardown and async refresh races

Why first:
- The shared mock-fixture cutover is now in place, so the remaining macOS churn is lifecycle noise rather than payload drift.
- Current passing test runs still emit benign-but-noisy runner-configuration failures after temporary fixture executables are removed.
- Quieting that noise will make real regressions easier to spot before more app-surface changes land.

Scope:
- Prevent background runner-config or watcher refresh work from outliving test fixtures.
- Tighten workspace/test teardown so temporary CLI binaries and temp directories are not observed after cleanup.
- Keep operational-health diagnostics meaningful instead of flooding logs with expected teardown errors.

### 2. Split oversized macOS test support and runner-configuration suites after the fixture cutover

Why second:
- The new shared mock-fixture layer reduced duplication, but `WorkspaceRunnerConfigurationTests.swift` still carries too many behaviors in one file.
- Decomposition is lower risk now that shared builders and resolved-path payloads are centralized.
- Smaller files will keep future macOS test churn localized and easier to review.

Scope:
- Break large RalphCore test suites into behavior-focused files without changing coverage.
- Keep `RalphCoreTestSupport.swift` and related helpers as thin facades over focused support files where needed.
- Preserve deterministic temp-fixture and queue-path helpers as the single source of truth.

### 3. Broaden post-run supervision regression coverage around adjacent lifecycle edges

Why third:
- The CI enforcement fix is now in place and green.
- Expanding coverage is safest after the macOS fixture churn above is reduced.
- This locks in the new supervision semantics before future run-loop or queue-lifecycle changes.

Scope:
- Add focused coverage for clean/dirty combinations around rejected tasks, already-archived done tasks, queue-maintenance repairs, and publish-mode variants.
- Keep post-run mutation/CI expectations explicit for both queue changes and repo changes.
- Guard the supervision refactor against future regressions without reopening the implementation design.

## Sequencing rules

- Keep completed roadmap items out of this file; replace them with the next active work only.
- Prefer infrastructure and fixture stabilization before broader feature churn.
- Do not reopen the completed Settings window cutover unless a new regression appears.
