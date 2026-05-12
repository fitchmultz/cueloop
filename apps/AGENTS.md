# AGENTS.md

<!-- AGENTS ONLY: app-scoped guidance. Repo-wide rules live in ../AGENTS.md. -->

## Purpose

This file applies to `apps/**`. CueLoopMac is a native SwiftUI companion app that stays a thin, user-friendly surface over the Rust `cueloop` CLI and versioned `cueloop machine ...` JSON contracts. Follow `../AGENTS.md` for repo-wide rules.

## Repository map

- `apps/CueLoopMac/CueLoopCore/` — CLI client, machine-contract models, workspace/domain state, queue watching, persistence, health, and app services.
- `apps/CueLoopMac/CueLoopMac/` — SwiftUI app shell, views, commands, settings, routing, and presentation state.
- `apps/CueLoopMac/CueLoopCoreTests/`, `CueLoopMacTests/`, `CueLoopMacUITests/` — non-UI, app, and headed UI test targets.
- `apps/CueLoopMac/CueLoopMac.xcodeproj/` — Xcode project. Avoid manual project churn unless target membership/settings really changed.
- `apps/CueLoopMac/CueLoopCLIInputs.xcfilelist` — committed input list for Xcode’s “Build and Bundle CueLoop” phase so Swift-only edits do not re-run the CLI bundle when Rust sources, embedded assets, manifests, or bundling scripts are unchanged; update it when those inputs change (release integration tests assert it stays aligned with `crates/cueloop`).
- `apps/CueLoopMac/build/`, `apps/CueLoopMac/target/`, and `target/tmp/xcode-deriveddata/` — ignored build artifacts.
- App/CLI parity registry: `../crates/cueloop/src/cli/app_parity.rs`.

## Operating rules

- Use versioned machine-contract JSON or shared JSON outputs. Do not parse human CLI output for app workflows.
- The Advanced Runner is diagnostic/debug tooling only; it does not make a CLI feature app-parity-complete.
- App-side task editing should use the canonical task mutation path, not field-by-field shellouts.
- When a CLI feature changes, update the machine contract, `CueLoopCore` decoding, SwiftUI surface, tests, and docs together. If blocked, record the explicit gap in `../crates/cueloop/src/cli/app_parity.rs`.
- Keep CLI parity while improving UX: guided inputs, visible state, previews where supported, progress, success/error states, recovery actions, keyboard access, and clear labels.
- Stop app-side exploration after you have identified the owning `Workspace`/CLI-client/view file, contract input/output, and validation path.

## Setup and commands

Run from the repository root; Make wraps Xcode with the required bundling, locks, derived-data policy, and deterministic smoke tests.

| Need | Command |
| --- | --- |
| Required app ship gate | `make macos-ci` |
| Routed local gate | `make agent-ci` |
| Build app | `make macos-build` |
| Build app (fresh Xcode DerivedData for this lane) | `make macos-build-clean` |
| Non-UI app tests | `make macos-test` |
| Non-UI app tests (fresh DerivedData first) | `make macos-test-clean` |
| Full macOS ship gate (fresh shared DerivedData first) | `make macos-ci-clean` |
| Build UI-test bundles once | `make macos-ui-build-for-testing` |
| Re-run UI tests | `make macos-ui-retest` |
| Capture headed UI artifacts | `make macos-test-ui-artifacts` |
| Clean UI artifacts | `make macos-ui-artifacts-clean` |
| Focused Settings contract | `make macos-test-settings-smoke` |
| Focused workspace-routing contract | `make macos-test-workspace-routing-contract` |
| Cache / DerivedData diagnostics | `make build-cache-doctor` |

`scripts/cueloop-cli-bundle.sh` is the single CLI bundling/build entrypoint for Makefile, Xcode, and release consumers. Do not add standalone Cargo fallback logic inside Xcode settings or app-specific scripts.

## Coding conventions

- Every Swift file starts with a purpose header covering responsibilities, non-scope, and invariants/assumptions.
- Keep access control minimal and explicit: `private` for implementation details, internal by default, `public` only for real `CueLoopCore` framework exports.
- Keep `CueLoopMacApp.swift` thin. Menu commands live in `CueLoopMacCommands.swift`, URL routing in `CueLoopMacApp+URLRouting.swift`, bootstrap helpers in `CueLoopMacApp+Support.swift`, window root composition in `WindowViewContainer.swift`, and UI-test window policy in `WorkspaceWindowAnchor.swift`.
- `Workspace.swift` is a `@MainActor` facade over domain owners and focused `Workspace+...` files. Do not re-accumulate runner, persistence, task mutation, recovery, or queue refresh logic in the root type.
- `CueLoopCLIClient.swift` owns the core subprocess API only. Retry, recovery classification, health probing, and lifecycle ownership live in companion files.
- `CueLoopModels.swift` is a facade only. Keep CLI spec models, generic JSON values, argument builders, and task-domain models in dedicated `CueLoopCore` files.
- Active-window navigation/task commands flow through focused scene values (`WorkspaceUIActions` / `WorkspaceWindowActions`). Unfocused menu, URL, and lifecycle surfaces route through `WorkspaceSceneRouter`.
- Do not reintroduce process-wide `NotificationCenter` broadcasts for focused workspace actions.
- Queue file watcher refreshes and CLI queue JSON decoding use `WorkspaceQueueSnapshotLoader`; publish only final state on the main actor.
- Operational visibility flows through `WorkspaceOperationalHealth`. Workspace identity persistence uses `WorkspaceStateStore`; persistence failures surface as `PersistenceIssue`.

## Validation and done criteria

App work is done when CLI/app behavior stays in parity, relevant UI/contract tests pass or have an explicit blocker, and user-facing state/errors are visible and recoverable.

- Use `CueLoopCoreTestSupport` for temp workspaces, readiness polling, and cleanup assertions.
- SwiftUI previews needing workspace URLs derive them from `PreviewWorkspaceSupport`.
- UI tests must not write into the production app defaults domain; `--uitesting` launches use the dedicated test suite.
- Normal UI-test launches should keep one visible workspace window; multiwindow tests should keep two. Avoid widths below the split-view practical minimum.
- UI screenshot capture is opt-in through `CUELOOP_UI_SCREENSHOTS=1` or `CUELOOP_UI_SCREENSHOT_MODE`.
- For visual UX changes, inspect the rendered app or preserve headed UI artifacts. If that cannot run, record why and what should be checked next.

## Planning and large changes

Use a short plan for changes that cross CLI machine contracts, `CueLoopCore`, SwiftUI, tests, and docs. Plan the contract first, then decoding/state ownership, then view behavior, then validation. Do not split parity work across hidden follow-ups unless the blocker is recorded in `app_parity.rs`.

## Security and side effects

- CueLoopMac may execute only the validated CueLoop CLI path selected by the app or user. Do not add alternate executable discovery paths, shell-string execution, or GUI-controlled arbitrary command launch surfaces.
- Workspace access is user-selected and workspace-scoped. Do not broaden file reads/writes outside the active workspace except for documented CueLoop config, cache, or app-support locations.
- The GUI does not make direct network calls for CueLoop workflows. Networked behavior must stay behind the CLI/machine contract or a documented app system service with explicit maintainer approval.
- Keep destructive or trust-changing operations visible and confirmed in the UI; do not add hidden bypasses around CLI trust, config, or queue safeguards.

## Progress updates and handoff

For app changes, progress updates should name the affected surface (`CueLoopCore`, SwiftUI view, CLI contract, tests). Handoffs must include UI/contract validation, any visual evidence path, and any app-parity gaps.

## Updating this file

Keep this file limited to app-specific rules. Move repo-wide commands or policy back to `../AGENTS.md`. Update this file when app architecture ownership, machine-contract policy, or app validation commands change.
