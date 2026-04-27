# Configuration: Queue and Parallel
Status: Active
Owner: Maintainers
Source of truth: this document for `queue.*`, `parallel.*`, archive, aging, and parallel queue-path behavior
Parent: [Configuration](../configuration.md)

Purpose: Document Ralph queue storage and parallel-run configuration.

## Parallel Configuration

`parallel` controls parallel execution for `ralph run loop` and RalphMac Run Control loop launches.

Key fields:
- `workers`: number of concurrent workers (must be `>= 2`). Default: `null` (disabled unless CLI
  `--parallel` is used).
- `max_push_attempts`: maximum integration loop attempts before giving up (default: `50`).
- `push_backoff_ms`: array of retry backoff intervals in milliseconds (default: `[500, 2000, 5000, 10000]`).
- `workspace_retention_hours`: hours to retain worker workspaces after completion (default: `24`).
- `workspace_root`: root directory for parallel workspaces (default: `<repo-parent>/.workspaces/<repo-name>/parallel`).

  **Git hygiene warning:** If you set `parallel.workspace_root` to a path **inside** the repository (for example `.ralph/workspaces`), you MUST gitignore it (or add it to `.git/info/exclude`). Otherwise Ralph will create workspace clone directories that appear as untracked files and the repo will look "dirty" across runs. Parallel mode will fail fast if the workspace root is inside the repo and not ignored.

Notes:
- CLI flag `--parallel` overrides `parallel.workers` for a single run.
- Workers push directly to the target branch; no PRs are created.
- Use `ralph run parallel status` to check worker states.
- Use `ralph run parallel retry --task <ID>` to retry blocked workers.
- Migration-related breaking changes for retired parallel keys and `parallel.workspace_root` live in [Migration notes](migration-notes.md).

Example:

```json
{
  "parallel": {
    "workers": 3,
    "max_push_attempts": 50,
    "push_backoff_ms": [500, 2000, 5000, 10000],
    "workspace_retention_hours": 24
  }
}
```

## Queue Configuration
`queue` controls file locations, task ID formatting, and auto-archive behavior.

Supported fields:
- `file`: path to the queue file (default: `.ralph/queue.jsonc`).
- `done_file`: path to the done archive (default: `.ralph/done.jsonc`).
- `id_prefix`: task ID prefix (default: `RQ`).
- `id_width`: zero padding width (default: `4`, e.g. `RQ-0001`).
- `auto_archive_terminal_after_days`: automatically archive terminal tasks (done/rejected) from queue to done after this many days (default: `null`/`None`, disabled).

RalphMac and other machine clients resolve these settings through `ralph machine config resolve` or `ralph machine workspace overview`; the `.ralph/...` locations are defaults, not a separate app contract.

**Parallel mode restriction:** When running `ralph run loop --parallel ...`, `queue.file` and
`queue.done_file` must resolve to paths **under the repository root**. Parallel mode maps these
paths into per-worker workspace clones; paths outside the repo root cannot be mapped safely and are
rejected during parallel preflight. Prefer repo-relative paths like `.ralph/queue.jsonc` and
`.ralph/done.jsonc`.

### Auto-Archive Configuration

The `auto_archive_terminal_after_days` setting provides a queue-level sweep that archives terminal tasks (done/rejected) automatically:

- **Not set / `null`** (default): Disabled; no automatic sweep occurs.
- **`0`**: Archive immediately when the sweep runs (during macOS app startup/reload and after CLI task edit).
- **`N > 0`**: Archive only when `completed_at` is at least `N` days old.

**Safety behavior:** When `N > 0`, tasks with missing, blank, or invalid `completed_at` timestamps are **not moved**. This ensures only tasks with valid completion timestamps are archived automatically.

Example configurations:

```json
{
  "version": 2,
  "queue": {
    "file": ".ralph/queue.jsonc",
    "done_file": ".ralph/done.jsonc",
    "id_prefix": "RQ",
    "id_width": 4,
    "auto_archive_terminal_after_days": 7
  }
}
```

Immediate archive (archive all terminal tasks on sweep):
```json
{
  "queue": {
    "auto_archive_terminal_after_days": 0
  }
}
```

The queue-level sweep runs:
- When the macOS app starts or reloads queue files
- After `ralph task edit` operations (CLI)

For immediate manual archiving, use `ralph queue archive`.

### Aging Thresholds

`queue.aging_thresholds` controls the day thresholds for `ralph queue aging` task categorization.
This helps identify stale work by grouping tasks into buckets based on their age.

Supported fields:
- `warning_days`: warn when age is strictly greater than N days (default: `7`)
- `stale_days`: stale when age is strictly greater than N days (default: `14`)
- `rotten_days`: rotten when age is strictly greater than N days (default: `30`)

**Ordering invariant:** Config validation enforces `warning_days < stale_days < rotten_days`.

**Age computation by status:**
- `draft`, `todo`: uses `created_at` timestamp
- `doing`: uses `started_at` if present, otherwise `created_at`
- `done`, `rejected`: uses `completed_at` if present, then `updated_at`, then `created_at`

Tasks with missing/invalid timestamps or future timestamps are categorized as `unknown`.

Example configuration:
```json
{
  "version": 2,
  "queue": {
    "aging_thresholds": {
      "warning_days": 5,
      "stale_days": 10,
      "rotten_days": 20
    }
  }
}
```
