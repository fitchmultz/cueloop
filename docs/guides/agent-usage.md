# Agent Usage Guide
Status: Active
Owner: Maintainers
Source of truth: this document for agent-oriented CueLoop usage
Parent: [CueLoop Documentation](../index.md)

For request/response JSON shapes and version fields, use [Machine Contract](../machine-contract.md). For how machine create/build relate to human `task` commands, see [Task Operations](../features/task-operations.md#machine-task-create-and-build-agents-and-automation).

CueLoop has different defaults for human operators and already-running coding agents.

Humans usually use CueLoop to start and supervise agent runners. An active coding agent should instead use CueLoop as a durable, repo-local task ledger while doing the work itself.

## Core rule

- Human operator path: create tasks, then run agents with `cueloop run ...`.
- Agent path: read or update queue state through `cueloop machine ...`, then implement and verify in the current session.

Agents should not spawn nested runners unless the user explicitly asks for that workflow.

## Agent-safe commands

Use machine commands for stable JSON and structured errors:

```bash
cueloop machine workspace overview
cueloop machine queue read
cueloop machine queue validate
cueloop machine config resolve
cueloop machine doctor report
```

Task lookup and lifecycle:

```bash
cueloop machine task show RQ-0001
cueloop machine task start RQ-0001 --note "Started by current agent"
cueloop machine task status RQ-0001 todo --note "Returned to backlog because ..."
cueloop machine task done RQ-0001 --note "Verified with make agent-ci"
cueloop machine task reject RQ-0001 --note "Rejected because ..."
```

Structured task creation (append one `todo` task with queue lock and undo; optional `template` instead runs the task-builder runner and must yield exactly one task; stdout is `MachineTaskCreateDocument`). Omit `--input` to read the JSON request from stdin instead of a file.

```bash
cueloop machine task create --input task-create.json
```

AI-assisted task drafting via the task-builder runner (stdout is `MachineTaskBuildDocument` only; same runner stack as `cueloop task build`). Same stdin rule as create when `--input` is omitted.

```bash
cueloop machine task build --input task-build-request.json
# Optional overrides mirror other machine runner surfaces, e.g. --runner, --model, --effort
```

Atomic queue shaping:

```bash
cueloop machine task insert --dry-run --input task-insert.json
cueloop machine task insert --input task-insert.json
cueloop machine task mutate --dry-run --input task-mutate.json
cueloop machine task mutate --input task-mutate.json
```

Follow-up proposals:

```bash
cueloop machine task followups apply --task RQ-0001 --dry-run
cueloop machine task followups apply --task RQ-0001
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

## When agents should use CueLoop

Use CueLoop when the task involves:

- a task ID or active queue item
- durable multi-session state
- dependencies or blockers
- lifecycle status changes
- follow-ups or handoff state
- queue validation or repair

Do not use CueLoop merely because `.cueloop/` exists.

## When agents should skip CueLoop

Skip CueLoop when the work is:

- a normal one-turn edit with no queue relevance
- ordinary code search, implementation, or test execution
- likely to store secrets or raw sensitive logs in task text

## Nested-runner commands

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

`cueloop machine task create` uses the task-builder runner only when the JSON request includes `template`; omit `template` for a pure queue append with no runner.

Use them only when the user explicitly asks CueLoop to run or plan work through another runner.

## Mutation invariant

After every queue mutation:

1. Run `cueloop machine queue validate`.
2. Re-read `cueloop machine queue read` or `cueloop machine workspace overview`.
3. Confirm task IDs, status, dependencies, blockers, and continuation next steps.
4. If invalid, inspect machine recovery output before applying repair or undo.
