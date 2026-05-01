<!-- CUELOOP_README_VERSION: 9 -->
# CueLoop runtime files

This repo is using CueLoop. The `ralph` executable is still the command name for this phase. This project stores runtime state in `.cueloop/`. New repos default to `.cueloop/`; legacy repos that already use `.ralph/` remain supported.

> This file is generated and owned by CueLoop. `ralph init` and agent-facing write-enabled commands may refresh it when CueLoop ships a newer template. Avoid hand-editing it unless you intentionally accept that local drift may be replaced.

## Files

- `.cueloop/config.jsonc` — repo-local configuration.
- `.cueloop/queue.jsonc` — JSONC task queue; source of truth for active work.
- `.cueloop/done.jsonc` — JSONC archive for completed tasks; only `done`/`rejected` statuses are valid.
- `.cueloop/cache/` — runtime cache for plans, completions, sessions, and temporary state.
- `.cueloop/logs/` — debug logs; should stay gitignored.
- `.cueloop/trust.jsonc` — machine-local trust decision; should stay gitignored.

Legacy `.ralph/` runtime directories are read in place. Do not rename `.ralph/` manually; use `ralph migrate runtime-dir --check` to preview and `ralph migrate runtime-dir --apply` to move project state to `.cueloop/` when ready.

## Core commands

### Bootstrap and health

- Bootstrap repo files:
  - `ralph init`
- Check this generated README:
  - `ralph init --check`
- Verify environment readiness:
  - `ralph doctor`
- Validate queue state:
  - `ralph queue validate`

### Queue management

- Inspect queue:
  - `ralph queue list`
  - `ralph queue next --with-title`
- Get next task ID:
  - `ralph queue next-id`
  - `ralph queue next-id --count 7`
- Show task details:
  - `ralph queue show RQ-0001`
- Archive completed tasks:
  - `ralph queue archive`
- Repair queue issues:
  - `ralph queue repair`
- Remove queue lock:
  - `ralph queue unlock`
- Sort and search tasks:
  - `ralph queue sort`
  - `ralph queue search "authentication"`
  - `ralph queue search "TODO" --status todo`
- Queue reports:
  - `ralph queue stats`
  - `ralph queue history --days 14`
  - `ralph queue burndown --days 30`
  - `ralph queue prune --age 90 --keep-last 100`

### Task creation and updates

- Build a task from a request:
  - `ralph task "Add tests for X"`
- Update task fields from repo state:
  - `ralph task update RQ-0001`
  - `ralph task update`
- Edit task fields:
  - `ralph task edit title "New title" RQ-0001`
  - `ralph task edit tags "rust, cli" RQ-0001`
- Change task status:
  - `ralph task status doing RQ-0001`
- Show task details:
  - `ralph task show RQ-0001`

### Execution

- Open the macOS app (macOS-only):
  - `ralph app open`
- Run one task:
  - `ralph run one`
  - `ralph run one --phases 3`
  - `ralph run one --quick`
  - `ralph run one --include-draft`
- Run multiple tasks:
  - `ralph run loop --max-tasks 0`
  - `ralph run loop --phases 2 --max-tasks 0`

### PRD, context, and scans

- Convert PRD markdown to tasks:
  - `ralph prd create docs/prd/feature.md`
  - `ralph prd create docs/prd/feature.md --multi`
  - `ralph prd create docs/prd/feature.md --dry-run`
- Generate or update AGENTS.md:
  - `ralph context init`
  - `ralph context update --section troubleshooting`
  - `ralph context validate`
- Seed tasks from a scan:
  - `ralph scan --focus "CI gaps"`
  - `ralph scan --focus "risk audit" --runner claude --model sonnet`

## Troubleshooting

### Duplicate task ID error

If `ralph queue validate` reports a duplicate task ID, this usually means a new task was added without incrementing the ID. Do not delete tasks.

1. Run `ralph queue next-id` to get the next available ID.
2. Edit `.cueloop/queue.jsonc` and change the colliding task ID.
3. Re-run `ralph queue validate`.

Task IDs must be unique across both `queue.jsonc` and `done.jsonc`.

### Generating multiple task IDs

Use `--count` to generate IDs in one call:

```bash
ralph queue next-id --count 7
```

`next-id` does not reserve IDs. Assign the printed IDs to tasks and insert all tasks into `.cueloop/queue.jsonc` before running other queue commands.

## Template variables

Prompt templates support variable interpolation for environment variables and config values:

- `${VAR}` — expand environment variable, leaving the literal when unset.
- `${VAR:-default}` — expand with a default value when unset.
- `{{config.agent.runner}}` — current runner.
- `{{config.agent.model}}` — current model.
- `{{config.queue.file}}` — queue file path, for example `.cueloop/queue.jsonc`.
- `{{config.queue.done_file}}` — done archive path, for example `.cueloop/done.jsonc`.
- `{{config.queue.id_prefix}}` — task ID prefix, for example `RQ`.
- `{{config.queue.id_width}}` — task ID width, for example `4`.
- `{{config.project_type}}` — project type.

Escaping:

- `$${VAR}` — outputs literal `${VAR}`.
- `\${VAR}` — outputs literal `${VAR}`.

Standard placeholders like `{{USER_REQUEST}}` are still processed after variable expansion.

## Prompt overrides

Default prompts are embedded in the `ralph` binary. Custom prompt files should live in `.cueloop/prompts/`; when both exist, `.cueloop/prompts/` takes precedence over legacy `.ralph/prompts/`.

Useful commands:

- `ralph prompt worker --phase 1`
- `ralph prompt worker --phase 2`
- `ralph prompt worker --phase 3`
- `ralph prompt list`
- `ralph prompt show worker --raw`
- `ralph prompt diff worker`
- `ralph prompt export --all`
- `ralph prompt sync --dry-run`
- `ralph prompt sync`

## Runner configuration

CueLoop can use built-in runner IDs (`codex`, `opencode`, `gemini`, `claude`, `cursor`, `kimi`, `pi`) or plugin runner IDs.

One-off usage:

- `ralph task --runner opencode --model gpt-5.2 "Add tests for X"`
- `ralph scan --runner gemini --model gemini-3-flash-preview --focus "risk audit"`
- `ralph task --runner claude --model opus --repo-prompt plan "Add tests for X"`
- `ralph run one --phases 3`
- `ralph run one --phases 2`
- `ralph run one --quick`

Defaults via config:

```json
{
  "version": 2,
  "agent": {
    "runner": "claude",
    "model": "sonnet",
    "phases": 3,
    "iterations": 1,
    "ci_gate": {
      "enabled": true,
      "argv": ["make", "ci"]
    }
  }
}
```

## Three-phase workflow

CueLoop supports a 3-phase workflow by default:

1. **Phase 1 (Planning):** generate a detailed plan and cache it in `.cueloop/cache/plans/<TASK_ID>.md`.
2. **Phase 2 (Implementation + CI):** implement the plan and pass the configured CI gate.
3. **Phase 3 (Code Review + Completion):** review the diff, refine if needed, rerun the CI gate, and complete the task.

Use `ralph run one --phases 3` for the full workflow. Use `--quick` as shorthand for `--phases 1`.

## Security: safeguard dumps and redaction

When runner operations fail, CueLoop writes safeguard dumps to temp directories for troubleshooting. By default, dumps are redacted before writing.

Raw, non-redacted dumps require explicit opt-in:

```bash
RALPH_RAW_DUMP=1 ralph run one
ralph run one --debug
```

Security notes:

- Never commit safeguard dumps.
- Debug mode writes raw runner output to `.cueloop/logs/debug.log`.
- Temp directories still use the legacy `/tmp/ralph/` root and `ralph_` prefixes until a later compatibility slice.

## Common flags

- `--quick` — shorthand for `--phases 1`.
- `--include-draft` — include draft tasks when selecting work.
- `--runner <codex|opencode|gemini|claude|cursor|kimi|pi>` — override runner.
- `--model <model-id>` — override model.
- `--repo-prompt <tools|plan|off>` / `-rp` — RepoPrompt mode.
- `--git-revert-mode <ask|enabled|disabled>` — control revert behavior on errors.
- `--git-commit-push-on` / `--git-commit-push-off` — toggle auto commit/push.
- `--debug` — capture raw output and imply raw dumps.
- `--force` — bypass locks or overwrite files where supported.
- `-v`, `--verbose` — increase output verbosity.
