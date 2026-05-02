# Configuration: Queue and Parallel
Status: Active
Owner: Maintainers
Source of truth: this document for `queue.*`, `parallel.*`, archive, aging, and parallel queue-path behavior
Parent: [Configuration](../configuration.md)

Purpose: Document CueLoop queue storage and parallel-run configuration.

## Parallel Configuration

`parallel` controls parallel execution for `cueloop run loop` and the current macOS Run Control loop launches.

Key fields:
- `workers`: number of concurrent workers (must be `>= 2`). Default: `null` (disabled unless CLI
  `--parallel` is used).
- `max_push_attempts`: maximum integration loop attempts before giving up (default: `50`).
- `push_backoff_ms`: array of retry backoff intervals in milliseconds (default: `[500, 2000, 5000, 10000]`).
- `workspace_retention_hours`: hours to retain worker workspaces after completion (default: `24`).
- `workspace_root`: root directory for parallel workspaces (default: `<repo-parent>/.workspaces/<repo-name>/parallel`).
- `ignored_file_allowlist`: optional trusted repo-relative file/glob allowlist for additional small gitignored local files to copy into worker workspaces. Default: `null` (`.env` / `.env.*` only). Project config that sets this key is repo-trust-gated.

  **Git hygiene warning:** If you set `parallel.workspace_root` to a path **inside** the repository (for example `.cueloop/workspaces`, or legacy `.ralph/workspaces`), you MUST gitignore it (or add it to `.git/info/exclude`). Otherwise CueLoop will create workspace clone directories that appear as untracked files and the repo will look "dirty" across runs. Parallel mode will fail fast if the workspace root is inside the repo and not ignored.

Notes:
- CLI flag `--parallel` overrides `parallel.workers` for a single run.
- Workers push directly to the target branch; no PRs are created.
- Use `cueloop run parallel status` to check worker states.
- Use `cueloop run parallel retry --task <ID>` to retry blocked workers.
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

### Ignored local file sync

Parallel worker workspaces receive tracked files through git and CueLoop runtime files under `.cueloop/` (or legacy `.ralph/`) through workspace seeding. By default, CueLoop also copies ignored `.env` and `.env.*` files so workers inherit common local environment files.

CueLoop does **not** copy all ignored files automatically. Broad ignored-file copying can duplicate heavy build/cache trees (`target/`, `node_modules/`, `.venv/`), stale generated artifacts, nested worker workspaces, or nondeterministic local state.

When a repository needs additional ignored local files for parallel workers, configure an explicit trusted allowlist. This allowlist is for small, file-shaped local inputs that workers need but git intentionally ignores: local tool config, private fixture JSON, or `*.local.toml` files. It is not a directory-tree sync feature. Do not use it for package caches, build outputs, virtual environments, worker workspaces, or broad ignored directories.

Valid examples:

```jsonc
{
  "parallel": {
    "ignored_file_allowlist": [
      "local/tool-config.json",
      "fixtures/local-*.json",
      "config/**/*.local.toml"
    ]
  }
}
```

Invalid examples:

| Entry | Why it is rejected |
|-------|--------------------|
| `node_modules/*` | under a denied heavy dependency tree |
| `target/**` | under a denied build-output tree |
| `dir/` | directory entries are unsupported |
| `/tmp/secret.json` | entries must be repo-relative |
| `../secret.json` | entries may not escape the repo |
| `.cueloop/cache/*` | runtime/cache paths are denied |
| `.ralph/cache/*` | legacy runtime/cache paths are denied |

Rules:
- entries are repo-relative file paths or file glob patterns
- directories, absolute paths, `..`, and `.` path components are rejected
- denied runtime/build paths such as `target/`, `node_modules/`, `.venv/`, `.git/`, `.cueloop/{cache,workspaces,logs,lock}/`, and legacy `.ralph/{cache,workspaces,logs,lock}/` are rejected
- entries that match no existing gitignored files are treated as optional and skipped with a warning during parallel preflight
- invalid entries or entries that match unsafe paths still fail preflight, including directories, denied runtime/build paths, symlinks resolving outside the repo, and paths inside or overlapping the parallel workspace root
- project config that sets this allowlist requires repo trust (`cueloop init` creates trust during bootstrap; `cueloop config trust init` is available for trust-only repair)

## Queue Configuration
`queue` controls file locations, task ID formatting, and auto-archive behavior.

Supported fields:
- `file`: path to the queue file (default: `.cueloop/queue.jsonc`; legacy default fallback: `.ralph/queue.jsonc`).
- `done_file`: path to the done archive (default: `.cueloop/done.jsonc`; legacy default fallback: `.ralph/done.jsonc`).
- `id_prefix`: task ID prefix (default: `RQ`).
- `id_width`: zero padding width (default: `4`, e.g. `RQ-0001`).
- `auto_archive_terminal_after_days`: automatically archive terminal tasks (done/rejected) from queue to done after this many days (default: `null`/`None`, disabled).

Machine clients resolve these settings through `cueloop machine config resolve` or `cueloop machine workspace overview`; the `.cueloop/...` locations are defaults, and legacy `.ralph/...` locations remain compatibility fallback, not a separate app contract.

**Parallel mode restriction:** When running `cueloop run loop --parallel ...`, `queue.file` and
`queue.done_file` must resolve to paths **under the repository root**. Parallel mode maps these
paths into per-worker workspace clones; paths outside the repo root cannot be mapped safely and are
rejected during parallel preflight. Prefer repo-relative paths like `.cueloop/queue.jsonc` and
`.cueloop/done.jsonc`.

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
    "file": ".cueloop/queue.jsonc",
    "done_file": ".cueloop/done.jsonc",
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
- After `cueloop task edit` operations (CLI)

For immediate manual archiving, use `cueloop queue archive`.

### Aging Thresholds

`queue.aging_thresholds` controls the day thresholds for `cueloop queue aging` task categorization.
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
