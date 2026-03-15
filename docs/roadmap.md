# Ralph Roadmap

Last updated: 2026-03-14

This is the canonical near-term roadmap for active follow-up work.

## Active roadmap

### 1. Extend supervision hardening to parallel-worker and revert-mode edge cases

Why first:
- Standard post-run supervision now has broader lifecycle regression coverage, so the remaining higher-risk seams are worker-specific restore flows and revert/error branches.
- With the macOS contract/test-suite split work now landed, the next highest-value regression gap is back in the Rust supervision flow.
- Tackling this before another app-side refactor wave keeps queue/git behavior expansion isolated during verification.

Scope:
- Add targeted coverage for parallel-worker bookkeeping restore failures, revert-mode inconsistency paths, and adjacent publish-mode/rebase surfaces not exercised by standard post-run tests.
- Keep runtime test modules behavior-grouped and thin at the root.
- Preserve the current cutover semantics; do not reintroduce compatibility branches.

### 2. Split the remaining oversized macOS app/core orchestration files after the suite cutover

Why second:
- The persistence/parsing suite split is complete, but several macOS production files still sit above the file-size target and continue to mix multiple responsibilities.
- Queue/file-watching, runner control, settings infrastructure, and workspace presentation are now the most obvious app-side decomposition debt.
- Keeping this after the supervision pass avoids mixing Rust workflow churn with another broad Swift structural refactor in the same verification window.

Scope:
- Decompose the current oversized macOS files (`QueueFileWatcher.swift`, `WorkspaceRunnerController.swift`, `ASettingsInfra.swift`, `AppSettings.swift`, `WorkspaceView.swift`, and `RunControlDetailSections.swift`) into thinner facades plus focused companion files.
- Preserve the current app/runtime/settings contracts and the recent noninteractive contract behavior while splitting responsibilities.
- Reuse shared support only when duplication is real; otherwise keep behavior-grouped companions adjacent to the facade.

## Sequencing rules

- Keep completed roadmap items out of this file; replace them with the next active work only.
- Prefer infrastructure and fixture stabilization before broader feature churn.
- Do not reopen the completed Settings/workspace-routing contract cutovers unless a new regression appears.
