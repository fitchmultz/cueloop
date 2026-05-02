# Configuration: Notifications and Webhooks
Status: Active
Owner: Maintainers
Source of truth: this document for desktop notification and webhook configuration
Parent: [Configuration](../configuration.md)

Purpose: Document CueLoop's local notification and outbound webhook settings, delivery semantics, and safety constraints.

## Notification Configuration

`agent.notification` controls desktop notifications for task completion and failures.

Supported fields:
- `enabled`: legacy field, enable/disable all notifications (default: `true`).
- `notify_on_complete`: enable notifications on task completion (default: `true`).
- `notify_on_fail`: enable notifications on task failure (default: `true`).
- `notify_on_loop_complete`: enable notifications when loop mode finishes (default: `true`).
- `notify_on_watch_new_tasks`: enable notifications when watch mode adds new tasks from comments (default: `true`).
- `suppress_when_active`: suppress notifications when the macOS app is active (default: `true`).
- `sound_enabled`: play sound with notification (default: `false`).
- `sound_path`: custom sound file path (optional, platform-specific).
- `timeout_ms`: notification display duration in milliseconds (default: `8000`, range: `1000-60000`).

Platform notes:
- **macOS**: Uses NotificationCenter; sound plays via `afplay` (default: `/System/Library/Sounds/Glass.aiff`).
- **Linux**: Uses D-Bus/notify-send; sound plays via `paplay`/`aplay` or `canberra-gtk-play` for default sounds.
- **Windows**: Uses toast notifications; custom sounds are `.wav`-only and play via `winmm.dll` PlaySound.

Example:

```json
{
  "version": 2,
  "agent": {
    "notification": {
      "enabled": true,
      "notify_on_complete": true,
      "notify_on_fail": true,
      "notify_on_loop_complete": true,
      "notify_on_watch_new_tasks": true,
      "suppress_when_active": true,
      "sound_enabled": true,
      "timeout_ms": 10000
    }
  }
}
```

CLI overrides:
- `--notify`: Enable notification on task completion (overrides config).
- `--no-notify`: Disable notification on task completion (overrides config).
- `--notify-fail`: Enable notification on task failure (overrides config).
- `--no-notify-fail`: Disable notification on task failure (overrides config).
- `--notify-sound`: Enable sound for this run (works with notification flags or when enabled in config).

## Webhook Configuration

![Webhooks](../assets/images/2026-02-07-webhooks.png)

`agent.webhook` controls HTTP webhook notifications for task events. Webhooks complement desktop notifications by enabling external integrations (Slack, Discord, CI systems, etc.) to receive real-time task events.

Supported fields:
- `enabled`: enable webhook notifications (default: `false`).
- `url`: webhook endpoint URL (required when enabled).
- `allow_insecure_http`: when `true`, allow `http://` URLs (default: `false` / unset; HTTPS-only).
- `allow_private_targets`: when `true`, allow loopback, link-local, and common cloud-metadata hostnames/IPs (default: `false` / unset).
- `secret`: secret key for HMAC-SHA256 signature generation (optional).
  When set, webhooks include an `X-CueLoop-Signature` header for verification.
- `events`: list of events to subscribe to (default: legacy task events only).
  - **Task events**: `task_created`, `task_started`, `task_completed`, `task_failed`, `task_status_changed`
  - **Loop events**: `loop_started`, `loop_stopped` (opt-in)
  - **Phase events**: `phase_started`, `phase_completed` (opt-in)
  - **Queue event**: `queue_unblocked` (opt-in)
  - Use `["*"]` to subscribe to all events including new ones
- `timeout_secs`: request timeout in seconds (default: `30`, max: `300`).
- `retry_count`: number of retry attempts for failed deliveries (default: `3`, max: `10`).
- `retry_backoff_ms`: base interval in milliseconds for exponential webhook retry delays (default: `1000`, max: `30000`); delays include bounded jitter and cap at 30 seconds between attempts.
- `queue_capacity`: maximum number of pending webhooks in the delivery queue (default: `500`, range: `10-10000`).
- `parallel_queue_multiplier`: multiplier for effective queue capacity in parallel mode (default: `2.0`, range: `1.0-10.0`).
- `queue_policy`: backpressure policy when queue is full (default: `drop_oldest`).
  - `drop_oldest`: Drop new webhooks when queue is full (preserves existing queue contents).
  - `drop_new`: Drop the new webhook if the queue is full.
  - `block_with_timeout`: Briefly block the caller (100ms), then drop if queue is still full.

URL validation (when `enabled` is `true`): only `http://` and `https://` are accepted; `http://` requires `allow_insecure_http: true`. Loopback, IPv4 link-local (`169.254.0.0/16`), IPv6 link-local, unspecified addresses, and `metadata.google.internal` are rejected unless `allow_private_targets: true`.

### Event Filtering

**Breaking change**: As of this version, new event types (`loop_*`, `phase_*`) are **opt-in** and not enabled by default.

- If `events` is not specified (or `null`): only legacy task events are delivered
- If `events` is `["*"]`: all events are delivered (legacy + new)
- If `events` is an explicit list: only those events are delivered

Example configuration for CI/dashboard integrations:

```json
{
  "agent": {
    "webhook": {
      "enabled": true,
      "url": "https://example.com/webhook",
      "events": ["loop_started", "phase_started", "phase_completed", "loop_stopped"]
    }
  }
}
```

### Delivery Semantics

Webhooks are delivered **asynchronously** by a background worker thread:

- **Best-effort delivery**: Webhooks may be dropped if the queue is full (per `queue_policy`).
- **Non-blocking**: The `send_webhook` call returns immediately after enqueueing.
- **Order preservation**: Webhooks are delivered in FIFO order within the constraints of the backpressure policy.
- **Failure handling**: Failed deliveries are retried (per `retry_count` and `retry_backoff_ms`).
- **Failure persistence**: Final delivery failures are persisted to `.cueloop/cache/webhooks/failures.json` (bounded to the most recent 200 records).
- **Worker lifecycle**: The background worker starts on first webhook send and shuts down when the process exits.

Example:

```json
{
  "version": 2,
  "agent": {
    "webhook": {
      "enabled": true,
      "url": "https://hooks.slack.com/services/T00000000/B00000000/XXXXXXXXXXXXXXXXXXXXXXXX",
      "secret": "my-webhook-secret",
      "events": ["task_completed", "task_failed"],
      "timeout_secs": 30,
      "retry_count": 3,
      "retry_backoff_ms": 1000,
      "queue_capacity": 100,
      "queue_policy": "drop_oldest"
    }
  }
}
```

### Webhook Payload Format

Webhooks are sent as HTTP POST requests with JSON payloads:

```json
{
  "event": "task_completed",
  "timestamp": "2024-01-15T10:30:00Z",
  "task_id": "RQ-0001",
  "task_title": "Add webhook support",
  "previous_status": "doing",
  "current_status": "done",
  "note": null
}
```

**Task events** (`task_*`) always include `task_id` and `task_title`. **Loop events** (`loop_*`) omit these fields since they are not task-specific.

#### Enriched Payloads (Phase Events)

Phase and loop events include additional context metadata:

```json
{
  "event": "phase_completed",
  "timestamp": "2024-01-15T10:30:00Z",
  "task_id": "RQ-0001",
  "task_title": "Add webhook support",
  "runner": "codex",
  "model": "gpt-5.4",
  "phase": 2,
  "phase_count": 3,
  "duration_ms": 12500,
  "repo_root": "/home/user/project",
  "branch": "main",
  "commit": "abc123def456",
  "ci_gate": "passed"
}
```

Optional context fields (only present when applicable):
- `runner`: The runner used (e.g., `claude`, `codex`, `kimi`)
- `model`: The model used for this phase
- `phase`: Phase number (1, 2, or 3)
- `phase_count`: Total configured phases
- `duration_ms`: Phase execution duration in milliseconds
- `repo_root`: Repository root path
- `branch`: Current git branch
- `commit`: Current git commit hash
- `ci_gate`: CI gate outcome (`skipped`, `passed`, or `failed`)

### Webhook Security

When a `secret` is configured, webhooks include an `X-CueLoop-Signature` header:

```
X-CueLoop-Signature: sha256=abc123...
```

The signature is computed as HMAC-SHA256 of the request body using the configured secret.

To verify in Python:

```python
import hmac
import hashlib

secret = b'my-webhook-secret'
body = request.body

expected_signature = 'sha256=' + hmac.new(
    secret, body, hashlib.sha256
).hexdigest()

if not hmac.compare_digest(
    expected_signature,
    request.headers.get('X-CueLoop-Signature', '')
):
    raise ValueError("Invalid signature")
```

### Testing Webhooks

Use the CLI to test your webhook configuration:

```bash
# Test with configured URL
cueloop webhook test

# Test with specific event type
cueloop webhook test --event task_completed

# Test with new event types (opt-in)
cueloop webhook test --event phase_started
cueloop webhook test --event loop_started

# Print the JSON payload without sending (useful for debugging)
cueloop webhook test --event phase_completed --print-json
cueloop webhook test --event task_created --print-json --pretty

# Test with custom URL
cueloop webhook test --url https://example.com/webhook

# Inspect queue/failure diagnostics
cueloop webhook status
cueloop webhook status --format json

# Replay failed deliveries safely
cueloop webhook replay --dry-run --id wf-1700000000-1
cueloop webhook replay --event task_completed --limit 5
```

Non-dry-run replay (`cueloop webhook replay` without `--dry-run`) requires:
- `agent.webhook.enabled: true`
- `agent.webhook.url` set to a non-empty endpoint URL
