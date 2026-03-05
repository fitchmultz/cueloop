# Architecture Overview

Purpose: provide a reviewer-friendly view of Ralph’s core components, execution flow, and design trade-offs.

## System Boundary

Ralph is a local-first orchestration system for AI-assisted engineering workflows.

- **Primary runtime:** `ralph` Rust CLI (`crates/ralph/`)
- **Optional UI:** SwiftUI macOS app (`apps/RalphMac/`) that shells out to the same CLI binary
- **State store:** repo-local files in `.ralph/` (`queue.jsonc`, `done.jsonc`, `config.jsonc`)
- **External dependencies:** runner CLIs (Codex/Claude/Gemini/OpenCode/Cursor/Kimi/Pi), git, optional GitHub CLI for releases

## Core Components

### 1) Queue + Task Lifecycle

- Task state is explicit (`todo`, `doing`, `done`, `rejected`, etc.) and persisted to `.ralph/queue.jsonc` and `.ralph/done.jsonc`.
- Queue operations (validation, sorting, archive, search, graph/tree views) are implemented in `crates/ralph/src/queue/` and command modules.

### 2) Run Supervision Engine

- `crates/ralph/src/commands/run/` controls plan/implement/review phase orchestration.
- Supports single-task runs, looped runs, resume/recovery, and parallel worker workflows.
- Includes CI gating, retries, and failure handling to keep repository state coherent after runner execution.

### 3) Runner Integration Layer

- Runner-specific invocation settings are normalized through contracts/config + command wiring.
- Phase-level runner/model/effort overrides are supported for controlled execution behavior.

### 4) Safety and Reliability Layers

- Startup sanity checks (`crates/ralph/src/sanity/`)
- Locking and concurrency controls (`crates/ralph/src/lock.rs`)
- Validation and guardrails around queue/state transitions

### 5) macOS App Bridge

- App UI (SwiftUI) focuses on queue visibility and workflow ergonomics.
- `RalphCLIClient` bridges app actions to CLI commands, preserving behavior parity and reducing duplicate logic.

## Data and Control Flow

A typical execution loop:

1. User creates/selects tasks (`ralph task ...`, queue commands, or app UI).
2. Run command resolves config + phase settings.
3. Supervision executes one or more phases via selected runner(s).
4. Outputs, queue transitions, and optional CI gate checks are applied.
5. Completion/failure state is persisted to `.ralph/queue.jsonc` / `.ralph/done.jsonc`.

Parallel mode adds workspace isolation per worker to reduce branch/queue contention.

## Key Design Decisions and Trade-Offs

### Local-first JSONC state

**Decision:** keep queue/config in repo-local JSONC files.

- **Pros:** transparent reviewability, diffable history, easy backup/recovery.
- **Trade-off:** requires strict validation and repair logic to avoid malformed state drift.

### Multi-phase supervised execution

**Decision:** support 1/2/3-phase workflows (single-pass vs plan+implement vs plan+implement+review).

- **Pros:** gives users control over quality/speed balance.
- **Trade-off:** more orchestration complexity and more edge cases in resume/retry.

### Thin macOS app over CLI parity

**Decision:** keep app as a thin client over CLI rather than separate business logic stack.

- **Pros:** one behavior source of truth, easier testing, lower drift risk.
- **Trade-off:** UX latency and error handling depend on robust CLI bridge behavior.

### Local-CI-first release workflow

**Decision:** run quality gates through Makefile/script tooling instead of remote CI pipelines.

- **Pros:** deterministic local verification and easier contributor reproducibility.
- **Trade-off:** requires stronger docs/scripts to keep onboarding and release flows frictionless.

## Operational Expectations

- Use `make agent-ci` for routine PR-equivalent checks.
- Use `make ci` for full Rust release gate.
- Use `make macos-ci` when app changes are in scope.
- Use `make pre-public-check` before public release windows.
