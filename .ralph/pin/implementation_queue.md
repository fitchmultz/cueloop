# Implementation Queue

## Queue
- [ ] RQ-0414 [code]: Fix CLI subcommands ignoring config defaults (runner, runner_args, reasoning_effort), making CLI runs diverge from the TUI. (ralph_tui/cmd/ralph/main.go, ralph_tui/internal/config/load.go, ralph_tui/internal/tui/args_helpers.go)
  - Evidence: `ralph loop run` defaults runnerName to "codex" and uses only positional args, ignoring cfg.Loop.Runner/RunnerArgs; `ralph specs build` defaults to codex and ignores cfg.Specs.Runner/RunnerArgs/ReasoningEffort unless flags are explicitly provided. This forces users to re-specify settings and causes surprising CLI-vs-TUI behavior differences.
  - Plan: When flags are not changed, default to cfg.Specs/Loop values; merge config runner args with CLI args; apply reasoning_effort consistently via shared helper; add unit tests for precedence (config vs flags vs positionals).
- [ ] RQ-0415 [code]: Remove or implement dead config knobs (runner.max_workers/dry_run, loop.workers/poll_seconds, git.require_clean/commit_prefix) so settings actually do something. (ralph_tui/internal/config/config.go, ralph_tui/internal/config/defaults.json, ralph_tui/internal/tui/config_editor.go, ralph_tui/cmd/ralph/main.go, ralph_tui/internal/loop/loop.go)
  - Evidence: config schema + config editor expose several fields that are not wired into behavior anywhere (runner.max_workers, runner.dry_run, loop.workers, loop.poll_seconds, git.require_clean, git.commit_prefix), so users can change them but nothing changes at runtime.
  - Plan: Audit each knob and either wire it into loop/specs/TUI behavior with clear semantics, or deprecate/remove it with a migration step and updated docs/tests; ensure config validation stays correct and does not enforce unused fields.

## Blocked

## Parking Lot
