# Machine Contract

Ralph exposes a first-class machine API under `ralph machine ...`.

This surface exists for the macOS app and any other automation that needs stable, versioned JSON instead of human-oriented CLI behavior.

## Rules

- Every machine response is a named JSON document with a top-level `version`.
- Breaking wire changes require a version bump for the affected machine document.
- Human CLI output and flags may change without preserving app compatibility.
- Machine run streams emit NDJSON on stdout.
- Machine run terminal summaries are single-line JSON documents so stream consumers can parse them deterministically.
- Machine clients should consume structured resume payloads instead of scraping prose from stderr/stdout.

## Current Machine Areas

- `ralph machine system info`
- `ralph machine queue read`
- `ralph machine queue graph`
- `ralph machine queue dashboard`
- `ralph machine queue validate`
- `ralph machine config resolve`
- `ralph machine task create`
- `ralph machine task mutate`
- `ralph machine task decompose`
- `ralph machine run one`
- `ralph machine run loop`
- `ralph machine run parallel-status`
- `ralph machine doctor report`
- `ralph machine cli-spec`
- `ralph machine schema`

## Important versioned documents

### `machine config resolve` (`version: 3`)

Includes:
- resolved queue/config paths
- safety summary
- resolved config
- optional `resume_preview`

`resume_preview` is the app/automation preflight signal for whether the next run would:
- resume the same session
- fall back to a fresh invocation
- refuse to resume

### `machine run` events (`version: 3`)

The NDJSON stream can emit both resume and progress-blocking state transitions.

Resume decisions remain structured:

```json
{
  "version": 3,
  "kind": "resume_decision",
  "task_id": "RQ-0001",
  "message": "Resume: continuing the interrupted session for task RQ-0001.",
  "payload": {
    "status": "resuming_same_session",
    "scope": "run_session",
    "reason": "session_valid",
    "task_id": "RQ-0001",
    "message": "Resume: continuing the interrupted session for task RQ-0001.",
    "detail": "Saved session is current and will resume from phase 2 with 1 completed loop task(s)."
  }
}
```

Blocking-state transitions are also structured:

```json
{
  "version": 3,
  "kind": "blocked_state_changed",
  "message": "Ralph is blocked by unfinished dependencies.",
  "payload": {
    "status": "blocked",
    "reason": {
      "kind": "dependency_blocked",
      "blocked_tasks": 2
    },
    "task_id": null,
    "message": "Ralph is blocked by unfinished dependencies.",
    "detail": "2 candidate task(s) are waiting on dependency completion."
  }
}
```

`kind: "blocked_state_cleared"` indicates that Ralph resumed forward progress.

### `machine run` summaries (`version: 2`)

Terminal summaries now include optional `blocking` state for `no_candidates`, `blocked`, and classified stalled failures.

### `machine queue read`

`runnability.summary.blocking` is the queue/read-side source of truth for why the queue is idle, dependency-blocked, schedule-blocked, or mixed.

Together, those payloads are the source of truth for live operator-state UI.

## Schemas

Generated machine schemas live in [schemas/machine.schema.json](../schemas/machine.schema.json).

Generate them locally with:

```bash
make generate
```

## App Contract Boundary

The macOS app should consume only machine surfaces for:

- queue snapshots
- config resolution
- task create/mutate/decompose flows
- graph and dashboard reads
- diagnostics consumed by the app
- run status and event streaming
- CLI spec loading
- resume preview / resume decision state

It should not infer app state from human CLI text, hidden commands, or direct queue-file decoding.
