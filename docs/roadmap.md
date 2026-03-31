# Ralph Roadmap

Last updated: 2026-03-30

This is the canonical near-term roadmap for active follow-up work.

## Active roadmap

### 1. Replace phrase-based app recovery classification with shared machine error codes

**Why next**: The macOS app still classifies many CLI failures by English substring matching and falls back to showing raw descriptions for unknown errors. That creates drift with Rust-side classification and can surface noisy internal stderr directly in UI.

**Outcome**: Use one shared machine-readable recovery/error contract between CLI and app. Unknown failures should be sanitized and structured.

**Steps**:
- Audit current phrase-based classification in `apps/RalphMac/RalphCore/RalphCLIRecoveryClassifier.swift`, `RetryHelper.swift`, and `RalphCLIClient+Retry.swift`.
- Extend the CLI/machine contract with stable recovery/error codes and any needed payload fields.
- Cut the app over to structured decoding and remove duplicated phrase tables.
- Verify app recovery surfaces and `make agent-ci`.

**Exit criteria**:
- App recovery classification does not depend on free-form English stderr matching.
- Unknown recovery messages are structured/sanitized before display.

**Files in scope**: `apps/RalphMac/RalphCore/RalphCLIRecoveryClassifier.swift`, `apps/RalphMac/RalphCore/RetryHelper.swift`, `apps/RalphMac/RalphCore/RalphCLIClient+Retry.swift`, relevant machine-contract files under `crates/ralph/src/cli/machine/`.

---

### 2. Fix watch comment parsing so content extraction is stable across modes

**Why next**: `watch` comment detection currently uses capture-group position heuristics; `CommentType::All` produces different extracted content than specific modes, which then feeds watch task titles, notes, and identity/content hashes.

**Outcome**: Watch comment extraction becomes mode-independent and stable for task materialization and identity bookkeeping.

**Steps**:
- Replace positional capture extraction in `commands/watch/comments.rs` with a single canonical capture shape.
- Add regression coverage for `CommentType::All` vs specific-type parity, including identity/hash behavior.
- Verify watch task materialization output stays stable and `make agent-ci` remains green.

**Exit criteria**:
- Extracted comment content is identical regardless of watch comment mode.
- Watch identity/content hash coverage locks that behavior down.

**Files in scope**: `crates/ralph/src/commands/watch/comments.rs`, `crates/ralph/src/commands/watch/identity.rs`, `crates/ralph/src/commands/watch/tasks/materialize.rs`, watch tests.

---

### 3. Split runner orchestration hotspots in Phase 3 and core execution handling

**Why next**: `run_prompt_with_handling_backend` and `execute_phase3_review` remain high-complexity orchestration hubs with retry, revert, continue-session, CI, integration, and finalization logic intertwined. They are the highest-risk change surfaces in the runtime.

**Outcome**: Smaller focused helpers for timeout/non-zero/signal handling and Phase 3 final/non-final flows, with behavior preserved.

**Steps**:
- Extract timeout, non-zero-exit, and signal-recovery branches from `runutil/execution/orchestration/core.rs` into focused helpers.
- Split `commands/run/phases/phase3.rs` into prompt assembly, non-final review flow, finalization loop, and completion enforcement helpers.
- Keep facade/module-boundary rules intact and expand regression coverage only where behavior was previously implicit.
- Verify `make agent-ci`.

**Exit criteria**:
- Core orchestration functions drop materially in size/branch count.
- Runtime behavior stays covered by existing and targeted regression tests.

**Files in scope**: `crates/ralph/src/runutil/execution/orchestration/core.rs`, `crates/ralph/src/commands/run/phases/phase3.rs`, adjacent runtime tests.

---

### 4. Deduplicate macOS task-mutation encoding and clean up portable-path test debt

**Why next**: The app still hand-assembles task field edits with stringly-typed field names while some tests continue to hardcode `/tmp` paths despite portable temp helpers.

**Outcome**: Task mutation encoding becomes centrally defined, and test fixtures stop depending on Unix-only temp paths.

**Steps**:
- Introduce one shared field-to-edit encoder for `Workspace+TaskMutations.swift` flows.
- Add focused coverage for multi-field diff generation, not just agent overrides.
- Replace hardcoded `/tmp` test paths with temp-root helpers in affected Rust/Swift tests.
- Re-run the relevant local gate.

**Exit criteria**:
- Task mutation field encoding is not duplicated across single-field and bulk edit flows.
- Audited tests no longer require literal `/tmp` paths.

**Files in scope**: `apps/RalphMac/RalphCore/Workspace+TaskMutations.swift`, `apps/RalphMac/RalphCoreTests/ErrorRecoveryCategoryTests.swift`, `crates/ralph/src/commands/app/tests.rs`, related tests.

---

## Sequencing rules

- Keep completed work out of this file.
- Prefer one canonical operator path over wrappers, aliases, or repeated prose.
- Prefer deleting dead wrappers before introducing new cleanup items in the same area.
- Preserve the hardened runtime split boundaries (`runutil/execution`, `runutil/retry`, `runutil/shell`, queue prune, fsutil, eta_calculator, undo, and `contracts/task`) while refactoring adjacent modules.
