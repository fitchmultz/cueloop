# Ralph Roadmap

Last updated: 2026-03-31

This is the canonical near-term roadmap for active follow-up work.
Source: comprehensive codebase audit (`docs/audits/codebase-audit-2026-03-31.md`)

## Active roadmap

### 1. Add `set -euo pipefail` to 8 library shell scripts
- `scripts/lib/ralph-shell.sh`, `scripts/lib/release_changelog.sh`, `scripts/lib/release_pipeline.sh`, `scripts/lib/release_policy.sh`, `scripts/lib/release_publish_pipeline.sh`, `scripts/lib/release_state.sh`, `scripts/lib/release_verify_pipeline.sh`, `scripts/lib/release_verify_state.sh`, `scripts/lib/xcodebuild-lock.sh`

### 2. Replace `unreachable!()` with graceful fallback in `notification/display.rs`
- `NotificationType::LoopComplete` match arm should `log::warn` + early return

### 3. Add explanatory comments to silent `Ok(_) => {}` success branches
- `fsutil/atomic.rs`, `undo/storage.rs`, `queue/backup.rs`

### 4. Check `setsid()` return value in daemon start
- `commands/daemon/start.rs:120` — log on failure instead of silently ignoring

### 5. Split 3 largest production files below 500 LOC
- `runner/error.rs` (530) → extract Display impl to `runner/error/display.rs`
- `queue/operations/mutation.rs` (522) → extract helpers to `mutation/helpers.rs`
- `queue/operations/batch/mod.rs` (512) → extract validation to `batch/validation.rs`

### 6. Split top 3 test suites below 600 LOC
- `queue/operations/tests/batch.rs` (753) → `batch_basic.rs`, `batch_edge_cases.rs`
- `runner/execution/tests/plugin_trait_tests.rs` (736) → split by trait method
- `runner/execution/tests/stream.rs` (708) → split by stream type

### 7. Add test coverage for highest-value untested modules
- `cli/machine/queue_docs.rs` (494 LOC, 0 tests) — machine document generation
- `commands/scan.rs` (454 LOC, 0 tests) — scan workflow orchestration
- `commands/watch/processor.rs` (438 LOC, 0 tests) — watch event processing

### 8. Clone audit for runner/queue hot paths
- Identify unnecessary `String`/`Vec` clones in streaming and queue loading
- Consider `Cow<str>` or borrowing where lifetimes permit

### 9. Proactive decomposition of files in 400–500 LOC range
- `cli/scan.rs`, `cli/machine/task.rs`, `commands/init/writers.rs`, and 28 others
- Split before they breach the hard limit

---

## Sequencing rules

- Keep completed work out of this file.
- Prefer one canonical operator path over wrappers, aliases, or repeated prose.
- Prefer deleting dead wrappers before introducing new cleanup items in the same area.
- Preserve the hardened runtime split boundaries (`runutil/execution`, `runutil/retry`, `runutil/shell`, queue prune, fsutil, eta_calculator, undo, and `contracts/task`) while refactoring adjacent modules.
