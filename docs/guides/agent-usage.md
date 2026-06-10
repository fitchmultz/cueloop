# Agent Usage Guide
Status: Active
Owner: Maintainers
Source of truth: this document for agent-oriented CueLoop usage
Parent: [CueLoop Documentation](../index.md)

CueLoop has different defaults for human operators and already-running coding agents.

Humans can use CueLoop to start and supervise agent runners. An active coding agent should usually use CueLoop as a durable, repo-local task ledger while doing the work itself.

## Core rule

- Human operator path: create tasks, then optionally run agents with `cueloop run ...`.
- Agent ledger path: read or update queue state through `cueloop agent ...` or compact `cueloop machine ...` JSON, then implement and verify in the current session.

Agents should not spawn nested runners unless the user explicitly asks for that workflow.

## Already-running agent quick path

Use this when the current agent session is doing the work and CueLoop is only tracking durable task state:

```bash
cueloop agent overview --format json
cueloop agent next --with-title
cueloop agent show CL-0001 --format json
cueloop agent claim CL-0001 --owner "$USER-session" --ttl-minutes 120
cueloop agent start CL-0001 --note "Started in current agent session"
# do the work with current tools
cueloop agent note CL-0001 "Found root cause in queue validation"
cueloop agent evidence CL-0001 "cargo test -p cueloop machine_contract_test passed"
cueloop agent handoff CL-0001 --next "Run make agent-ci" --format json
cueloop agent complete CL-0001 --evidence "make agent-ci passed"
cueloop agent validate
```

`cueloop agent complete` requires explicit `--evidence` and archives the task through the same done-archive path used by human lifecycle commands.

## Agent ledger commands

These commands are designed for active agents and do not invoke runner CLIs:

| Need | Command |
| --- | --- |
| Compact queue context | `cueloop agent overview --format json` |
| Next runnable task | `cueloop agent next --with-title` |
| Show one task | `cueloop agent show CL-0001 --format json` |
| Claim or release work | `cueloop agent claim CL-0001 --owner <session>`, `cueloop agent release CL-0001` |
| Start work | `cueloop agent start CL-0001 --note "Started"` |
| Append working note | `cueloop agent note CL-0001 "..."` |
| Append verification evidence | `cueloop agent evidence CL-0001 "make test passed"` |
| Append a plan item | `cueloop agent plan-append CL-0001 "Run targeted tests"` |
| Create a handoff packet | `cueloop agent handoff CL-0001 --format json` |
| Complete with evidence | `cueloop agent complete CL-0001 --evidence "make agent-ci passed"` |
| Reject with reason | `cueloop agent reject CL-0001 --reason "Duplicate"` |
| Validate queue/done state | `cueloop agent validate --format json` |

Claims are stored as task `custom_fields` (`agent_claim_owner`, `agent_claimed_at`, and optional `agent_claim_expires_at`) so they remain visible in the normal queue file. Claims coordinate external agents; they do not start, stop, or supervise runners.

## Machine-safe commands

Use machine commands when a stable JSON contract or structured error document matters more than the human-oriented `agent` convenience text.

Read-only context:

```bash
cueloop machine workspace overview
cueloop machine queue read --active-only
cueloop machine queue read --done-limit 5
cueloop machine queue validate
cueloop machine config resolve
cueloop machine doctor report
```

Task lookup and lifecycle:

```bash
cueloop machine task show CL-0001
cueloop machine task start CL-0001 --note "Started by current agent"
cueloop machine task status CL-0001 todo --note "Returned to backlog because ..."
cueloop machine task done CL-0001 --evidence "Verified with make agent-ci"
cueloop machine task reject CL-0001 --note "Rejected because ..."
```

Structured task creation and shaping:

```bash
cueloop machine task insert --dry-run --input task-insert.json
cueloop machine task insert --input task-insert.json
cueloop machine task mutate --dry-run --input task-mutate.json
cueloop machine task mutate --input task-mutate.json
```

Prefer `machine task insert` for fully-shaped agent-created tasks because it supports local keys, dependency mapping, notes, evidence, plan, parent/relationship fields, custom fields, and atomic insertion under the queue lock.

Minimal `task-insert.json`:

```json
{
  "version": 1,
  "tasks": [
    {
      "key": "docs-agent-ledger",
      "title": "Document agent ledger workflow",
      "status": "todo",
      "priority": "medium",
      "tags": ["docs", "agents"],
      "scope": ["docs/guides/agent-usage.md"],
      "plan": ["Update guide", "Run docs validation"],
      "evidence": ["make ci-docs passes"]
    }
  ]
}
```

Minimal append mutation:

```json
{
  "version": 1,
  "tasks": [
    {
      "task_id": "CL-0001",
      "edits": [
        {"field": "notes", "mode": "append", "value": "Root cause found"},
        {"field": "evidence", "mode": "append", "value": "cargo test passed"}
      ]
    }
  ]
}
```

Use `mode: "set"` or omit `mode` to replace a field. Mutation requests reject unknown fields and unsupported versions.

Follow-up proposals:

```bash
cueloop machine task followups apply --task CL-0001 --dry-run
cueloop machine task followups apply --task CL-0001
```

Recovery and diagnostics:

```bash
cueloop machine queue validate
cueloop machine queue repair --dry-run
cueloop machine queue repair
cueloop machine queue unlock-inspect
cueloop machine queue undo --dry-run
cueloop machine doctor report
```

## Runner-backed commands

These commands can invoke external agent runners or planner workflows and are not the default path for an already-running agent:

```bash
cueloop run ...
cueloop task build ...
cueloop task decompose ...
cueloop task update ...
cueloop scan ...
cueloop machine task build ...
cueloop machine task decompose ...
```

`cueloop machine task create` is a simple queue append when the JSON request omits `template`. If the request includes `template`, it uses the task-builder runner. Use it only when that behavior is intentional; otherwise prefer `machine task insert`.

## Relationship rule for agents

Use `depends_on` for canonical execution constraints. `blocks` is the inverse relationship and now also affects runnability: if task A lists B in `blocks`, B is not runnable until A is `done` or `rejected`. Prefer keeping both directions consistent when you manually shape relationship graphs.

## When agents should use CueLoop

Use CueLoop when the task involves:

- a task ID or active queue item
- durable multi-session state
- dependencies or blockers
- lifecycle status changes
- notes, evidence, claims, or handoff state
- follow-ups or queue shaping
- queue validation or repair

Do not use CueLoop merely because `.cueloop/` exists.

## When agents should skip CueLoop

Skip CueLoop when the work is:

- a normal one-turn edit with no queue relevance
- ordinary code search, implementation, or test execution
- likely to store secrets or raw sensitive logs in task text

## Mutation invariant

After every queue mutation:

1. Run `cueloop agent validate` or `cueloop machine queue validate`.
2. Re-read `cueloop agent overview --format json`, `cueloop machine queue read --active-only`, or `cueloop machine workspace overview`.
3. Confirm task IDs, status, dependencies, blockers, evidence, claims, and continuation next steps.
4. If invalid, inspect machine recovery output before applying repair or undo.
