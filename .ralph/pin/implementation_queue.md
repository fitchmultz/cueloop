# Implementation Queue

## Queue
- [ ] RQ-0415 [code]: Remove or implement dead config knobs (runner.max_workers/dry_run, loop.workers/poll_seconds, git.require_clean/commit_prefix) so settings actually do something. (ralph_tui/internal/config/config.go, ralph_tui/internal/config/defaults.json, ralph_tui/internal/tui/config_editor.go, ralph_tui/cmd/ralph/main.go, ralph_tui/internal/loop/loop.go)
  - Evidence: config schema + config editor expose several fields that are not wired into behavior anywhere (runner.max_workers, runner.dry_run, loop.workers, loop.poll_seconds, git.require_clean, git.commit_prefix), so users can change them but nothing changes at runtime.
  - Plan: Audit each knob and either wire it into loop/specs/TUI behavior with clear semantics, or deprecate/remove it with a migration step and updated docs/tests; ensure config validation stays correct and does not enforce unused fields.

## Blocked

## Parking Lot
