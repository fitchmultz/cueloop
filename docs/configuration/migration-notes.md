# Configuration: Migration Notes
Status: Active
Owner: Maintainers
Source of truth: this document for current configuration migration notes and breaking config changes
Parent: [Configuration](../configuration.md)

Purpose: Collect configuration-breaking changes and the supported migration commands for existing Ralph config files.

## Current migration command

Run the config migrator after upgrading older repositories or global configs:

```bash
ralph migrate --apply
```

Use `ralph migrate` without `--apply` first when you want a preview.

## Version 0.3 config changes

- Config files must use `"version": 2`.
- `agent.git_publish_mode` replaces the removed `git_commit_push_enabled` setting.
- Built-in reserved profiles are `safe` and `power-user`; defining either name in config is a validation error.
- `make install` updates the CLI and macOS app bundle, but it does not mutate repo-local config files. Older repos still need `ralph migrate --apply` after upgrading to `0.3`.

## Runner and reasoning changes

- `reasoning_effort` no longer accepts `minimal`; use `low`, `medium`, `high`, or `xhigh`.

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
