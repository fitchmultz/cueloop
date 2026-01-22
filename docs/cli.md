# CLI Reference

Purpose: Summarize Ralph commands, flags, and customization points with examples for common workflows.

## Global Flags
- `--force`: force operations (e.g., bypass stale queue locks).
- `-v`, `--verbose`: increase output verbosity.

Examples:
```bash
ralph --verbose queue list
ralph --force queue archive
```

## Core Commands

* `ralph init`: bootstrap `.ralph/queue.json`, `.ralph/done.json`, and `.ralph/config.json`.
* `ralph queue <subcommand>`: validate, list, search, and batch-maintain tasks.
* `ralph run <subcommand>`: run tasks via a runner (codex/opencode/gemini/claude).
* `ralph tui`: launch the interactive UI (queue + execution + loop).
* `ralph task`: create a task from a request.
* `ralph scan`: generate new tasks via scanning.
* `ralph doctor`: verify environment readiness.

## `ralph tui`

Launch the interactive TUI. This is the primary user-facing entry point.

Behavior:

* Execution is enabled by default (press Enter to run selected task).
* Use `--read-only` to disable execution.
* Loop mode is available inside the TUI (press `l` to toggle).
* Archive done/rejected tasks inside the TUI (press `a`, then confirm).
* Use `:` to open the command palette for discoverability.
* The footer shows status messages and errors as actions run.

Examples:

```bash
ralph tui
ralph tui --read-only
ralph tui --runner codex --model gpt-5.2-codex --effort high
```

## `ralph run`

### Subcommands

* `one`: run exactly one task (optionally by ID or via interactive TUI).
* `loop`: run tasks until none remain (or `--max-tasks` reached).

### Interactive flags

* `ralph run one -i` launches the same TUI as `ralph tui`.
* `ralph run loop -i` launches the same TUI and auto-starts loop mode.

Examples:

```bash
ralph run one
ralph run one -i
ralph run loop --max-tasks 0
ralph run loop -i --max-tasks 3
```

## Runner and Model Overrides

These flags are supported on `task`, `scan`, `run one`, `run loop`, and `tui`:

* `--runner <codex|opencode|gemini|claude>`
* `--model <model-id>`
* `--effort <minimal|low|medium|high>` (codex only)
* `--rp-on` / `--rp-off`

Examples:

```bash
ralph tui --runner claude --model opus
ralph run one --runner codex --model gpt-5.2-codex --effort high
```

## Help Output

For the full, authoritative list of flags and examples, run:

```bash
ralph --help
ralph tui --help
ralph queue --help
ralph run --help
```
