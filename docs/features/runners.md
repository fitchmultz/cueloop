# CueLoop Runners System
Status: Active
Owner: Maintainers
Source of truth: this document for its stated scope
Parent: [Feature Documentation](README.md)


Purpose: Comprehensive documentation for CueLoop's AI runner orchestration system, including supported runners, configuration, and extension mechanisms.

## Overview

CueLoop's runners system provides a unified interface for executing AI agents across multiple CLI-based code generation tools. Runners are external binaries that CueLoop orchestrates to perform task planning, implementation, and review.

### Supported Runners

CueLoop supports 7 built-in runners and a plugin system for custom runners:

| Runner | Provider | Best For | Default Model |
|--------|----------|----------|---------------|
| **Claude** | Anthropic | Complex reasoning, code review | sonnet |
| **Codex** | OpenAI | Expert coding workflows, fastest path to production changes | gpt-5.4 |
| **OpenCode** | Flexible | Custom model selection | zai-coding-plan/glm-4.7 |
| **Gemini** | Google | Google ecosystem integration | gemini-3-pro-preview |
| **Cursor** | Cursor | IDE-integrated workflows | (cursor-specific) |
| **Kimi** | Moonshot AI | Fast execution, session management | kimi-for-coding |
| **Pi** | Pi Coding Agent | Lightweight tasks | gpt-5.3 |

### Runner Comparison

| Feature | Claude | Codex | OpenCode | Gemini | Cursor | Kimi | Pi |
|---------|--------|-------|----------|--------|--------|------|-----|
| Session Resume | ✅ | ✅ | ✅ | ✅ | ✅ | ✅* | ✅ |
| Custom Models | ✅ | ❌** | ✅ | ✅ | ✅ | ✅ | ✅ |
| Reasoning Effort | ❌ | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ |
| Sandbox Control | Limited | ✅ | ❌ | ✅ | ✅ | ❌ | Limited |
| Approval Modes | ✅ | Config file | ❌ | ✅ | ✅ | ✅ | ✅ |
| Verbose Output | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| Plan Mode | ❌ | ❌ | ❌ | ❌ | ✅ | ❌ | ❌ |

*Kimi requires CueLoop-managed session IDs (see [Session Management](#session-management))
**Codex only supports specific OpenAI models

## Supported Runners

### Claude (Anthropic)

**Best for:** Complex reasoning tasks, multi-file refactoring, code review, and scenarios requiring deep context understanding.

**Model Options:**
- `sonnet` (default) - Balanced performance and speed
- `opus` - Most capable, best for complex tasks
- Arbitrary model IDs (e.g., `claude-opus-4`, `claude-sonnet-4`)

**Permission Modes:**
- `accept_edits` - Auto-approve edits but prompt for other actions
- `bypass_permissions` - Full auto-approval (use with caution)

**Special Configuration:**
```json
{
  "agent": {
    "runner": "claude",
    "model": "sonnet",
    "claude_permission_mode": "accept_edits"
  }
}
```

**CLI Flags Mapped:**
- `--verbose` - When `verbosity=verbose`
- `--permission-mode` - Based on `approval_mode` or `claude_permission_mode`

### Codex (OpenAI)

**Best for:** Expert coding work with built-in reasoning effort control and CueLoop's strongest default workflow.

**Allowed Models (Restricted):**
Codex only supports this restricted model list:
- `gpt-5.4` (default)
- `gpt-5.3-codex`
- `gpt-5.3-codex-spark`
- `gpt-5.3`

> **Important:** Codex will reject arbitrary model IDs. CueLoop automatically normalizes incompatible models to the Codex default.

**Reasoning Effort:**
- `low` - Fastest, minimal reasoning
- `medium` (default) - Balanced
- `high` - More thorough reasoning
- `xhigh` - Maximum reasoning (consumes quota rapidly)

**Special Behavior:**
> **INTENDED BEHAVIOR:** CueLoop should pass approval flags to Codex based on `runner_cli.approval_mode`.
>
> **CURRENTLY IMPLEMENTED BEHAVIOR:** CueLoop intentionally does NOT pass any approval flags (`-a`, `--ask-for-approval`) to Codex. This allows Codex to use the user's global config file (`~/.codex/config.json`) settings. If you want YOLO behavior with Codex, configure it in `~/.codex/config.json`, not in CueLoop.

**Sandbox Control:**
- `enabled` - Uses `--sandbox workspace-write`
- `disabled` - Uses `--dangerously-bypass-approvals-and-sandbox`

**Example Configuration:**
```json
{
  "agent": {
    "runner": "codex",
    "model": "gpt-5.4",
    "reasoning_effort": "high",
    "runner_cli": {
      "defaults": {
        "sandbox": "enabled"
      }
    }
  }
}
```

### OpenCode

**Best for:** Flexibility - supports arbitrary model IDs from various providers.

**Model Options:**
- `zai-coding-plan/glm-4.7` (default)
- Any arbitrary model ID (e.g., `custom-provider/model-name`)

**Special Features:**
- Uses temp prompt files (`--file`) rather than stdin
- Supports session resumption with `-s <session_id>`

**Example Configuration:**
```json
{
  "agent": {
    "runner": "opencode",
    "model": "zai-coding-plan/glm-4.7"
  }
}
```

### Gemini (Google)

**Best for:** Google ecosystem integration and users familiar with Gemini models.

**Model Options:**
- `gemini-3-pro-preview` - Most capable
- `gemini-3-flash-preview` - Faster, lighter
- Any arbitrary model ID

**CLI Options Mapped:**
- `--approval-mode` - `yolo`, `auto_edit`, or default
- `--sandbox` - When sandbox is enabled

**Example Configuration:**
```json
{
  "agent": {
    "runner": "gemini",
    "model": "gemini-3-pro-preview",
    "runner_cli": {
      "defaults": {
        "approval_mode": "yolo"
      }
    }
  }
}
```

### Cursor

**Best for:** Users who want IDE-integrated AI capabilities through CueLoop's orchestration.

**Model Options:**
- Arbitrary model IDs supported
- Cursor uses CueLoop's local SDK bridge through Node and `@cursor/sdk`

**Special Features:**
- Durable SDK agent IDs are used for resume (`agent-...` locally, `bc-...` for cloud IDs)
- Phase-aware sandbox defaults (enabled for planning, disabled for implementation)
- Project-level Cursor selection requires repository trust because the SDK can be resolved from the workspace

**SDK Options Mapped:**
- `local.sandboxOptions.enabled` - `enabled`, `disabled`, or phase-dependent default
- `local.settingSources` - CueLoop defaults to project, user, and plugin settings so `.cursor/` context is available

**Unsupported SDK Options:**
- Cursor SDK plan mode is not used. CueLoop's own planning phase still writes CueLoop plan/cache artifacts.

`approval_mode=yolo` remains the default runner posture for Cursor, but CueLoop does not map it to SDK `local.force`; that SDK option is active-run recovery, not approval control.

**Example Configuration:**
```json
{
  "agent": {
    "runner": "cursor",
    "model": "composer-2",
    "cursor_sdk_node_bin": "node"
  }
}
```

### Kimi

**Best for:** Fast execution with explicit session management requirements.

**Model Options:**
- `kimi-for-coding` (default) - Kimi 2.5 coding model
- Any arbitrary model ID

**Special Session Handling:**
> **INTENDED BEHAVIOR:** Kimi should emit session IDs in JSON output for automatic tracking.
>
> **CURRENTLY IMPLEMENTED BEHAVIOR:** Kimi does not emit session IDs in its JSON output. CueLoop must supply and manage session IDs explicitly using the `--session` flag. This is why `requires_managed_session_id()` returns `true` for Kimi.

**CLI Flags Mapped:**
- `--yolo` / `-y` - When `approval_mode=yolo` (Kimi doesn't use `--approval-mode`)
- `--session` - CueLoop-managed session ID
- `--print` - Non-interactive mode

**Example Configuration:**
```json
{
  "agent": {
    "runner": "kimi",
    "model": "kimi-for-coding"
  }
}
```

### Pi

**Best for:** Lightweight tasks and users of the Pi Coding Agent ecosystem.

**Model Options:**
- `gpt-5.3` (default)
- Any arbitrary model ID

**Session Handling:**
Pi sessions are file-based. CueLoop resolves session files from:
1. Direct path if the session_id is a file
2. `$PI_CODING_AGENT_DIR/sessions/<workspace-dir>/*_<session_id>.jsonl`
3. `~/.pi/agent/sessions/<workspace-dir>/*_<session_id>.jsonl`

**CLI Flags Mapped:**
- `--print` / `-p` - When `approval_mode=yolo` or `auto_edits`
- `--sandbox` - When sandbox is enabled

**Example Configuration:**
```json
{
  "agent": {
    "runner": "pi",
    "model": "gpt-5.3"
  }
}
```

## Runner Configuration

### Binary Path Configuration

Override runner binary paths in your config:

```json
{
  "agent": {
    "claude_bin": "claude",
    "codex_bin": "codex",
    "opencode_bin": "opencode",
    "gemini_bin": "gemini",
    "cursor_sdk_node_bin": "node",
    "kimi_bin": "kimi",
    "pi_bin": "pi"
  }
}
```

**Note:** Cursor uses CueLoop's Node-based Cursor SDK bridge, not the legacy `agent` binary. The trusted target workspace must be able to resolve the pinned `@cursor/sdk` package (for example, via `npm install --save-exact @cursor/sdk@1.0.11`) or `RALPH_CURSOR_SDK_MODULE_PATH` must point to a trusted/global SDK entrypoint.

### Configuration Precedence

Runner settings are resolved in this order (highest to lowest):

1. **CLI flags** (e.g., `--runner`, `--model`, `--effort`)
2. **Task overrides** (`task.agent.*` in queue)
3. **Config phase overrides** (`agent.phase_overrides.phaseN.*`)
4. **CLI global overrides** (from `--runner-cli-*` flags)
5. **Config defaults** (`agent.*`)
6. **Code defaults** (schema defaults)

### Phase Overrides

Configure different runners/models for different phases:

```json
{
  "agent": {
    "runner": "codex",
    "model": "gpt-5.3-codex",
    "reasoning_effort": "medium",
    "phase_overrides": {
      "phase1": {
        "model": "gpt-5.3",
        "reasoning_effort": "high"
      },
      "phase2": {
        "runner": "kimi",
        "model": "kimi-code/kimi-for-coding"
      },
      "phase3": {
        "runner": "claude",
        "model": "opus"
      }
    }
  }
}
```

## Model Selection

### Per-Runner Allowed Models

| Runner | Model Type | Examples |
|--------|------------|----------|
| **Claude** | Named + Arbitrary | `sonnet`, `opus`, `claude-opus-4` |
| **Codex** | Restricted list only | `gpt-5.4`, `gpt-5.3-codex`, `gpt-5.3-codex-spark`, `gpt-5.3` |
| **OpenCode** | Arbitrary | `zai-coding-plan/glm-4.7`, `provider/model` |
| **Gemini** | Named + Arbitrary | `gemini-3-pro-preview`, `custom-model` |
| **Cursor** | Arbitrary | Any valid Cursor model ID |
| **Kimi** | Named + Arbitrary | `kimi-for-coding`, `custom-model` |
| **Pi** | Named + Arbitrary | `gpt-5.3`, `custom-model` |

### Model Normalization

When a model is incompatible with a runner, CueLoop automatically normalizes:

- Codex-only models (`gpt-5.*-codex`) → runner's default when used with other runners
- Non-Codex models → `gpt-5.4` when used with Codex

### Using Arbitrary Model IDs

For runners supporting arbitrary IDs, specify any model string:

```bash
cueloop run one --runner claude --model claude-opus-4
cueloop run one --runner gemini --model gemini-custom-v1
cueloop run one --runner opencode --model my-provider/my-model
```

## Runner CLI Normalization

CueLoop provides a normalized configuration surface for runner CLI behavior via `agent.runner_cli`.

### Structure

```json
{
  "agent": {
    "runner_cli": {
      "defaults": {
        "output_format": "stream_json",
        "approval_mode": "yolo",
        "sandbox": "default",
        "verbosity": "normal",
        "plan_mode": "default",
        "unsupported_option_policy": "warn"
      },
      "runners": {
        "codex": { "sandbox": "disabled" },
        "claude": { "verbosity": "verbose" }
      }
    }
  }
}
```

### Normalized Options

#### `output_format`
- `stream_json` (default) - Newline-delimited JSON required for execution
- `json` - JSON output (not supported for execution)
- `text` - Plain text (not supported for execution)

> **Important:** CueLoop execution requires `stream_json`. Other formats will be rejected.

#### `approval_mode`
- `default` - Use runner defaults
- `auto_edits` - Auto-approve edits only (Claude, Gemini, Cursor)
- `yolo` (default) - Bypass all approvals
- `safe` - Strict safety mode (may cause hangs)

**Safety Warning:** `yolo` mode bypasses all approval prompts, allowing the runner to make changes without confirmation. Use with extreme caution.

#### `sandbox`
- `default` - Runner-specific default behavior
- `enabled` - Enable sandbox (Codex, Gemini, Cursor, Pi)
- `disabled` - Disable sandbox

#### `verbosity`
- `quiet` - Minimal output
- `normal` (default) - Standard output
- `verbose` - Detailed output (Claude only)

#### `plan_mode`
- `default` - Do not request runner-native plan mode
- `enabled` / `disabled` - Unsupported for Cursor SDK runs and ignored or rejected for other runners according to `unsupported_option_policy`

Cursor SDK plan mode is intentionally unsupported because it prevents Cursor from writing CueLoop's expected plan/cache artifacts. This does not disable CueLoop's own planning phase.

#### `unsupported_option_policy`
- `ignore` - Silently ignore unsupported options
- `warn` (default) - Log warning and continue
- `error` - Fail if unsupported options are requested

### Runner-Specific Mappings

| Normalized Option | Codex | Claude | Gemini | Cursor | Kimi | Pi |
|-------------------|-------|--------|--------|--------|------|-----|
| `approval_mode=yolo` | *see note | `--permission-mode bypassPermissions` | `--approval-mode yolo` | SDK default posture, no `local.force` mapping | `--yolo` | `--print` |
| `approval_mode=auto_edits` | *see note | `--permission-mode acceptEdits` | `--approval-mode auto_edit` | (not mapped) | `--yolo` | `--print` |
| `sandbox=enabled` | `--sandbox workspace-write` | (not supported) | `--sandbox` | `local.sandboxOptions.enabled` = `true` via SDK (no Cursor agent CLI flags) | (not supported) | `--sandbox` |
| `sandbox=disabled` | `--dangerously-bypass-approvals-and-sandbox` | (not supported) | (not mapped) | `local.sandboxOptions.enabled` = `false` via SDK (no Cursor agent CLI flags) | (not supported) | (not mapped) |

*Codex approval mode is controlled via `~/.codex/config.json`, not CLI flags.

## Session Management

### Explicit Sessions

CueLoop manages runner sessions explicitly for reliable crash recovery. Each phase generates a unique session ID at phase start.

**Session ID Format:**
```
{task_id}-p{phase}-{timestamp}
```

**Example:** `RQ-0001-p2-1704153600`

- `task_id` - The task identifier (e.g., `RQ-0001`)
- `phase` - Phase number (1, 2, or 3)
- `timestamp` - Unix epoch seconds

> **Note:** No `ralph-` prefix, no PID suffix. The same session ID is reused for all continue/resume operations within a phase.

### Kimi Session Handling

Kimi requires special session management because it doesn't emit session IDs in JSON output:

1. CueLoop generates and passes the session ID via `--session` flag
2. Kimi stores session state internally
3. On resume, CueLoop uses the same session ID format

### Session Timeout

Configure session timeout for crash recovery:

```json
{
  "agent": {
    "session_timeout_hours": 24
  }
}
```

Sessions older than this threshold are considered stale and require explicit user confirmation to resume.

## Runner Retry

CueLoop provides configurable retry behavior for transient runner failures.

### Configuration

```json
{
  "agent": {
    "runner_retry": {
      "max_attempts": 3,
      "base_backoff_ms": 1000,
      "multiplier": 2.0,
      "max_backoff_ms": 30000,
      "jitter_ratio": 0.2
    }
  }
}
```

### Retry Classification

**Retryable (automatic retry):**
- Rate limits (HTTP 429)
- Temporary unavailability (HTTP 503)
- Transient I/O errors (connection reset, timeout)
- Timeouts

**Requires User Input (no retry):**
- Authentication failures (HTTP 401)
- Missing binaries

**Non-Retryable (no retry):**
- Invalid invocations
- Fatal exits
- Interruptions (Ctrl+C)

### Retry Conditions

Retries only occur when:
- The repository is clean, OR
- Only CueLoop-allowed paths (`.ralph/`) are dirty, OR
- `git_revert_mode` is `enabled` for auto-revert

### Disabling Retry

To disable retry entirely:

```json
{
  "agent": {
    "runner_retry": {
      "max_attempts": 1
    }
  }
}
```

## Runner Output Parsing

### NDJSON Format

CueLoop requires runners to emit **newline-delimited JSON (NDJSON)** objects. Each line is a separate JSON event.

**Example NDJSON stream:**
```json
{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"Analyzing..."}]}}
{"type":"tool_use","tool_name":"read_file","parameters":{"path":"src/main.rs"}}
{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"Done!"}]}}
```

### Response Extraction

Each runner has a specialized parser that extracts the final assistant response:

| Runner | JSON Pattern | Extracted Content |
|--------|--------------|-------------------|
| **Claude** | `type="assistant"` | `message.content[].text` |
| **Codex** | `type="item.completed"` with `item.type="agent_message"` | `item.text` |
| **Gemini** | `type="message"` with `role="assistant"` | `content` (string or array) |
| **OpenCode** | `type="text"` | Accumulated streaming `part.text` |
| **Kimi** | `role="assistant"` | `content[].text` |
| **Pi** | `type="result"` | `result` field |
| **Cursor** | Primary: `type="assistant"` (streaming `message.content`); legacy: `type="message_end"`; terminal: `type="result"` replaces streamed assistant text when present | `message.content` or `result` string |

### Tool Call Tracking

CueLoop tracks tool calls for display:
- Tool invocations with parameters
- Tool results with status
- Permission denials

Example formatted output:
```
🔧 read_file(path=src/main.rs)
🔧 read_file(completed)
```

## Adding Custom Runners via Plugins

CueLoop supports custom runners through a plugin system.

### Plugin Protocol

Custom runner plugins must implement this CLI protocol:

**Run:**
```bash
<bin> run --model <id> --output-format stream-json [--session <id>]
# Reads prompt from stdin
```

**Resume:**
```bash
<bin> resume --session <id> --model <id> --output-format stream-json <message>
```

**Environment Variables:**
- `RALPH_PLUGIN_ID` - Plugin identifier
- `RALPH_PLUGIN_CONFIG_JSON` - Opaque plugin configuration
- `RALPH_RUNNER_CLI_JSON` - Resolved normalized CLI options

### Plugin Configuration

Enable and configure a plugin:

```json
{
  "plugins": {
    "plugins": {
      "my.custom-runner": {
        "enabled": true,
        "runner": {
          "bin": "custom-runner"
        },
        "config": {
          "api_key": "secret",
          "endpoint": "https://api.example.com"
        }
      }
    }
  }
}
```

### Plugin Manifest

Plugin manifests are located at:
- Project: `.ralph/plugins/<plugin_id>/plugin.json`
- Global: `~/.config/ralph/plugins/<plugin_id>/plugin.json`

### Security Warning

> **Plugins are NOT sandboxed.** Enabling a plugin is equivalent to trusting it with full system access. Only enable plugins from trusted sources.

### Plugin Commands

```bash
# List discovered plugins
cueloop plugin list

# Validate plugin manifests
cueloop plugin validate

# Install a plugin
cueloop plugin install <path> --scope project|global

# Uninstall a plugin
cueloop plugin uninstall <id> --scope project|global
```

See [Plugin Development Guide](../plugin-development.md) for creating custom plugins.

## Practical Examples

### Example 1: Basic Runner Configuration

```json
{
  "version": 1,
  "agent": {
    "runner": "claude",
    "model": "sonnet",
    "phases": 3
  }
}
```

### Example 2: Multi-Phase with Different Runners

```json
{
  "version": 1,
  "agent": {
    "runner": "codex",
    "model": "gpt-5.4",
    "phase_overrides": {
      "phase1": {
        "runner": "codex",
        "model": "gpt-5.4",
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

### Example 3: CLI Override Examples

```bash
# Use specific runner and model
cueloop run one --runner claude --model opus

# Run with YOLO mode disabled
cueloop run one --approval-mode safe

# Single-phase quick execution
cueloop run one --phases 1 --runner codex --model gpt-5.4 --effort low

# Use custom model with OpenCode
cueloop task "Add tests" --runner opencode --model custom/model-v2
```

### Example 4: Session Recovery

```bash
# Run a task (session automatically created)
cueloop run one

# If interrupted, resume from the same session
# CueLoop automatically detects and offers to resume stale sessions
cueloop run one

# Or specify a specific session to resume
# (Handled internally by CueLoop's session management)
```

### Example 5: Retry Configuration for API Rate Limits

```json
{
  "version": 1,
  "agent": {
    "runner": "codex",
    "model": "gpt-5.4",
    "runner_retry": {
      "max_attempts": 5,
      "base_backoff_ms": 2000,
      "multiplier": 2.0,
      "max_backoff_ms": 60000,
      "jitter_ratio": 0.3
    }
  }
}
```

## Troubleshooting

### Runner Binary Not Found

```
Error: runner binary not found: claude
```

**Solution:** Ensure the runner binary is installed and on your PATH, or configure the binary path:

```json
{
  "agent": {
    "claude_bin": "/usr/local/bin/claude"
  }
}
```

### Model Not Supported

```
Error: model custom-model is not supported for codex runner
```

**Solution:** Use a supported model for Codex, or switch to a runner that supports arbitrary model IDs (Claude, OpenCode, Gemini, Kimi, Pi).

### Session Timeout

```
Warning: Session is older than 24 hours. Confirm to resume (y/n):
```

**Solution:** Either confirm the resume or increase `session_timeout_hours` in config.

### Output Format Error

```
Error: runner_cli.output_format=Text is not supported for execution
```

**Solution:** Set `runner_cli.defaults.output_format` to `stream_json`.

## Related Documentation

- [Configuration](../configuration.md) - Full configuration reference
- [Plugin Development](../plugin-development.md) - Creating custom runners
- [Workflow](../workflow.md) - Three-phase execution model
- [Queue and Tasks](../queue-and-tasks.md) - Task management
