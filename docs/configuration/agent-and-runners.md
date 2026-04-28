# Configuration: Agent and Runners
Status: Active
Owner: Maintainers
Source of truth: this document for `agent.*`, runner controls, retry policy, phase overrides, and CI gate behavior
Parent: [Configuration](../configuration.md)

Purpose: Document Ralph's runner-facing agent configuration and execution controls.

## Agent Configuration
`agent` controls default execution settings. Defaults are schema-defined.

Supported fields:
- `runner`: Built-in runner ID (`codex`, `opencode`, `gemini`, `claude`, `cursor`, `kimi`, or `pi`) or plugin runner ID.
- `model`: default model id (string).
- `phases`: number of phases (1, 2, or 3).
- `reasoning_effort`: `low`, `medium`, `high`, `xhigh` (Codex and Pi only).
- `iterations`: number of iterations to run per task (default: 1).
- `followup_reasoning_effort`: reasoning effort for iterations after the first (Codex and Pi only).
- `repoprompt_plan_required`: inject RepoPrompt planning guidance (favoring `context_builder` when available) during Phase 1.
- `repoprompt_tool_injection`: inject RepoPrompt tooling guidance into prompts when that environment is enabled.
- `git_revert_mode`: `ask`, `enabled`, or `disabled`.
- `git_publish_mode`: automatic git publish behavior after successful runs. Supported values: `off`, `commit`, `commit_and_push` (default: `off`).
  **Safety note:** `commit_and_push` has the highest blast radius because it publishes to the remote repository automatically. Prefer `off` or `commit` unless you explicitly want automated publishing.
  **Parallel workers:** Parallel workers inherit this setting inside each workspace. Parallel execution remains experimental.
- `session_timeout_hours`: session timeout in hours for crash recovery (default: `24`). Sessions older than this threshold are considered stale and require explicit user confirmation to resume. Set to a higher value if you want to allow resuming sessions after longer periods.
- `runner_retry`: runner invocation retry/backoff configuration for transient failure handling. See [`agent.runner_retry`](#agentrunner_retry) below.
- `ci_gate`: structured CI gate config. Use `argv` only; shell-string execution is unsupported.
  **Safety warning:** Disabling the CI gate skips Ralph-managed validation before completion/publish, which may allow broken code to be pushed. This does not disable the task run itself.
- `claude_bin`, `codex_bin`, `opencode_bin`, `gemini_bin`, `cursor_bin`, `kimi_bin`, `pi_bin`: override built-in runner executable path/name (Cursor uses the `agent` binary).
- `claude_permission_mode`: `accept_edits` or `bypass_permissions`.
  **Safety warning:** `bypass_permissions` allows Claude to make edits without prompting for approval. Use with caution.
- `runner_cli`: normalized runner CLI behavior (output/approval/sandbox/etc), with global defaults and optional per-runner overrides.
- `instruction_files`: optional list of additional instruction file paths to inject at the top of every prompt sent to runner CLIs (repo-root relative, absolute, or `~/`). Each list entry must be a non-empty path; blank strings are rejected during config validation.

  To inject both global and repo-local AGENTS.md:

  ```json
  {
    "agent": {
      "instruction_files": ["~/.codex/AGENTS.md", "AGENTS.md"]
    }
  }
  ```

Notes:
- Multi-phase runs (`phases >= 2`) always refresh task fields (`scope,evidence,plan,notes,tags,depends_on`) at the start of Phase 1, then generate the plan in that same Phase 1 runner session. This behavior is built in and not configurable.
- `followup_reasoning_effort` is used by Codex and Pi runners and ignored by runners without reasoning-effort support.
- Migration-related breaking changes for `reasoning_effort`, `agent.git_publish_mode`, and older config files live in [Migration notes](migration-notes.md).
- CI gate auto-retry: When enabled, Ralph automatically sends a strict compliance message and retries up to 2 times on CI failure during Phase 2, Phase 3, or single-phase execution. This behavior is not configurable; after 2 automatic retries, the user is prompted via the configured `git_revert_mode`. Post-run supervision prompts immediately on CI failure.
- Phase 1 plan-only violations: when `git_revert_mode=ask`, the prompt includes a keep+continue override to proceed to the next phase without reverting changes.
- **Runner session handling**: For runners that support session resumption (e.g., Kimi), Ralph generates unique session IDs per phase (format: `{task_id}-p{phase}-{timestamp}`) and uses explicit `--session` flags rather than runner-specific continue mechanisms. This provides deterministic session management and reliable crash recovery.
- **macOS app boundary**: app-launched runs are noninteractive. The app can display the resolved approval posture, but interactive approvals remain terminal-only until the transport changes.

### `agent.runner_cli`

`agent.runner_cli` provides a normalized configuration surface for runner CLI behavior so Ralph can keep parity across runners while still emitting runner-specific flags.

Structure:
- `agent.runner_cli.defaults`: applied to all runners (unless overridden)
- `agent.runner_cli.runners.<runner>`: per-runner overrides (merged leaf-wise over `defaults`)

Supported normalized fields:
- `output_format`: `stream_json`, `json`, `text` (execution requires `stream_json`)
- `verbosity`: `quiet`, `normal`, `verbose`
- `approval_mode`: `default`, `auto_edits`, `yolo`, `safe`
  **Safety warning:** `yolo` mode bypasses all approval prompts, allowing the runner to make changes without confirmation. The recommended default profile is `safe`.

  **Codex exception**: Ralph does NOT pass approval flags to Codex, regardless of this setting. Codex will use whatever approval mode is configured in your global Codex config file (`~/.codex/config.json`). If you want YOLO behavior with Codex, configure it there, not in Ralph.
- `sandbox`: `default`, `enabled`, `disabled`
- `plan_mode`: `default`, `enabled`, `disabled`
- `unsupported_option_policy`: `ignore`, `warn`, `error`

Notes:
- Unsupported options are dropped by default with a warning (policy `warn`).
- `agent.claude_permission_mode` remains supported; when `runner_cli.approval_mode` is set, it takes precedence for Claude mapping.
Example:

```json
{
  "version": 2,
  "agent": {
    "runner": "codex",
    "model": "gpt-5.4",
    "phases": 3,
    "iterations": 2,
    "reasoning_effort": "high",
    "followup_reasoning_effort": "low",
    "repoprompt_plan_required": false,
    "repoprompt_tool_injection": false,
    "git_publish_mode": "off",
    "git_revert_mode": "ask",
    "claude_permission_mode": "accept_edits",
    "runner_cli": {
      "defaults": {
        "output_format": "stream_json",
        "approval_mode": "default",
        "unsupported_option_policy": "warn"
      },
      "runners": {
        "codex": { "sandbox": "disabled" },
        "claude": { "verbosity": "verbose" }
      }
    },
    "ci_gate": {
      "enabled": true,
      "argv": ["make", "ci"]
    }
  }
}
```

To disable CI gating entirely (skip Ralph-managed execution of the configured CI command), set:

```json
{
  "agent": {
    "ci_gate": {
      "enabled": false
    }
  }
}
```

When `agent.ci_gate.enabled=false`, Ralph still runs all task phases; prompts should report that configured CI validation was skipped by configuration and summarize any other verification performed.

To configure a longer session timeout for crash recovery (e.g., 72 hours for weekend-long tasks):

```json
{
  "agent": {
    "session_timeout_hours": 72
  }
}
```

### `agent.runner_retry`

Runner invocation retry/backoff configuration for transient failure handling. Controls automatic retry behavior when runner invocations fail with transient errors (rate limits, temporary unavailability, network issues). Distinct from webhook retry settings (`agent.webhook.retry_*`).

**Fields:**
- `max_attempts`: Total attempts including initial (default: `3`, range: `1-20`).
- `base_backoff_ms`: Base backoff in milliseconds (default: `1000`, range: `0-600000`).
- `multiplier`: Exponential multiplier (default: `2.0`, range: `1.0-10.0`).
- `max_backoff_ms`: Maximum backoff cap in milliseconds (default: `30000`, range: `0-600000`).
- `jitter_ratio`: Jitter ratio in `[0,1]` for variance (default: `0.2`, range: `0.0-1.0`).

**Retry classification:**
- **Retryable**: Rate limits (HTTP 429), temporary unavailability (HTTP 503), transient I/O errors (connection reset, timeout), and timeouts.
- **Requires user input**: Authentication failures (HTTP 401), missing binaries.
- **Non-retryable**: Invalid invocations, fatal exits, interruptions (Ctrl+C).

**Example:**

```json
{
  "agent": {
    "runner_retry": {
      "max_attempts": 5,
      "base_backoff_ms": 2000,
      "multiplier": 2.0,
      "max_backoff_ms": 60000,
      "jitter_ratio": 0.2
    }
  }
}
```

Notes:
- Retries only occur when the repository is clean (or dirty only in Ralph-allowed paths like `.ralph/`), or when `git_revert_mode` is `enabled` for auto-revert.
- Retry attempt counts and backoff delays are emitted via `RALPH_OPERATION:` markers in runner output.
- To disable retry entirely, set `max_attempts: 1`.

### `agent.phase_overrides`

Optional. Per-phase overrides for runner, model, and reasoning effort. Allows using different runners or models for different phases of task execution.

**Structure:**
- `phase1` - Overrides for phase 1 (planning)
- `phase2` - Overrides for phase 2 (implementation)
- `phase3` - Overrides for phase 3 (review)

Each phase config can specify:
- `runner` - Override the runner (e.g., "codex", "claude")
- `model` - Override the model (e.g., "o3-mini", "claude-opus-4")
- `reasoning_effort` - Override reasoning effort ("low", "medium", "high", "xhigh")

**Example:**

```json
{
  "agent": {
    "runner": "codex",
    "model": "gpt-5.4",
    "reasoning_effort": "medium",
    "phase_overrides": {
      "phase1": {
        "model": "gpt-5.3",
        "reasoning_effort": "high"
      },
      "phase2": {
        "runner": "codex",
        "model": "gpt-5.4",
        "reasoning_effort": "medium"
      },
      "phase3": {
        "runner": "codex",
        "model": "gpt-5.4",
        "reasoning_effort": "high"
      }
    }
  }
}
```

**Precedence (per phase):** CLI phase flags > task phase override (`task.agent.phase_overrides.phaseN.*`) > config phase override (`agent.phase_overrides.phaseN.*`) > CLI global overrides > task global overrides (`task.agent.*`) > config defaults (`agent.*`) > code defaults
