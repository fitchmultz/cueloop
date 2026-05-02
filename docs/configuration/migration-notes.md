# Configuration: Migration Notes
Status: Active
Owner: Maintainers
Source of truth: this document for current configuration migration notes and breaking config changes
Parent: [Configuration](../configuration.md)

Purpose: Collect configuration-breaking changes and the supported migration commands for existing CueLoop/CueLoop config files.

## Current migration command

Run the config migrator after upgrading older repositories or global configs. Use the primary `cueloop` executable:

```bash
cueloop migrate --apply
```

Use `cueloop migrate` without `--apply` first when you want a preview.

## CueLoop runtime directory migration

New repositories default to `.cueloop/`. Legacy `.ralph/` runtime directories remain supported, including project config, queue/done files, plugins, prompts, caches, and trust files.

The runtime directory move is explicit and separate from normal config migrations:

```bash
cueloop migrate runtime-dir --check
cueloop migrate runtime-dir --apply
```

Use `--check` before `--apply`. The migration moves `.ralph/` to `.cueloop/` when safe and refuses to auto-merge when both directories exist. Do not rename runtime directories manually.

## Version 0.3 config changes

- Config files must use `"version": 2`.
- `agent.git_publish_mode` replaces the removed `git_commit_push_enabled` setting.
- Built-in reserved profiles are `safe` and `power-user`; defining either name in config is a validation error.
- `make install` updates the CLI and macOS app bundle, but it does not mutate repo-local config files. Older repos still need `cueloop migrate --apply` after upgrading to `0.3`.

## Runner and reasoning changes

- `reasoning_effort` no longer accepts `minimal`; use `low`, `medium`, `high`, or `xhigh`.
- Cursor runner execution now uses CueLoop's local Cursor SDK bridge. Legacy `agent.cursor_bin` and `profiles.<name>.cursor_bin` settings are removed by `cueloop migrate --apply`; use `agent.cursor_sdk_node_bin` only when you need to override the Node.js executable. Project-level Cursor selection requires repo trust because the target workspace can provide `@cursor/sdk@1.0.11`; alternatively set `RALPH_CURSOR_SDK_MODULE_PATH` to a trusted/global SDK entrypoint.

## Parallel configuration changes

- Parallel mode was rewritten for direct-push execution in 2026-02.
- Removed keys: `auto_pr`, `auto_merge`, `merge_when`, `merge_method`, `merge_retries`, `draft_on_failure`, `conflict_policy`, `branch_prefix`, `delete_branch_on_merge`, and `merge_runner`.
- `parallel.worktree_root` was renamed to `parallel.workspace_root`; configs using the old key fail to load until migrated.

## Profile name changes

`quick` and `thorough` are no longer built in. If your team depends on those names, define them explicitly under `profiles` in your config. See [Plugins and profiles](plugins-and-profiles.md#migration-from-retired-default-names).

## Related chapters

- [Trust and precedence](trust-and-precedence.md)
- [Agent and runners](agent-and-runners.md)
- [Queue and parallel](queue-and-parallel.md)
- [Plugins and profiles](plugins-and-profiles.md)
