# Repository Guidelines

## Structure & Entry Points
- `ralph_legacy/`: Legacy scripts and prompt templates (see `ralph_legacy/legacy/`).
- `ralph_tui/`: Go-based CLI/TUI (`go run ./cmd/ralph`).
- Default pin/spec templates live in `ralph_legacy/specs/` and `.ralph/pin/`.

## Local Verification
- Use `make ci` as the local gate before considering work complete.

## Docs & Prompts
- Keep prompt templates and pin/spec fixtures generalized (no project-specific assumptions).
- Update path references when moving or renaming directories.
