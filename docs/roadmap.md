# Ralph Roadmap

Last updated: 2026-03-28

This is the canonical near-term roadmap for active follow-up work.

## Active roadmap

### 1. Finish collapsing macOS operator guidance onto one canonical doc path

Why next:
- The profiling and cleanup entrypoints now exist, but `Makefile` help, `docs/index.md`, `docs/features/app.md`, `docs/troubleshooting.md`, and `docs/guides/ci-strategy.md` still share overlapping macOS guidance.
- The remaining work is now pure doc-surface cleanup, so it can land without touching the profiling contract again.

Primary outcome:
- The macOS CI/profile/UI-artifact workflow has one primary home, with short pointers elsewhere.

Implementation steps:
- Choose the canonical operator doc for macOS validation, profiling, and UI evidence capture.
- Trim secondary surfaces to one-line pointers or examples only.
- Remove wording that duplicates the shipped profiling and cleanup contract.

Exit criteria:
- The same macOS workflow is no longer described in multiple places with different levels of detail.
- Secondary docs stay short and non-conflicting.

## Sequencing rules

- Keep completed work out of this file.
- Prefer one canonical operator path over wrappers, aliases, or repeated prose.
- Preserve the hardened runtime split boundaries (`runutil/execution`, `runutil/retry`, `runutil/shell`, queue prune, fsutil, eta_calculator, undo, and contracts/task) while refactoring adjacent modules.
