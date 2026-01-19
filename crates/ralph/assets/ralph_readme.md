# Ralph (Rust rewrite) runtime files

This repo is using Ralph. The implementation uses the `.ralph/` directory for repo-local state.

## Files

- `.ralph/queue.yaml` — YAML task queue (source of truth for active work).
- `.ralph/done.yaml` — YAML archive of completed tasks (same schema as queue).
- `.ralph/prompts/` — optional prompt overrides (defaults are embedded in the Rust CLI).

## Minimal Rust Commands

- Validate queue:
  - `ralph queue validate`
- Bootstrap repo files (queue + done + config):
  - `ralph init`
- Inspect queue:
  - `ralph queue list`
  - `ralph queue next --with-title`
- Next task ID:
  - `ralph queue next-id`
- Archive completed tasks:
  - `ralph queue done`
- Build a task from a request:
  - `ralph task build "<request>"`
- Seed tasks from a scan:
  - `ralph scan --focus "<focus>"`
- Run one task:
  - `ralph run one`
- Run multiple tasks:
  - `ralph run loop --max-tasks 0`
