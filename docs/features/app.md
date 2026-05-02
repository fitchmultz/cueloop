# macOS App

Purpose: document CueLoop's macOS SwiftUI app, user-facing workflows, and CLI parity expectations.
Status: Active
Owner: Maintainers
Source of truth: this document for macOS app user-facing workflows and parity expectations
Parent: [Feature Documentation](README.md)
Related: [Machine Contract](../machine-contract.md), [CI and Test Strategy](../guides/ci-strategy.md)

## Overview

CueLoop includes a macOS app for interactive queue and run supervision workflows.

The app is intended for:
- Browsing and editing the workspace's configured queue and done files
  (`.cueloop/queue.jsonc` and `.cueloop/done.jsonc` by default; legacy `.ralph/` repos remain supported)
- Triage and prioritization with a richer visual layout than terminal output
- Triggering common run operations while keeping CLI-compatible behavior
- Multi-window workflows across repositories and workstreams

The app does **not** replace the CLI for automation, CI, or scripted workflows.

## Open the App

From a repository initialized with `cueloop init`:

```bash
cueloop app open
```

If you are not on macOS (or prefer terminal workflows), use the CLI directly:

```bash
cueloop queue list
cueloop run one
cueloop run loop
```

Security note:
- `cueloop app open` deep links now carry only workspace context.
- The app ignores legacy `cli=` URL parameters and will not swap the CLI executable from URL input.

## Feature Tour

The app centers around workspace navigation and fast task handling:

- **Queue**: inspect tasks, status, priority, and dependency context
- **Quick Actions**: shortcuts for frequent task and run operations
- **Run Control**: launch and supervise execution flows, including machine-backed continuation and recovery actions
- **Advanced Runner**: diagnostic runner/model controls; not a parity-completion surface
- **Analytics**: high-level productivity and queue trend visibility
- **Graph View**: visualize dependency relationships
- **Command Palette**: keyboard-first command execution

## Keyboard Shortcuts

Documented from `apps/CueLoopMac/CueLoopMac/CueLoopMacApp.swift` command registrations.

### Navigation
- `⌘1`: Show Queue
- `⌘2`: Show Quick Actions
- `⌘3`: Show Run Control
- `⌘4`: Show Advanced Runner diagnostics
- `⌘5`: Show Analytics
- `⌃⌘S`: Toggle sidebar
- `⇧⌘K`: Toggle view mode
- `⇧⌘G`: Show graph view

### Task Actions
- `⌥⌘N`: New task
- `⌥⌘D`: Decompose task
- `⌘Return`: Start work on selected task

### Workspace / Window Management
- `⌘T`: New tab
- `⌘W`: Close tab
- `⇧⌘W`: Close window
- `⇧⌘]`: Next tab
- `⇧⌘[`: Previous tab
- `⌘D`: Duplicate tab

### Tools and Support
- `⌘K`: Quick command
- `⇧⌘P`: Command palette
- `⇧⌘L`: Export logs
- `⇧⌘R`: View crash reports
- `⌘,`: Settings

## Task Decomposition

The macOS app now exposes the same preview-first decomposition workflow as the CLI.

Use any of these entry points:
- Task menu: `Decompose Task...`
- Command palette: `Decompose Task...`
- Queue toolbar: `Decompose`
- Queue row context menu: `Decompose Task...`
- Menu bar: `Decompose Task...`

Behavioral notes:
- The sheet defaults to preview mode and only writes after an explicit second action.
- Launching from a selected task defaults to decomposing that task in place.
- Freeform mode can optionally attach a new subtree under an existing parent.
- The app calls `cueloop machine task decompose` for both preview and write flows and renders the versioned machine-contract response; it does not reimplement planner logic locally.

## How the App Integrates with the CLI

The app is a thin client that shells out to the primary `cueloop` binary via `RalphCLIClient` (`ralph` remains a fallback alias during migration).

Practical implications:
- Native workflows should use versioned `cueloop machine ...` JSON contracts,
  not human CLI text or older app-targeted CLI JSON surfaces.
- Scenario-level parity coverage lives in
  `crates/cueloop/src/cli/app_parity.rs`; every user-visible parity claim should
  point to explicit Rust and CueLoopMac proof anchors instead of broad command
  family labels alone.
- Task override and Run Control execution affordances should come from
  `cueloop machine config resolve.execution_controls`, not hardcoded native menus.
- Decomposition preview/write flows should stay wired to
  `cueloop machine task decompose` so the app consumes the same stable contract
  used by other machine clients.
- Trusted plugin runners appear in native controls through the same machine-fed
  contract; unknown configured runner or effort values must remain visible
  instead of being coerced away.
- Stop After Current specifically uses `cueloop machine run stop`; the app should
  never infer stop state by streaming or scraping human `cueloop queue stop`
  output.
- Run Control continuation cards should prefer structured native actions over
  terminal-only instructions when the machine contract exposes a safe preview or
  refresh path.
- Queue recovery remains preview-first in the app: validation, repair preview,
  restore preview, lock inspection, unlock preview, and shared-status refresh
  are native; unsupported continuations fall back to command copy or an explicit
  "not native yet" explanation.
- CLI and app should remain behaviorally aligned for core task/run operations.
- Advanced Runner access does not count as native app parity.
- Most data and execution issues can be reproduced via CLI commands.
- `cueloop doctor` remains the primary diagnostics entry point.

## Automated UI Testing

UI tests are headed and stay out of the default macOS CI path. Use [`docs/guides/ci-strategy.md`](../guides/ci-strategy.md) for validation cadence, shared-workstation caps, preserved `.xcresult` capture, and profiling.

Build/sign UI bundles once for an interactive debugging session:

```bash
make macos-ui-build-for-testing
```

Re-run UI tests without rebuilding bundles:

```bash
make macos-ui-retest
# Focus one test:
CUELOOP_UI_ONLY_TESTING=CueLoopMacUITests/CueLoopMacUILaunchAndTaskFlowTests/test_createNewTask_viaQuickCreate make macos-ui-retest
```

Run all UI tests end-to-end in one command:

```bash
make macos-test-ui
# macOS/Homebrew GNU Make users: gmake macos-test-ui
```

Run the focused window/tab shortcut regression suite:

```bash
make macos-test-window-shortcuts
```

Test sources live in `apps/CueLoopMac/CueLoopMacUITests/`.

## Troubleshooting

### App does not open
- Verify you are on macOS.
- Run `cueloop app open` from the repository root.

### Queue data not loading
- Run `cueloop machine config resolve` to inspect the machine-resolved queue, done, and config paths.
- Confirm the configured queue file exists at the resolved `queue_path`.
- Run `cueloop queue validate` and resolve schema errors.

### Runner commands fail
- Run `cueloop doctor` for environment diagnostics.
- Confirm configured runner CLIs are installed and authenticated.

### Need deterministic verification
- Validate behavior in terminal first (`cueloop queue ...`, `cueloop run ...`).
- Then verify equivalent flows in the app UI.

## Notes

- For complete command coverage and automation, use the CLI reference: `docs/cli.md`.
- For release-quality app validation, use [`docs/guides/ci-strategy.md`](../guides/ci-strategy.md).
