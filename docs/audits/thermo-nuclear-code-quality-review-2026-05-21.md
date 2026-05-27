# Thermo-Nuclear Code Quality Review — 2026-05-21
Status: Review snapshot
Owner: Maintainers
Source of truth: audit artifact only; active operating guidance remains in linked canonical docs
Parent: [CueLoop Documentation](../index.md)

**Date:** 2026-05-21
**Branch:** `review/thermo-nuclear-code-quality-20260521`
**Baseline:** `main` / `origin/main` at `851f4bf3`
**Scope:** General maintainability review of the current `main` codebase after confirming the new review branch had no feature diff.
**Auditor:** AI Agent using the thermo-nuclear code quality rubric
**Approval:** Withheld — structural blockers below should be addressed before treating the reviewed surfaces as clean.

---

## Executive summary

This was not a feature-diff review. The requested branch was created from `main`, and the diff against `origin/main` was empty, so the review treated the current codebase as the target.

No production source file crosses the 1,000 LOC blocker. The generated `schemas/machine.schema.json` is large by design. The main issue is not raw file size; it is contract and orchestration complexity that forces app, CLI, and runner changes to thread through stringly typed or duplicated paths.

### Top findings

| # | Severity | Area | Finding |
| --- | --- | --- | --- |
| 1 | Blocker | Machine contracts | `machine schema` is hand-maintained and already omits an active machine document. |
| 2 | Blocker | CLI/app boundary | Continuation actions are stringly typed, so the app reverse-engineers CLI commands. |
| 3 | Blocker | Machine errors | Stable app-facing error codes are derived from prose substring matching. |
| 4 | High | Queue runnability | Candidate semantics are duplicated outside the canonical runnability report path. |
| 5 | High | Run orchestration | Phase 2 carries a multi-axis orchestration matrix inline. |
| 6 | High | macOS app architecture | `CueLoopCore` persistence owns AppKit UI selection and retarget reset orchestration. |
| 7 | High | Runner plugin boundary | The shared runner plugin context leaks runner-specific options into every plugin. |

---

## Findings

### 1. Blocker — `machine schema` is stale because the machine contract registry is manual

**Evidence**

- `crates/cueloop/src/contracts/machine.rs:192` defines `MachineQueueUnlockInspectDocument`.
- `crates/cueloop/src/cli/machine/queue.rs:122` emits that document for `cueloop machine queue unlock-inspect`.
- `docs/machine-contract.md:279` documents `machine queue unlock-inspect` as a versioned machine command.
- `crates/cueloop/src/cli/machine/handle.rs:103-134` manually enumerates schema keys, but has no `queue_unlock_inspect` entry.
- `schemas/machine.schema.json` contains no `unlock-inspect`, `unlock_inspect`, `MachineQueueUnlockInspectDocument`, or `queue_unlock` key.

**Why this fails the rubric**

This is a source-of-truth split in a stable machine API. The command, contract type, and docs say the machine document exists, but the schema endpoint and committed schema artifact do not expose it. That is not a one-off omission; it is a symptom of a hand-built registry where each new machine document must be remembered in several places.

**Preferred remedy**

Create one machine-contract registry that owns `{command/doc key, version, schema type, docs anchor}`. Drive `machine schema`, committed schema generation, and contract coverage tests from that registry. Add a test that fails when a machine command document is emitted but missing from the schema registry.

---

### 2. Blocker — continuation actions are stringly typed across the CLI/app boundary

**Evidence**

- `crates/cueloop/src/contracts/machine.rs:119-123` defines `MachineContinuationAction` as only `title`, `command`, and `detail`.
- `docs/machine-contract.md:387-388` says continuation `next_steps` are the canonical input for native action mapping and copy-only fallbacks.
- `apps/CueLoopMac/CueLoopCore/Workspace+RunControlOperatorState.swift:421-493` switches on normalized command strings such as `machine queue validate`, `machine run stop`, and `machine run parallel-status` to decide native app actions.
- `apps/CueLoopMac/CueLoopCore/Workspace+RunControlOperatorState.swift:579-586` has duplicated identical `cueloop ` prefix handling, which is a small but telling symptom of brittle command-string plumbing.

**Why this fails the rubric**

The app is reverse-engineering CLI command text to discover behavior. That leaks terminal syntax into the native action boundary and makes flag ordering, spelling, and future aliases contract-significant. This also contradicts the app rule that machine JSON should be the app boundary, not human command parsing.

**Preferred remedy**

Extend `MachineContinuationAction` with a typed action identity, for example `action_id`, `kind`, `capability`, and optional structured args. Keep `command` only as display/copy metadata. Then collapse the Swift command-string classifier into a small typed dispatcher from action IDs to native app actions.

---

### 3. Blocker — stable machine error codes are built from prose substring matching

**Evidence**

- `crates/cueloop/src/cli/machine/error.rs:21-23` says machine errors must remain stable unless the machine contract version changes.
- `crates/cueloop/src/cli/machine/error.rs:34-36` lowercases a sanitized error string and classifies that text.
- `crates/cueloop/src/cli/machine/error.rs:38-117` assigns `MachineErrorCode` values based on English fragments such as `permission denied`, `queue validation failed`, `unknown field`, `minimum supported version`, `network`, and `timed out`.

**Why this fails the rubric**

The machine contract is supposed to be typed and stable, but the error boundary depends on incidental error prose from config, queue, IO, and dependency layers. A harmless wording cleanup can change app-facing error codes without any contract type or schema change.

**Preferred remedy**

Introduce typed domain errors or a conversion trait that carries `MachineErrorCode` at the config, queue, version, and process boundaries. Keep substring matching only as the final fallback for truly unknown errors. Add contract tests that construct typed errors directly instead of depending on prose fragments.

---

### 4. High — runnability candidate semantics are duplicated outside the canonical runnability path

**Evidence**

- `crates/cueloop/src/queue/operations/runnability/report.rs:249-253` has a private `is_candidate` helper.
- `crates/cueloop/src/cli/machine/common.rs:148-155` reimplements the same executable/Todo/Draft candidate check for validation-failure payloads.
- `crates/cueloop/src/cli/queue/explain.rs:159-165` reimplements it again for human explanation output.

**Why this fails the rubric**

Candidate semantics are queue-contract behavior. Duplicating them across machine and human presentation paths means a future change to Draft, Doing, group task, or executable-kind policy can drift silently.

**Preferred remedy**

Move the candidate decision into the canonical runnability model. Either expose a `TaskRunnabilityRow::is_candidate(options)` helper or include `candidate: bool` in each row. Machine fallback payloads and human explain output should consume that canonical fact instead of restating status/kind checks.

---

### 5. High — Phase 2 carries a multi-axis orchestration matrix inline

**Evidence**

- `crates/cueloop/src/commands/run/phases/phase2.rs:37-318` is a single `execute_phase2_implementation` function.
- The function branches on `total_phases == 3`, `ctx.is_final_iteration`, and `ctx.post_run_mode`.
- It repeats runner pass setup, `ContinueSession` construction, timing callbacks, cache updates, CI continuation, post-run supervision, and parallel integration handling in one flow.

**Why this fails the rubric**

This is spaghetti growth in a central execution path. Phase count, final-iteration policy, CI retry, resume session state, final-response caching, and parallel integration are all interleaved. The function has been decomposed by file location, but not by the concepts a maintainer must reason about.

**Preferred remedy**

Model Phase 2 as a small strategy/plan:

- `Phase2Mode::Handoff`
- `Phase2Mode::FinalSupervised`
- `Phase2Mode::FollowupCi`

Then share helpers for prompt building, runner-pass execution/caching, `ContinueSession` construction, and timing callbacks. The final branch should choose a mode; it should not contain the full orchestration matrix.

---

### 6. High — `CueLoopCore` persistence owns AppKit UI selection and retarget reset orchestration

**Evidence**

- `apps/CueLoopMac/CueLoopCore/Workspace+Persistence.swift:25` imports `AppKit` inside `CueLoopCore`.
- `apps/CueLoopMac/CueLoopCore/Workspace+Persistence.swift:416-426` constructs and runs `NSOpenPanel` directly.
- `apps/CueLoopMac/CueLoopCore/Workspace+Persistence.swift:516-557` resets runner, task, command, graph, analytics, output, diagnostics, cache, and health state from the persistence extension.

**Why this fails the rubric**

This is facade re-growth. A file scoped to persistence now owns app UI selection and cross-domain repository retargeting. That makes `CueLoopCore` less reusable, blurs app/core boundaries, and concentrates unrelated reset behavior in one extension.

**Preferred remedy**

Move folder selection UI into the `CueLoopMac` app layer or a small app service. Split retargeting into a dedicated workspace retarget coordinator, or push `resetForRetarget()` methods into each state owner/runtime so the persistence extension only changes identity and asks the coordinator to retarget.

---

### 7. High — runner plugin context leaks runner-specific options into every plugin

**Evidence**

- `crates/cueloop/src/runner/execution/plugin_trait.rs:21-24` uses `#![allow(dead_code)]` because trait/context fields are not used by every implementation.
- `crates/cueloop/src/runner/execution/plugin_trait.rs:47-65` puts `cursor`, `reasoning_effort`, `permission_mode`, and `phase_type` on every `RunContext`.
- `crates/cueloop/src/runner/execution/plugin_trait.rs:92-108` repeats those runner-specific fields on every `ResumeContext`.

**Why this fails the rubric**

The shared plugin trait is becoming a grab bag. Each runner-specific feature expands the global context and then every plugin must ignore the fields it does not understand. The dead-code allow is not the problem; it is evidence that the boundary is too broad.

**Preferred remedy**

Split shared execution context from runner-specific config. Use a typed runner-config enum, per-plugin adapter, or plugin-specific options object so Cursor/Claude/Pi-only fields do not leak into every plugin implementation.

---

## Additional pressure points

These are worth fixing after the blockers above, but they are lower priority than the contract and boundary issues.

- `crates/cueloop/src/commands/run/run_loop/orchestration.rs:150-318` duplicates `WaitExit` handling across `NoCandidates` and `Blocked`. Extract one wait-transition helper before adding more stop, timeout, or notification semantics.
- `apps/CueLoopMac/CueLoopCore/ConfigModels.swift:679-717`, `apps/CueLoopMac/CueLoopMac/SettingsViewModel.swift:237-247`, and `apps/CueLoopMac/CueLoopMac/TaskExecutionOverrideSupport.swift:25-26` duplicate runner/model/effort fallback knowledge that should flow from `MachineExecutionControls` through one presenter/catalog.
- `crates/cueloop/src/commands/doctor/runner.rs:25-354` mixes runner binary lookup, untrusted project override policy, Cursor SDK checks, API-key checks, model compatibility, and instruction-file status. Split provider-specific readiness checks before adding more runner-specific doctor policy.

---

## Recommended remediation order

1. Create the machine-contract registry and fix the missing `queue_unlock_inspect` schema entry.
2. Add typed continuation action IDs/kinds and migrate app native-action mapping off command-string parsing.
3. Replace machine-error prose matching with typed domain error conversion for known error classes.
4. Centralize runnability candidate semantics and delete the duplicated predicates.
5. Refactor Phase 2 around explicit modes and shared execution/session helpers.
6. Move AppKit folder selection out of `CueLoopCore` and split repository retarget reset ownership.
7. Narrow the runner plugin context boundary.

---

## Validation notes

This review is a documentation artifact and does not change runtime behavior.

Validation run after writing the report and index link:

```bash
make ci-docs
```

Result: passed. The docs-only gate completed pre-public checks, file-size checks, and markdown link/session-cache path checks.
