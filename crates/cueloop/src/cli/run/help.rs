//! Long-help text for `cueloop run`.
//!
//! Purpose:
//! - Long-help text for `cueloop run`.
//!
//! Responsibilities:
//! - Centralize verbose clap help text separately from clap type definitions.
//!
//! Not handled here:
//! - CLI argument parsing or dispatch.
//!
//!
//! Usage:
//! - Used through the crate module tree or integration test harness.
//!
//! Invariants/assumptions:
//! - Help strings stay as `'static` constants for clap attributes.

pub(super) const RUN_AFTER_LONG_HELP: &str = "Runner selection:\n\
  - `cueloop run` selects runner/model/effort with this precedence:\n\
  1) CLI overrides (flags on `run one` / `run loop`)\n\
  2) task's `agent` override (runner/model plus `model_effort` if set)\n\
  3) otherwise: resolved config defaults (`agent.runner`, `agent.model`, `agent.reasoning_effort`).\n\
 \n\
Loop limits:\n\
  - Safe default: use `cueloop run one` or `cueloop run loop --max-tasks <N>` with a positive cap.\n\
  - Advanced unlimited mode: `cueloop run loop --max-tasks 0` runs until CueLoop runs out of runnable work, blocks, or is stopped.\n\
 \n\
 Resume behavior:\n\
  - CueLoop now narrates whether it is resuming the same session, starting fresh, or refusing to guess.\n\
  - `cueloop run one` inspects interrupted-session state too; add `--resume` to auto-continue when safe.\n\
  - Timed-out sessions still require explicit confirmation and are refused in non-interactive mode.\n\
 \n\
 Blocking-state diagnosis:\n\
  - CueLoop uses one canonical BlockingState vocabulary everywhere: waiting, blocked, stalled.\n\
  - Recovery-entry commands (`task mutate`, `task decompose`, `queue validate`, `queue repair`, `undo`) use the same waiting/blocked/stalled narration when CueLoop needs operator guidance.\n\
  - Canonical reasons are: idle, dependency_blocked, schedule_blocked, lock_blocked, ci_blocked, runner_recovery, operator_recovery, mixed_queue.\n\
  - Use `cueloop doctor` for human-readable diagnosis when CueLoop is not making progress.\n\
  - Use `cueloop doctor --format json` or `cueloop machine doctor report` for machine-readable blocking diagnosis.\n\
 \n\
 Notes:\n\
  - Allowed runners: codex, opencode, gemini, claude, cursor, kimi, pi\n\
  - Allowed models: gpt-5.4, gpt-5.3-codex, gpt-5.3-codex-spark, gpt-5.3, zai-coding-plan/glm-4.7, gemini-3-pro-preview, gemini-3-flash-preview, sonnet, opus, kimi-for-coding (codex supports only gpt-5.4 + gpt-5.3-codex + gpt-5.3-codex-spark + gpt-5.3; opencode/gemini/claude/cursor/kimi/pi accept arbitrary model ids)\n\
  - `--effort` is codex-only and is ignored for other runners.\n\
  - `--git-revert-mode` controls whether CueLoop reverts uncommitted changes on errors (ask, enabled, disabled).\n\
  - `--git-publish-mode` controls post-run git behavior: off, commit, or commit_and_push.\n\
  - `--parallel` is experimental and runs loop tasks concurrently in workspaces (clone-based).\n\
  - Experimental parallel workers push directly to the target branch after phase execution.\n\
  - Clean-repo checks allow changes to `.cueloop/config.jsonc`, `.cueloop/queue.jsonc`, and `.cueloop/done.jsonc`; use `--force` to bypass entirely.\n\
 \n\
Phase-specific overrides:\n\
  Use --runner-phaseN, --model-phaseN, --effort-phaseN to override settings for a specific phase.\n\
  Phase-specific flags take precedence over global flags for that phase.\n\
  Single-pass (--phases 1) uses Phase 2 overrides.\n\
 \n\
  Precedence per phase (highest to lowest):\n\
    1) CLI phase override (--runner-phaseN, --model-phaseN, --effort-phaseN)\n\
    2) Task phase override (task.agent.phase_overrides.phaseN.*)\n\
    3) Config phase override (agent.phase_overrides.phaseN.*)\n\
    4) CLI global override (--runner, --model, --effort)\n\
    5) Task global override (task.agent.runner/model/model_effort)\n\
    6) Config defaults (agent.*)\n\
 \n\
 To change defaults for this repo, edit .cueloop/config.jsonc:\n\
  version: 2\n\
  agent:\n\
  runner: codex\n\
  model: gpt-5.4\n\
  gemini_bin: gemini\n\
 \n\
Examples:\n\
 cueloop run one\n\
 cueloop run one --resume\n\
 cueloop run one --phases 2\n\
 cueloop run one --phases 1\n\
 cueloop run one --runner opencode --model gpt-5.3\n\
 cueloop run one --runner codex --model gpt-5.4 --effort high\n\
 cueloop run one --runner-phase1 codex --model-phase1 gpt-5.4 --effort-phase1 high\n\
 cueloop run one --runner-phase2 claude --model-phase2 opus\n\
 cueloop run one --runner gemini --model gemini-3-flash-preview\n\
 cueloop run one --runner pi --model gpt-5.3\n\
 cueloop run one --include-draft\n\
 cueloop run one --git-revert-mode disabled\n\
 cueloop run one --git-publish-mode off\n\
 cueloop run one --lfs-check\n\
 cueloop run loop --max-tasks 1\n\
 cueloop run loop --max-tasks 1 --runner opencode --model gpt-5.3\n\
 cueloop run loop --include-draft --max-tasks 1\n\
 cueloop run loop --git-revert-mode ask --max-tasks 1\n\
 cueloop run loop --git-publish-mode commit_and_push --max-tasks 1\n\
 cueloop run loop --lfs-check --max-tasks 1\n\
 cueloop run loop --parallel --max-tasks 4\n\
 cueloop run loop --parallel 4 --max-tasks 8\n\
 cueloop run loop --max-tasks 0 (advanced unlimited)\n\
 cueloop run resume\n\
 cueloop run resume --force\n\
 cueloop run loop --resume --max-tasks 5";

pub(super) const RESUME_AFTER_LONG_HELP: &str = "Resume behavior:\n\
 - If the saved session is still valid, CueLoop resumes the same interrupted task.\n\
 - Resume treats the current dirty tree as the interrupted task baseline and restarts at the saved phase.\n\
 - `--force` is only for bypassing other safety checks.\n\
 - If the saved session is stale or no longer safe, CueLoop says so and starts fresh.\n\
 - If confirmation is required but unavailable (for example `--non-interactive`), CueLoop refuses instead of guessing.\n\
\n\
Examples:\n\
 cueloop run resume\n\
 cueloop run resume --force\n\
 cueloop run resume --non-interactive";

pub(super) const RUN_ONE_AFTER_LONG_HELP: &str = "Runner selection (precedence):\n\
 1) CLI overrides (--runner/--model/--effort)\n\
 2) task.agent in the configured queue file (if present)\n\
 3) selected profile (if --profile specified)\n\
 4) config defaults (.cueloop/config.jsonc then ~/.config/cueloop/config.jsonc)\n\
\n\
Resume behavior:\n\
 - `cueloop run one` inspects interrupted-session state before selecting work.\n\
 - `cueloop run one --resume` auto-resumes the interrupted session when CueLoop can do so safely.\n\
 - A valid resume treats the current dirty tree as the interrupted task baseline and restarts at the saved phase.\n\
 - Explicit `--id <TASK_ID>` beats an unrelated interrupted session, and CueLoop says so.\n\
 - If confirmation is required but unavailable (for example `--non-interactive`), CueLoop refuses instead of silently guessing.\n\
\n\
Blocking-state diagnosis:\n\
 - If CueLoop refuses to continue, stalls, or appears blocked, run `cueloop doctor`.\n\
 - `cueloop doctor` uses the same BlockingState explanation shown by run, machine, and app surfaces.\n\
\n\
Examples:\n\
 cueloop run one\n\
 cueloop run one --resume\n\
 cueloop run one --id RQ-0001\n\
 cueloop run one --id RQ-0001 --resume\n\
 cueloop run one --debug\n\
 cueloop run one --profile fast-local\n\
 cueloop run one --profile deep-review\n\
 cueloop run one --phases 3 (plan/implement+CI/review+complete)\n\
 cueloop run one --phases 2 (plan/implement)\n\
 cueloop run one --phases 1 (single-pass)\n\
 cueloop run one --quick (single-pass, same as --phases 1)\n\
 cueloop run one --runner opencode --model gpt-5.3\n\
 cueloop run one --runner gemini --model gemini-3-flash-preview\n\
 cueloop run one --runner pi --model gpt-5.3\n\
 cueloop run one --runner codex --model gpt-5.4 --effort high\n\
 cueloop run one --runner-phase1 codex --model-phase1 gpt-5.4 --effort-phase1 high\n\
 cueloop run one --runner-phase2 claude --model-phase2 opus\n\
 cueloop run one --include-draft\n\
 cueloop run one --git-revert-mode enabled\n\
 cueloop run one --git-publish-mode off\n\
 cueloop run one --lfs-check\n\
 cueloop run one --repo-prompt plan\n\
 cueloop run one --repo-prompt off\n\
 cueloop run one --non-interactive\n\
 cueloop run one --dry-run\n\
 cueloop run one --dry-run --include-draft\n\
 cueloop run one --dry-run --id RQ-0001";

pub(super) const RUN_LOOP_AFTER_LONG_HELP: &str = "Resume behavior:\n\
 - `cueloop run loop --resume` auto-resumes the interrupted session when safe.\n\
 - A valid resume treats the current dirty tree as the interrupted task baseline and restarts at the saved phase.\n\
 - Without `--resume`, CueLoop still narrates stale/fresh/refusal cases instead of hiding them.\n\
 - If confirmation is required but unavailable (for example `--non-interactive`), CueLoop refuses instead of silently guessing.\n\
\n\
Loop limits:\n\
 - Safe default: use a positive `--max-tasks` value when you want a fixed cap.\n\
 - Advanced unlimited mode: `--max-tasks 0` means unlimited successful iterations.\n\
\n\
Blocking-state diagnosis:\n\
 - `cueloop run loop` emits canonical blocking states during waiting and stall transitions.\n\
 - Use `cueloop doctor` to diagnose the same state outside the live run loop.\n\
\n\
Queue validation recovery:\n\
 - If the loop stops on queue validation, preview repair with `cueloop queue repair --dry-run`.\n\
 - Apply recoverable fixes with `cueloop queue repair`, then re-run `cueloop queue validate` if desired.\n\
\n\
Examples:\n\
 cueloop run loop --max-tasks 1\n\
 cueloop run loop --profile fast-local --max-tasks 5\n\
 cueloop run loop --profile deep-review --max-tasks 5\n\
 cueloop run loop --resume --max-tasks 5\n\
 cueloop run loop --phases 3 --max-tasks 3 (plan/implement+CI/review+complete)\n\
 cueloop run loop --phases 2 --max-tasks 3 (plan/implement)\n\
 cueloop run loop --phases 1 --max-tasks 1 (single-pass)\n\
 cueloop run loop --quick --max-tasks 1 (single-pass, same as --phases 1)\n\
 cueloop run loop --max-tasks 3\n\
 cueloop run loop --max-tasks 1 --debug\n\
 cueloop run loop --max-tasks 1 --runner opencode --model gpt-5.3\n\
 cueloop run loop --runner-phase1 codex --model-phase1 gpt-5.4 --effort-phase1 high --max-tasks 1\n\
 cueloop run loop --runner-phase2 claude --model-phase2 opus --max-tasks 1\n\
 cueloop run loop --include-draft --max-tasks 1\n\
 cueloop run loop --git-revert-mode disabled --max-tasks 1\n\
 cueloop run loop --git-publish-mode off --max-tasks 1\n\
 cueloop run loop --repo-prompt tools --max-tasks 1\n\
 cueloop run loop --repo-prompt off --max-tasks 1\n\
 cueloop run loop --lfs-check --max-tasks 1\n\
 cueloop run loop --dry-run\n\
 cueloop run loop --wait-when-blocked\n\
 cueloop run loop --wait-when-blocked --wait-timeout-seconds 600\n\
 cueloop run loop --wait-when-blocked --wait-poll-ms 250\n\
 cueloop run loop --wait-when-blocked --notify-when-unblocked\n\
 Advanced unlimited mode:\n\
 cueloop run loop --max-tasks 0 (intentional unlimited)\n\
 cueloop run loop --phases 2 --max-tasks 0 (intentional unlimited, plan/implement)";

pub(super) const PARALLEL_AFTER_LONG_HELP: &str = "Experimental direct-push parallel execution.\n\
\n\
Examples:\n\
 cueloop run parallel status\n\
 cueloop run parallel status --json\n\
 cueloop run parallel retry --task RQ-0001";

pub(super) const PARALLEL_STATUS_AFTER_LONG_HELP: &str = "Examples:\n\
 cueloop run parallel status\n\
 cueloop run parallel status --json\n\
 cueloop run parallel retry --task RQ-0001";

pub(super) const PARALLEL_RETRY_AFTER_LONG_HELP: &str = "Examples:\n\
 cueloop run parallel retry --task RQ-0001\n\
 cueloop run loop --parallel <N>";
