# Ralph Roadmap

Last updated: 2026-03-14

This is the canonical near-term roadmap for active follow-up work.

## Active roadmap

### 1. Broaden post-run supervision regression coverage around adjacent lifecycle edges

Why first:
- The CI enforcement fix is now in place and green.
- The macOS fixture/suite split cutover is complete, so supervision coverage can expand without competing test-structure churn.
- This locks in the new supervision semantics before future run-loop or queue-lifecycle changes.

Scope:
- Add focused coverage for clean/dirty combinations around rejected tasks, already-archived done tasks, queue-maintenance repairs, and publish-mode variants.
- Keep post-run mutation/CI expectations explicit for both queue changes and repo changes.
- Guard the supervision refactor against future regressions without reopening the implementation design.

### 2. Continue consolidating macOS workspace background-task ownership

Why second:
- The teardown-race cutover removed the noisy failures, but more workspace entrypoints still launch ad hoc background tasks.
- Finishing task-ownership cleanup next reduces the chance that later app changes reintroduce nondeterministic lifecycle bugs.
- This work is safer now that the supporting macOS test fixtures and runner suites are decomposed.

Scope:
- Audit remaining fire-and-forget workspace/bootstrap tasks for explicit ownership and cancellation.
- Prefer workspace-owned task slots over detached lifecycle work where repository context matters.
- Keep close/retarget/shutdown semantics deterministic across app and tests.

### 3. Split the remaining oversized macOS persistence and parsing suites after the lifecycle audit settles

Why third:
- `WindowStateTests.swift` remains above the file-size target and still mixes multiple persistence behaviors.
- `ANSIParserTests.swift` is near the soft limit and is a good candidate for behavior-focused decomposition once lifecycle churn subsides.
- Deferring this until after the ownership audit avoids re-splitting files that may still absorb lifecycle-driven test changes.

Scope:
- Break large persistence/parsing suites into behavior-focused files without changing coverage.
- Keep suite-level facade files thin and move reusable support into focused companions only when duplication is real.
- Preserve the current deterministic test-support entrypoints introduced by the recent cutovers.

## Sequencing rules

- Keep completed roadmap items out of this file; replace them with the next active work only.
- Prefer infrastructure and fixture stabilization before broader feature churn.
- Do not reopen the completed Settings window cutover unless a new regression appears.
