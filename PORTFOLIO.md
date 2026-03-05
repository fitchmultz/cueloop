# Ralph Portfolio Guide

Purpose: give human reviewers a fast, high-signal tour of the repository.

## If You Only Read 3 Things

1. [README.md](README.md) — product overview, quickstart, and docs map.
2. [docs/architecture.md](docs/architecture.md) — component boundaries, data/control flow, trade-offs.
3. [apps/RalphMac/RalphCore/RalphCLIClient.swift](apps/RalphMac/RalphCore/RalphCLIClient.swift) — macOS app ↔ CLI bridge.

## 2-Minute Architecture Tour

- **Core runtime**: Rust CLI in `crates/ralph/`.
- **State model**: structured repo-local queue files under `.ralph/`.
- **Execution**: runner-specific subprocess adapters (Codex, Claude, Gemini, OpenCode, Cursor, Kimi, Pi).
- **UI**: SwiftUI macOS app in `apps/RalphMac/` that shells out to the same CLI for parity.

## Quality and Verification Signals

- Fast Rust/CLI gate: `make ci-fast`
- Local CI gate (path-aware): `make agent-ci`
- Ship gate (includes macOS): `make macos-ci`
- Resource-aware knobs for shared workstations: `RALPH_CI_JOBS` and `RALPH_XCODE_JOBS`
- API docs generation: `make docs`
- Pre-public audit automation: `make pre-public-check`
- Integration test suite: `crates/ralph/tests/`
- Snapshot tests: `crates/ralph/tests/snapshots/`
- Security policy: [SECURITY.md](SECURITY.md)

## Suggested Reviewer Walkthrough

```bash
# install from source (GNU Make >=4 required; macOS/Homebrew users should run gmake)
make install

# evaluate queue workflows without external runner setup
ralph init
ralph task "Create first review task"
ralph queue list
ralph queue graph
ralph queue validate
ralph doctor

# run the quality gate with explicit resource caps (optional)
RALPH_CI_JOBS=4 make agent-ci
```

## Where the Interesting Engineering Lives

- `crates/ralph/src/main.rs` — startup path, error handling, and command wiring.
- `crates/ralph/src/sanity/mod.rs` — preflight checks and guardrails.
- `crates/ralph/src/commands/run/` — supervision, phases, and recovery behavior.
- `apps/RalphMac/RalphMac/TaskListView.swift` — primary UI workflow.
- `apps/RalphMac/RalphCore/RalphCLIClient.swift` — command execution and output normalization.

## Public-Readiness Checklist

See [docs/guides/public-readiness.md](docs/guides/public-readiness.md) for a repeatable pre-publication audit.
