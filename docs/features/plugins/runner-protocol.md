# Plugin Runner Protocol
Status: Active
Owner: Maintainers
Source of truth: this document for custom runner plugin invocation and output protocol
Parent: [CueLoop Plugin System](../plugins.md)

Purpose: Define the command-line, environment, streaming-output, and session contracts for runner plugins.

---

## Runner Commands

### Run

```bash
<runner-bin> run --model <model-id> --output-format stream-json [--session <session-id>]
```

- `--model`: model identifier
- `--output-format`: must be `stream-json`
- `--session`: optional session ID for resumable sessions
- stdin: prompt text
- stdout: newline-delimited JSON (NDJSON)

### Resume (Optional)

Required only when `supports_resume` is `true` in the manifest:

```bash
<runner-bin> resume --session <session-id> --model <model-id> --output-format stream-json <message>
```

- `<message>` is the continue/resume text argument.

## Environment Variables

| Variable | Description |
|----------|-------------|
| `RALPH_PLUGIN_ID` | Plugin ID (for example `my.plugin`) |
| `RALPH_PLUGIN_CONFIG_JSON` | Opaque plugin config JSON (`{}` when unset) |
| `RALPH_RUNNER_CLI_JSON` | Resolved runner CLI options |

`RALPH_RUNNER_CLI_JSON` example:

```json
{
  "output_format": "stream_json",
  "verbosity": "normal",
  "approval_mode": "yolo",
  "sandbox": "default",
  "plan_mode": "default",
  "unsupported_option_policy": "warn"
}
```

## Output Contract (NDJSON)

Runners must emit newline-delimited JSON objects. CueLoop parses and displays these output shapes:

**1. Claude format (`type=assistant`)**

```json
{"type": "assistant", "message": {"role": "assistant", "content": [{"type": "text", "text": "Hello"}]}}
```

**2. Kimi format (`role=assistant`)**

```json
{"role": "assistant", "content": [{"type": "text", "text": "Hello"}]}
```

**3. Codex format (`item.completed`)**

```json
{"type": "item.completed", "item": {"type": "agent_message", "text": "Hello"}}
```

**4. Gemini format (`type=message`)**

```json
{"type": "message", "role": "assistant", "content": "Hello"}
```

**5. Text streaming (Opencode)**

```json
{"type": "text", "part": {"text": "Hello "}}
{"type": "text", "part": {"text": "World"}}
```

**6. Tool calls**

```json
{"type": "tool_use", "tool_name": "write", "parameters": {"path": "file.txt", "content": "data"}}
```

**7. Session markers**

```json
{"type": "session", "id": "RQ-0001-p2-1704153600"}
```

## Session IDs

CueLoop extracts session IDs from:

- `id` (when `type` is `session`)
- `thread_id`
- `session_id`
- `sessionID`

Session ID format:

```text
{task_id}-p{phase}-{timestamp}
```

Example: `RQ-0001-p2-1704153600`

## Related Docs

- [Architecture and Manifest](architecture.md)
- [Examples](examples.md)
- [Troubleshooting and Compatibility](troubleshooting.md)
- [Runners Feature](../runners.md)
