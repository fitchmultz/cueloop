# Ralph (Rust rewrite) runtime files

This repo is undergoing a Rust rewrite of Ralph. The Rust implementation uses the
`.ralph/` directory for repo-local state.

## Files

- `.ralph/queue.yaml` — YAML task queue (source of truth for active work).
- `.ralph/prompts/` — optional prompt overrides used by the runner.

## Supervisor Workflow (Rust)

`ralph run one` (and `ralph run loop`) act as a lightweight supervisor around the execution agent.

Core behavior:
- Task order is priority: the first `todo` in `.ralph/queue.yaml` is selected.
- The supervisor does NOT set `doing`; the agent does.
- After the agent exits, the supervisor checks the repo state:
  - If the repo is clean and the task is `done`, it proceeds to the next task.
  - If the repo is dirty, it runs `make ci`. On green, it commits + pushes all changes.
  - If the task is not `done`, the supervisor sets `done`, runs `make ci`, and commits + pushes.
- `status: blocked` is not supported. If encountered, the supervisor reverts uncommitted changes
  (if any) and stops.

Common scenarios:
- Agent completes normally (done + CI + commit + push) -> supervisor sees clean repo and moves on.
- Agent leaves dirty repo -> supervisor runs CI, commits, pushes.
- Agent forgets to mark `done` -> supervisor sets `done`, runs CI, commits, pushes.

## Legacy (Go) Ralph

The existing Go-based implementation still uses:

- `.ralph/ralph.json`
- `.ralph/pin/`

Those files remain in the repo during migration but are not part of the Rust
queue contract.
