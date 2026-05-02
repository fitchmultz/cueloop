# CLI Reference
Status: Active
Owner: Maintainers
Source of truth: this document for its stated scope
Parent: [CueLoop Documentation](index.md)


This page documents CueLoop's current command surface. The executable is `cueloop`. Default `cueloop --help` shows the core workflow only; use `cueloop help-all` or `cueloop <command> --help` to reveal advanced and experimental surfaces.

## Global Flags

These are available on most commands:

- `--force`
- `-v, --verbose`
- `--debug` (supported on run flows; writes raw logs to `.cueloop/logs/debug.log`)
- `--color <auto|always|never>`
- `--no-color`
- `--auto-fix`
- `--no-sanity-checks`

## Core Commands

- `cueloop queue` - Inspect and manage queue/done files
- `cueloop config` - Show resolved config, schema, paths, profiles, repo trust (`config trust init`)
- `cueloop run` - Execute tasks (`one`, `loop`, `resume`, `parallel`)
- `cueloop task` - Build/create and manage task lifecycle
- `cueloop scan` - Create tasks by scanning repository state
- `cueloop init` - Bootstrap `.cueloop/` files for a repo
- `cueloop app` - macOS app integration
- `cueloop version` - Build/version info

## Advanced Commands

- `cueloop machine` - Versioned machine-facing JSON API for the macOS app and automation
- `cueloop prompt` - Render/export/sync/diff prompts
- `cueloop doctor` - Environment diagnostics
- `cueloop context` - Manage AGENTS.md context docs
- `cueloop daemon` - Background daemon controls
- `cueloop prd` - Convert PRD markdown into tasks
- `cueloop completions` - Generate shell completions
- `cueloop migrate` - Check/apply config migrations and explicit `runtime-dir` migration (`.ralph` → `.cueloop`)
- `cueloop cleanup` - Remove temporary runtime artifacts
- `cueloop version` - Build/version info
- `cueloop watch` - File watch to detect task comments
- `cueloop webhook` - Test/status/replay webhook deliveries
- `cueloop productivity` - Analytics summaries and trends
- `cueloop plugin` - Plugin discovery and lifecycle
- `cueloop runner` - Runner list and capabilities
- `cueloop tutorial` - Interactive onboarding walkthrough
- `cueloop undo` - Restore most recent queue snapshot

## Experimental Commands

- `cueloop run parallel` - Experimental direct-push parallel worker operations

## High-Value Workflows

### Initialize

```bash
cueloop init
cueloop init --non-interactive
```

`cueloop init` creates or updates local repository trust (`.cueloop/trust.jsonc`), refreshes the generated runtime README, and adds the trust file to `.gitignore`. Use `cueloop config trust init` only for trust-only repair in an already-initialized repo.

Interactive init can select extra ignored files for parallel workers; manual additions belong in trusted `parallel.ignored_file_allowlist` and follow the small-file allowlist contract in [Ignored local file sync](configuration/queue-and-parallel.md#ignored-local-file-sync).

### Create and Run

```bash
cueloop task "Stabilize flaky CI test"
cueloop run one --profile safe
cueloop run one --resume
cueloop run one --debug
cueloop run loop --max-tasks 0
cueloop run loop --max-tasks 5
cueloop run resume
```

`cueloop run loop --max-tasks 0` means unlimited execution. Use a positive `--max-tasks` value when you want a fixed cap on successful iterations.

### Resume-aware execution

CueLoop now explicitly narrates whether it is:
- resuming the same session
- falling back to a fresh invocation
- refusing to resume because confirmation is required

Useful commands:

```bash
# Inspect interrupted work and choose interactively when needed
cueloop run one

# Auto-resume when CueLoop can do so safely
cueloop run one --resume
cueloop run loop --resume --max-tasks 5
cueloop run resume

# Headless use: explicit policy only
cueloop run one --resume --non-interactive
cueloop run loop --non-interactive
```

### Blocked / waiting / stalled narration

When CueLoop cannot make progress, it now classifies the current state instead of only printing generic wait prose. Operator-facing run surfaces distinguish:

- true idle waiting (no todo work)
- dependency blocking
- schedule blocking
- queue lock contention
- CI-gate stalls
- runner/session recovery stalls

For automation, the same model is exposed through:

- `cueloop machine run ...` NDJSON `blocked_state_changed` / `blocked_state_cleared` events
- `cueloop machine run ...` terminal summaries via `blocking`
- `cueloop machine queue read` via `runnability.summary.blocking`

### Execution Shape

```bash
# Single-pass
cueloop run one --quick

# 2-phase
cueloop run one --phases 2

# 3-phase (default)
cueloop run one --phases 3
```

### Runner Overrides

```bash
cueloop run one --runner codex --model gpt-5.4 --effort high
cueloop run one --runner-phase1 codex --model-phase1 gpt-5.4 --effort-phase1 high
cueloop run one --runner-phase2 codex --model-phase2 gpt-5.4 --effort-phase2 medium
```

### Queue Operations

```bash
cueloop queue list
cueloop queue next --with-title
cueloop queue validate
cueloop queue graph --format dot
cueloop queue tree
cueloop queue archive
```

### Task Lifecycle

```bash
cueloop task build "Refactor queue parsing"
cueloop task decompose "Build OAuth login with GitHub and Google"
cueloop task decompose --from-file docs/plans/oauth.md
cueloop task decompose --from-file docs/plans/oauth.md --attach-to RQ-0042 --child-policy append --write
cueloop task decompose RQ-0001 --child-policy append --with-dependencies --write
cueloop task decompose --write --parent-status draft --leaf-status todo "Plan webhook reliability work"
cueloop task ready RQ-0003
cueloop task decompose --attach-to RQ-0042 --format json "Plan webhook reliability work"
cueloop task start RQ-0001
cueloop task status doing RQ-0001
cueloop task done RQ-0001 --note "Verified with make agent-ci"
```

On macOS, the app exposes the same workflow through `Decompose Task...` in the Task menu, command palette, queue toolbar, and task context menus, including a plan-file picker that passes `--from-file` to the machine API.

`cueloop task decompose --from-file <path>` is the planner-backed path for arbitrary plan documents. `cueloop prd create <path>` remains the PRD-parser-specific import path.

For file-backed plans, validate full-plan coverage and ordering with a preview/write/navigation pass:

```bash
cueloop task decompose --from-file docs/plans/full-plan-ordering.md --with-dependencies --format json
cueloop task decompose --from-file docs/plans/full-plan-ordering.md --with-dependencies --write
cueloop queue validate
cueloop queue tree
cueloop task children <ROOT_TASK_ID>
```

Expected result: every meaningful plan section appears as a task or a documented warning, ordered phases stay in logical execution order, and `--with-dependencies` creates sibling prerequisite edges for ordered phase work. Written decomposition trees persist umbrella/root/phase nodes as `kind: group`, leave runnable leaf work as the default `kind: work_item`, and report both the root/group task and the first actionable leaf task in text and JSON output.

Preview-only decomposition saves an exact replay checkpoint under `.cueloop/cache/decompose-previews/` and prints a copy/pasteable continuation command with the checkpoint ID.

Decomposition status controls are explicit and opt-in. `--status <STATUS>` applies to every generated node by default; `--parent-status <STATUS>` overrides generated group/non-leaf nodes; `--leaf-status <STATUS>` overrides generated leaf work items. Plain `--write` remains review-first and writes generated tasks as `draft`. To make leaf work immediately runnable while keeping parent/group nodes as drafts, use `--parent-status draft --leaf-status todo`. If a write leaves every generated task in `draft`, the continuation output prints an exact activation command such as `cueloop task ready RQ-0003` for the first actionable leaf. `cueloop queue validate` and `cueloop queue explain` use the same calm activation guidance instead of reporting an all-draft decomposition as dependency failure.

### Diagnostics

```bash
cueloop doctor
cueloop doctor --format json
cueloop runner list
cueloop runner capabilities claude
cueloop config show --format json
```

When CueLoop is not making progress, `cueloop doctor` now uses the same canonical `BlockingState` vocabulary as the live run surfaces: `waiting`, `blocked`, or `stalled`.

### Recovery and continuation

```bash
cueloop queue validate
cueloop queue repair --dry-run
cueloop queue repair
cueloop task mutate --dry-run --input request.json
cueloop task mutate --format json --input request.json
cueloop task decompose --format json "Improve webhook reliability"
cueloop task decompose --from-file docs/plans/oauth.md --format json
cueloop task decompose --write "Improve webhook reliability"
cueloop task decompose --write --from-preview <CHECKPOINT_ID>
cueloop task followups apply --task RQ-0135
cueloop task followups apply --task RQ-0135 --dry-run --format json
cueloop undo --list
cueloop undo --dry-run
```

These commands are now first-class continuation tools. They explain whether CueLoop is ready, waiting, blocked, or stalled, preserve partial value where safe, and use undo-backed writes for queue/done mutations.

If `cueloop run loop` stops on queue validation, start with `cueloop queue repair --dry-run` to preview recoverable fixes, apply them with `cueloop queue repair`, and optionally confirm the result with `cueloop queue validate`.

`cueloop task mutate --format json` and `cueloop task decompose --format json` now emit the same shared versioned continuation documents used by `cueloop machine` commands.
`cueloop task followups apply` consumes `.cueloop/cache/followups/<TASK_ID>.json`, validates the proposal, creates undo, inserts generated tasks into the queue, and records continuation state in the same family as task mutate/decompose.

### Machine API

```bash
cueloop machine system info
cueloop machine queue read
cueloop machine queue validate
cueloop machine queue repair --dry-run
cueloop machine queue undo --dry-run
cueloop machine config resolve
cueloop machine doctor report
cueloop machine task mutate --input request.json
cueloop machine task decompose --from-file docs/plans/oauth.md --with-dependencies
cueloop machine task decompose --write --from-preview <CHECKPOINT_ID>
cueloop machine run one --resume --id RQ-0001
cueloop machine run loop --resume --max-tasks 5
cueloop machine run loop --resume --max-tasks 0 --parallel 2
cueloop machine run stop
cueloop machine schema
```

Machine run loops use the same convention: `--max-tasks 0` means unlimited execution.

### Experimental Parallel Supervision

```bash
cueloop run loop --parallel 4 --max-tasks 8
cueloop run parallel status --json
cueloop run parallel retry --task RQ-0007
```

Parallel direct-push execution is experimental. Keep it out of default onboarding paths and opt in only when the repository and branch policy are ready for it.

### Daemon and Watch

```bash
cueloop daemon start
cueloop daemon status
cueloop daemon logs --follow --tail 200
cueloop watch --auto-queue
```

### Webhooks

```bash
cueloop webhook test
cueloop webhook status
cueloop webhook replay --dry-run --id <delivery-id>
```

### Prompt Management

```bash
cueloop prompt list
cueloop prompt worker --phase 1 --repo-prompt plan
cueloop prompt export --all
cueloop prompt diff worker
```

### Undo

```bash
cueloop undo --list
cueloop undo --dry-run
cueloop undo
```

## Key Subcommand Groups

### `cueloop run`

- `resume`
- `one`
- `loop`
- `parallel` (`status`, `retry`) - experimental

### `cueloop queue`

- `validate`, `prune`, `next`, `next-id`, `show`, `list`, `search`, `archive`, `repair`, `unlock`, `sort`
- Analytics/reporting: `stats`, `history`, `burndown`, `aging`, `dashboard`
- Integrations: `schema`, `graph`, `export`, `import`, `issue`, `stop`, `explain`, `tree`

### `cueloop task`

- Build/create: `task` (freeform), `build`, `refactor`, `build-refactor`, `from`, `template`, `followups`
- Lifecycle: `show`, `ready`, `status`, `done`, `reject`, `start`, `schedule`
- Editing: `field`, `edit`, `update`
- Structure/relations: `clone`, `split`, `relate`, `blocks`, `mark-duplicate`, `children`, `parent`
- Bulk operations: `batch`

## Shell Completions

```bash
cueloop completions bash > ~/.local/share/bash-completion/completions/cueloop
cueloop completions zsh > ~/.zfunc/_cueloop
cueloop completions fish > ~/.config/fish/completions/cueloop.fish
cueloop completions powershell > $PROFILE.CurrentUserAllHosts
```

## Source of Truth

For behavior that may change between releases, trust live command help first:

```bash
cueloop --help
cueloop help-all
cueloop <command> --help
cueloop <command> <subcommand> --help
```
