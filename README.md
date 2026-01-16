# Ralph

Ralph is a tool for managing AI agent loops and pin operations.

## Project Structure

This repository is split into two main components:

- **[ralph_legacy](./ralph_legacy)**: Frozen, maintenance-only scripts and legacy spec templates. No new features land here; only critical fixes.
- **[ralph_tui](./ralph_tui)**: The active Go TUI/CLI. All new work and feature development targets this path.

## Getting Started

Refer to the individual READMEs in each directory for specific instructions on how to use each version. Default TUI pin files live under `.ralph/pin/`.

## Project Types

Ralph supports a configurable `project_type` to tune prompts and workflows:

- `code` (default): code-focused prompts.
- `docs`: documentation-focused prompts (doc maintenance, link checks, research synthesis).

Set it via `ralph init --project-type docs` or the config editor to persist in `.ralph/ralph.json`.
