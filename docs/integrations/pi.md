# Pi Coding Agent Integration
Status: Active
Owner: Maintainers
Source of truth: this document for its stated scope
Parent: [Feature Documentation](../features/README.md)

Purpose: Document how CueLoop integrates with the Pi Coding Agent CLI, including version expectations and session semantics.

## Tested Version

CueLoop currently targets Pi **0.79.10** or newer for `agent.runner = "pi"`. Keep this document and the bridge package metadata in sync when the Pi floor changes.

## CLI Integration

CueLoop invokes Pi in JSON mode:

```bash
pi --mode json [--session-id <id>|--session <path>] [--print] [--sandbox] [--model <id>] [--thinking <level>] "<prompt>"
```

### Session flags (Pi 0.76+)

| Flag | When CueLoop uses it |
|------|----------------------|
| `--session-id <id>` | Resume or create an exact project-local session by ID |
| `--session <path>` | When the stored session identifier is already a session file path |

Resume and run share the same command shape. CueLoop passes the continue message as the trailing prompt argument.

### Process wrapper

When Pi is installed as a Node entrypoint, CueLoop launches it through a small wrapper so inherited process titles cannot leak secrets. Native Pi binaries are invoked directly.

## Stream Parsing

CueLoop understands Pi JSON envelopes such as:

- `{"type":"result","result":"..."}` for final assistant text
- `{"type":"session","id":"..."}` for session lifecycle IDs
- `message_update`, `message_end`, and `tool_execution_start` for live terminal rendering

See `crates/cueloop/src/runner/execution/stream_events/pi.rs` for display-line extraction.

## Configuration

```json
{
  "agent": {
    "runner": "pi",
    "model": "gpt-5.3",
    "pi_bin": "pi"
  }
}
```

Override `pi_bin` when Pi is not on `PATH`.

## Continue / Resume Behavior

When a stored Pi session ID cannot be resumed, CueLoop falls back to a fresh invocation with a new CueLoop-managed `--session-id` and records an explicit resume decision event. Fresh fallback is triggered by Pi runtime errors such as `session not found`.
