# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- CueLoop is now the primary CLI/crate/app identity, with matching `cueloop` binary/package metadata, public docs, help examples, shell helpers, URL scheme, macOS operator copy, bundled app CLI preference, environment aliases, and final cutover checks.
- Task decomposition gained first-class plan-file input, source-plan provenance, group/draft semantics, activation modes, first-runnable-leaf reporting, exact copy-paste continuations, and calmer guidance for all-draft decomposition results.
- Queue follow-up proposal application can run during final Phase 3 completion, and task follow-up queues now support broader roadmap materialization flows.
- Repo trust and ignored-file synchronization gained a trusted allowlist for files beneath ignored directories, with explicit docs/init UX for the real allowlist contract.
- macOS app parity tracking now uses scenario-level proof entries for machine contracts, native surfaces, and Rust/CueLoopMac regression anchors.
- Release and development workflows gained Rust 1.95 verification gates, dependency/security posture refreshes, local file-size guardrails, repeatable dogfood harness support, and faster local CI waits.

### Changed

- The Rust baseline moved to Rust `1.95.0`, with crate `rust-version` aligned to `1.95` and dependencies refreshed.
- The Cursor runner was cut over to the SDK bridge, and Pi reasoning-effort support, runner buffer noise, and runner/session diagnostics were tightened.
- Parallel queue/done synchronization now handles gitignored JSONC queues, ignored-file sync, coordinator branch refreshes, local clone races, and push/rebase failure reporting more reliably.
- Machine run, machine error, recovery, queue validation, workspace overview, and runner-session flows now prefer versioned/structured state over stderr heuristics and stale fallback parsing.
- Webhook runtime behavior was hardened around startup gating, shutdown retry cancellation, persisted failure redaction, and repository context propagation.
- macOS retry, watcher, health, execution-control, workspace routing, and operator-copy behavior were aligned with actual CLI semantics.
- Documentation was split and refreshed across configuration, task, plugin, public-readiness, generated AGENTS templates, and current CueLoop naming.

### Fixed

- Pi-backed `cueloop scan` and `cueloop run loop` invocations now exit cleanly after the Pi runner returns: CueLoop's Pi wrapper awaits Pi `main(...)` and exits, while runner cleanup terminates lingering process-group descendants before joining stdout/stderr reader threads.
- Machine run loops now guarantee terminal summaries, select next tasks correctly, and fail fast instead of drifting after terminal runner/session states.
- Runner and recovery paths now validate session IDs before persistence, quarantine corrupt session caches, classify missing CLI binaries as `cli_unavailable`, keep machine-mode recovery JSON-only, and defer retry warnings until recovery is exhausted.
- Trust allowlist matching no longer lets zero-match entries block unrelated machines, and ignored-file sync rejects repo-escaping symlinks.
- Decompose preview continuations, queue selection messaging, CI gate wording, and validation output were made more accurate for normal operator workflows.
- Old runtime directory migration, cutover gate policy, macOS stale alias paths, and generated runtime guidance were cleaned up.

### Removed

- Legacy Ralph/RalphMac naming, filenames, helper symbols, docs, compatibility messaging, and self-compat fallback paths were removed from active surfaces.
- Obsolete debug logging, outdated critical notes, stale macOS aliases, and outdated CI/runtime docs were cleaned out.

### Security

- Ignored-file sync now blocks repo-escaping symlinks and requires explicit trust for allowlisted ignored paths.
- Webhook failure payloads are redacted before persistence, and webhook runtime startup is gated.
- Untrusted execution settings are classified as configuration errors, machine-mode recovery output stays JSON-only, and public-readiness/release gates continue to reject local artifacts, obvious secrets, and unsafe release states.

## [0.4.0] - 2026-04-23

### Added

- CueLoopMac and machine integrations now cover more of the CLI contract, including workspace overview, parallel run-control support, machine error documents, and fail-fast version checks.
- Webhook delivery gained configurable retry backoff, retry counts in app/config surfaces, safer diagnostics, replay hardening, and reloadable runtime behavior.
- Watch mode can emit desktop notifications, with matching CLI, configuration, app, and documentation support.
- Repository trust setup is easier to bootstrap with CLI-supported `.cueloop/trust.jsonc` flows and clearer built-in profile safety summaries.

### Changed

- Parallel worker integration now lets CueLoop rebuild queue/done bookkeeping from the latest target branch, archive the finished task, retry push races, and refresh the coordinator branch after worker success.
- Run, doctor, queue repair, task mutation, and recovery surfaces now share clearer blocking/resume-state narration across CLI, machine output, and CueLoopMac.
- Managed subprocess, wait, runner invocation, queue repair, webhook runtime, release, and macOS test code paths were split into smaller focused modules for more predictable behavior and maintenance.
- Release verification now preserves curated `Unreleased` changelog notes when they are already present, while still auto-generating entries for blank release notes.

### Fixed

- Sequential run loops fail fast instead of drifting after terminal runner/session states, and run-one keeps its queue lock alive through execution.
- Runner stream handling is more robust for Cursor/Gemini-style assistant deltas, Pi stream detail visibility, invalid resume fallbacks, and UTF-8 chunks split across fixed reads.
- CueLoopMac startup, workspace launch, permission prompts, run-control lock recovery, config persistence, and settings/window routing were hardened.
- Webhook failure storage avoids cross-process lost updates, and retry scheduling no longer blocks hot delivery workers.
- macOS CI and release bundling are faster and more deterministic.

### Security

- Webhook URL validation rejects unsafe destinations by default, and public-readiness checks redact secret findings before reporting.
- CI gate migration refuses lossy shell-string conversions, project-local execution remains trust-gated, and instruction-file path entries are validated.

## [0.3.1] - 2026-04-06

### Fixed

- Release automation now treats missing GitHub releases as `missing` without leaking JSON parser tracebacks during draft/publish state probing.

## [0.3.0] - 2026-03-24

### Added

- Shared machine-contract coverage for queue, run, doctor, and task recovery flows, plus the generated `machine.schema.json`, so app and automation clients can integrate against one versioned JSON surface.
- Explicit operator blocking/resume-state modeling across CLI, machine output, and CueLoopMac so stalled, waiting, and recovery states are narrated consistently.
- Durable watch-task identity metadata, atomic task mutation JSON flows, and safer queue repair/undo paths for structured recovery work.

### Changed

- **Breaking (`0.3`)**: CueLoop now requires the `0.3` config contract: config files must use `"version": 2`, `agent.git_publish_mode`, and the reserved built-in profiles `safe` / `power-user`; legacy `git_commit_push_enabled`, `quick`, and `thorough` flows are no longer the active contract. Run `cueloop migrate --apply` after upgrading older repos.
- Config, queue, and done workflows now center on the JSONC/runtime-cutover model, with clearer validation/migration messaging and no legacy JSON fallback guidance.
- `make release-verify` now prepares and records a publish-ready local snapshot under `target/release-verifications/`, and `make release` publishes only if that exact snapshot still matches `HEAD`, release metadata, release notes, and artifacts.
- Public-readiness scans, release artifact packaging, and CLI/app bundling now run through one hardened local release pipeline.
- CueLoopMac queue refresh, window routing, settings smoke coverage, and run-control status handling were tightened so the shipped app behavior stays aligned with the CLI/machine contract.

### Security

- Repo-local CI gates, runner overrides, and project plugins are now trust-gated through local `.cueloop/trust.jsonc`, and CI gate shell-string launchers are rejected.

## [0.2.2] - 2026-03-08

### Added

- Durable watch-task identity metadata and reconciliation rules so scan/remove flows only mutate the files processed in the current batch.
- Atomic task mutation support for the macOS app through `cueloop task mutate`, including optimistic locking and status-derived field updates in a single transaction path.
- Repo execution trust controls for project-local CI gate, runner override, and plugin execution settings.

### Changed

- Release automation now uses an explicit transaction workflow with `scripts/release.sh verify`, `execute`, and `reconcile`, transaction state under `target/release-transactions/`, and local-finalization-before-publication semantics.
- Public-readiness checks now scan the whole repository for markdown-link breakage, tracked runtime artifacts, tracked env files, and obvious secret material instead of relying on a hardcoded document subset.
- Agent CI routing now follows dependency surface changes instead of `apps/CueLoopMac/` path prefixes, escalating shared CLI/build/runtime contract changes to `macos-ci`.
- The macOS app, Makefile, and release artifact builder now share one CLI bundling/build entrypoint to keep app-bundled and shipped binaries on the same toolchain contract.
- Queue loading, managed subprocess execution, runner/runtime modules, and macOS app window/task presentation flows were refactored into smaller focused components for more predictable behavior and recovery.

### Security

- CI gate execution now rejects shell-string launchers and untrusted repo-local execution settings, and webhook failure diagnostics store only redacted destinations.

## [0.2.1] - 2026-03-06

## [0.2.0] - 2026-03-06

### Added

- macOS SwiftUI app (`apps/CueLoopMac/`) that drives CueLoop by executing the bundled `cueloop` CLI.
- `cueloop app open` (macOS-only) to launch the installed app (bundle id: `com.mitchfultz.cueloop`).
- Hidden GUI/tooling contract: `cueloop __cli-spec --format json` (emitted from clap's command model).
- `cueloop task decompose` to recursively plan task trees from a freeform goal or an existing queue task, preview the hierarchy, and write durable child tasks back into the queue.
- Dedicated decomposition prompt plumbing, queue-safe subtree materialization, optional sibling dependency inference, attach/replace child policies, and machine-readable preview/write output for automation.
- Full macOS app parity for task decomposition, including dedicated UI flows, toolbar/menu entry points, preview/write behavior, and regression coverage.

### Removed

- Rust terminal UI (`cueloop tui`) and interactive `-i/--interactive` entrypoints.
- TUI-only dependencies (`ratatui`, `crossterm`, and related crates).

## [0.1.0] - 2026-01-27

### Added

- Initial release of CueLoop, a Rust CLI for managing AI agent loops with a structured JSON task queue.
- Queue management: JSON-based task queue (`.cueloop/queue.json`) with priority, status, and dependency tracking.
- Task lifecycle: Create, update, complete, reject, and archive tasks with automatic timestamp tracking.
- Multi-phase workflow: Configurable 1, 2, or 3-phase execution (planning → implementation → review).
- Runner integration: Support for Codex, OpenCode, Gemini, Claude, and Cursor CLIs.
- TUI interface: Interactive terminal UI for queue inspection and task management.
- Prompt system: Embedded prompt templates with per-repo override support.
- Configuration: Layered JSON config (global + project) with schema validation.
- RepoPrompt integration: Optional planning and tooling injection for RepoPrompt-enabled environments.
- Git integration: Automatic commit/push on task completion with configurable behavior.
- CI gate: Built-in `make macos-ci` validation pipeline (format, lint, type-check, test, build, install).
- Queue validation: Schema validation for queue and config files.
- Task scanning: Automatic task generation from codebase analysis.
- Lock management: File-based locking with stale lock detection and force options.

### Security

- Secure credential handling: Secrets redaction in logs and queue entries.
- Lock file isolation: Prevents concurrent queue modifications.

[Unreleased]: https://github.com/fitchmultz/cueloop/compare/v0.4.0...HEAD
[0.4.0]: https://github.com/fitchmultz/cueloop/compare/v0.3.1...v0.4.0
[0.3.1]: https://github.com/fitchmultz/cueloop/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/fitchmultz/cueloop/compare/v0.2.2...v0.3.0
[0.2.2]: https://github.com/fitchmultz/cueloop/compare/v0.2.1...v0.2.2
[0.2.1]: https://github.com/fitchmultz/cueloop/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/fitchmultz/cueloop/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/fitchmultz/cueloop/releases/tag/v0.1.0
