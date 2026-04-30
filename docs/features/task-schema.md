# Task Schema and Field Reference
Status: Active
Owner: Maintainers
Source of truth: this document for task object fields, per-task agent overrides, examples, and schema-level validation
Parent: [Task System](tasks.md)

This page defines the shape and validation expectations for Ralph task objects stored in `.ralph/queue.jsonc` and `.ralph/done.jsonc`. It includes the minimum task-bearing queue envelope only; queue operations, ordering, locking, repair, archive, import/export, and migration behavior live in [Queue](queue.md). Relationship field meanings are summarized here as schema fields; relationship behavior and validation semantics live in [Task Relationships](task-relationships.md).

## Related Task Docs

- [Task System](tasks.md) — overview and task documentation index.
- [Task Lifecycle and Priority](task-lifecycle.md) — statuses, lifecycle timestamps, runnability basics, and priority semantics.
- [Task Relationships](task-relationships.md) — dependency, blocking, relation, duplicate, and hierarchy semantics.
- [Task Operations](task-operations.md) — creation, editing, templates, batch operations, and CLI workflows.
- [Queue](queue.md) — queue file operations, ordering, archive, repair, import/export, and locks.

---

## Overview

### What is a Task?

A **Task** in Ralph is a JSON object representing a discrete unit of work. Tasks are stored in `.ralph/queue.jsonc` (active work) or `.ralph/done.jsonc` (completed or rejected work). Each task has:

- **Identity**: Unique ID, title, timestamps
- **State**: Status, priority, tags
- **Context**: Scope, evidence, plan, notes, description
- **Relationships**: Dependencies, blocking, related tasks, hierarchy
- **Execution config**: Per-task runner, model, and phase overrides

### Task as Unit of Work

Tasks serve as the fundamental interface between you and AI agents:

1. **Capture Intent**: The `request` field preserves the original human request
2. **Guide Execution**: Scope, plan, and evidence help agents understand context
3. **Track Progress**: Status transitions provide visibility into work state
4. **Enable Recovery**: Timestamps and relationships support crash recovery and planning

### Minimum Queue Envelope

Task objects live inside Ralph queue files. This page owns the minimum task-bearing envelope shown below; [Queue](queue.md) owns queue file operations, ordering, locking, repair, archive, import/export, and migration behavior.

```
.ralph/
├── queue.jsonc   # Active tasks
├── done.jsonc    # Completed/rejected tasks archive
└── cache/        # Plans, completions, and queue backups
```

**Minimum queue structure:**
```json
{
  "version": 1,
  "tasks": []
}
```

---

## Task Fields

### Required Fields

| Field | Type | Description |
|-------|------|-------------|
| `id` | string | Unique task identifier (e.g., `RQ-0001`) |
| `title` | string | Short, descriptive task title |
| `created_at` | string | RFC3339 UTC timestamp of creation |
| `updated_at` | string | RFC3339 UTC timestamp of last modification |

### Common Optional Fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `description` | string \| null | null | Detailed context, goal, and desired outcome |
| `status` | enum | `todo` | Lifecycle status: `draft`, `todo`, `doing`, `done`, `rejected` |
| `kind` | enum | `work_item` | Actionability: `work_item` is executable atomic work; `group` is a non-runnable decomposition/organization node |
| `priority` | enum | `medium` | Priority level: `critical`, `high`, `medium`, `low` |
| `tags` | string[] | [] | Categorical labels for filtering/grouping |
| `scope` | string[] | [] | Starting points for work (files, paths, commands) |
| `evidence` | string[] | [] | Observed behavior, references, justifications |
| `plan` | string[] | [] | Step-by-step execution plan |
| `notes` | string[] | [] | Working notes, observations, references |
| `request` | string \| null | null | Original human request that created the task |

### Relationship Fields

| Field | Type | Description |
|-------|------|-------------|
| `depends_on` | string[] | Task IDs that must complete before this task can run |
| `blocks` | string[] | Task IDs that are blocked by this task (inverse of depends_on) |
| `relates_to` | string[] | Task IDs with loose semantic coupling (no execution constraint) |
| `duplicates` | string \| null | Task ID this task duplicates (singular reference) |
| `parent_id` | string \| null | Parent task ID for hierarchical organization; does not by itself make a task non-runnable |

### Actionability Semantics

`kind` is the canonical machine-readable execution contract:

- `work_item` tasks are executable when their `status`, dependencies, and schedule allow it.
- `group` tasks organize decomposition trees and are skipped by `ralph queue next`, `run one`, `run loop`, parallel worker selection, and machine runnability by default.
- Group tasks remain visible in queue read/list/search/tree/graph surfaces and app models.
- Missing `kind` defaults to `work_item` for existing queues; queue file `version` remains 1.
- `status` is lifecycle, and `parent_id` is hierarchy. Do not infer actionability from either field.

### Agent Override Fields

| Field | Type | Description |
|-------|------|-------------|
| `agent.runner` | string \| null | Override runner with a built-in runner ID (`codex`, `opencode`, `gemini`, `claude`, `cursor`, `kimi`, `pi`) or plugin runner ID |
| `agent.model` | string \| null | Override model identifier |
| `agent.model_effort` | enum | Override reasoning effort: `default`, `low`, `medium`, `high`, `xhigh` |
| `agent.iterations` | integer \| null | Number of iterations for this task (default: 1) |
| `agent.followup_reasoning_effort` | enum \| null | Reasoning effort for iterations > 1 |
| `agent.runner_cli` | object | Normalized CLI overrides (approval_mode, sandbox, etc.) |

### Scheduling Fields

| Field | Type | Description |
|-------|------|-------------|
| `started_at` | string \| null | RFC3339 UTC when work actually started |
| `completed_at` | string \| null | RFC3339 UTC when task was done/rejected |
| `scheduled_start` | string \| null | RFC3339 UTC when task should become runnable |

### Custom Fields

| Field | Type | Description |
|-------|------|-------------|
| `custom_fields` | object | User-defined key-value pairs (values coerced to strings) |

**Custom Field Constraints:**
- Keys must not contain whitespace
- Values may be string, number, or boolean (coerced to strings on load)
- Arrays and objects are not allowed as values
- Reserved analytics keys: `runner_used`, `model_used` (auto-populated on completion)

---

## Per-Task Agent Configuration

The `agent` field allows overriding global configuration for individual tasks.

### Configuration Precedence (Highest to Lowest)

1. Per-task `agent` field in task
2. Project config (`.ralph/config.jsonc`)
3. Global config (`~/.config/ralph/config.jsonc`)
4. Schema defaults

### Override Fields

```json
{
  "id": "RQ-0001",
  "title": "Complex refactoring task",
  "agent": {
    "runner": "codex",
    "model": "gpt-5.3-codex",
    "model_effort": "high",
    "iterations": 2,
    "followup_reasoning_effort": "low",
    "runner_cli": {
      "approval_mode": "auto_edits",
      "sandbox": "enabled"
    }
  }
}
```

### Field Reference

| Field | Values | Description |
|-------|--------|-------------|
| `runner` | Built-in runner ID or plugin runner ID | Which AI runner to use |
| `model` | model identifier string | Specific model version |
| `model_effort` | `default`, `low`, `medium`, `high`, `xhigh` | Reasoning effort (Codex and Pi only) |
| `iterations` | integer ≥ 1 | Number of execution iterations |
| `followup_reasoning_effort` | `low`, `medium`, `high`, `xhigh` | Effort for iterations > 1 |

### Runner CLI Overrides

```json
{
  "agent": {
    "runner_cli": {
      "approval_mode": "yolo",        // "default", "auto_edits", "yolo", "safe"
      "output_format": "stream_json",  // "stream_json", "json", "text"
      "plan_mode": "disabled",        // "default", "enabled", "disabled"
      "sandbox": "enabled",           // "default", "enabled", "disabled"
      "verbosity": "verbose",          // "quiet", "normal", "verbose"
      "unsupported_option_policy": "warn"  // "ignore", "warn", "error"
    }
  }
}
```

### Override Behavior Notes

**INTENDED BEHAVIOR:**
- `agent.model_effort: default` falls back to config's `agent.reasoning_effort`
- `agent.followup_reasoning_effort` is used by Codex and Pi runners and ignored by runners without reasoning-effort support
- CLI overrides should merge with config, with CLI taking precedence

**CURRENTLY IMPLEMENTED BEHAVIOR:**
- Overrides are resolved at task execution time
- Some runners may not support all CLI options (handled per `unsupported_option_policy`)
- `approval_mode=safe` fails fast in non-interactive contexts (task building/updating)

---

## Task Validation

### Validation Levels

| Level | Behavior | Examples |
|-------|----------|----------|
| **Hard Errors** | Block queue operations | Invalid IDs, missing required fields, circular dependencies |
| **Warnings** | Logged but non-blocking | Deep dependency chains, dependency on rejected task |

### Hard Error Conditions

#### ID Validation
- Empty ID
- Missing `-` separator (must be `PREFIX-NUMBER`)
- Wrong prefix (must match config `id_prefix`)
- Wrong width (must match config `id_width`)
- Non-digit characters in numeric suffix
- Duplicate IDs (within queue or across queue/done)

#### Required Fields
- Missing `id`
- Missing `title` (or empty)
- Missing `created_at`
- Missing `updated_at`
- Missing `completed_at` when status is `done` or `rejected`

#### Timestamp Validation
- Invalid RFC3339 format
- Non-UTC timestamps (must end in `Z`)

#### List Field Validation
- Empty strings within lists (e.g., `["a", "", "b"]`)

#### Custom Field Validation
- Empty keys
- Keys containing whitespace
- Non-scalar values (arrays, objects, null)

#### Dependency Validation
- Self-dependency (`depends_on` contains own ID)
- Missing dependency (target doesn't exist)
- Circular dependency (cycles in `depends_on` graph)

#### Relationship Validation
- Self-blocking, self-relation, self-duplication
- Missing target task
- Circular blocking relationships

### Warning Conditions

| Warning | Trigger |
|---------|---------|
| Dependency on rejected task | Task depends on a `rejected` task |
| Deep dependency chain | Chain depth exceeds `queue.max_dependency_depth` (default: 10) |
| Blocked execution path | All dependency paths lead to incomplete/rejected tasks |
| Duplicate of done/rejected task | `duplicates` points to terminal task |
| Missing parent | `parent_id` references non-existent task |
| Self-parent | Task references itself as parent |
| Circular parent chain | Cycle in `parent_id` hierarchy |

### Running Validation

```bash
# Validate queue
ralph queue validate

# Validation runs automatically on most queue operations
ralph task edit status done RQ-0001  # Validates after edit
```

### Configuration

```json
{
  "queue": {
    "max_dependency_depth": 15
  }
}
```

---

## Complete Task Examples

### Basic Task

```json
{
  "id": "RQ-0001",
  "title": "Add user authentication",
  "description": "Implement JWT-based authentication for the API",
  "status": "todo",
  "priority": "high",
  "created_at": "2026-01-15T10:00:00Z",
  "updated_at": "2026-01-15T10:00:00Z",
  "tags": ["api", "auth", "security"],
  "scope": ["src/auth.rs", "src/middleware/"],
  "evidence": ["API spec v2.1"],
  "plan": [
    "Design JWT token structure",
    "Implement token generation",
    "Add authentication middleware",
    "Write tests"
  ],
  "notes": [],
  "request": "Add JWT authentication to protect API endpoints"
}
```

### Task with Dependencies

```json
{
  "id": "RQ-0003",
  "title": "Implement login endpoint",
  "status": "todo",
  "priority": "high",
  "created_at": "2026-01-15T10:30:00Z",
  "updated_at": "2026-01-15T10:30:00Z",
  "depends_on": ["RQ-0001", "RQ-0002"],
  "tags": ["api", "endpoint"],
  "scope": ["src/routes/login.rs"]
}
```

### Task with Agent Overrides

```json
{
  "id": "RQ-0005",
  "title": "Complex algorithm optimization",
  "status": "todo",
  "priority": "critical",
  "created_at": "2026-01-15T11:00:00Z",
  "updated_at": "2026-01-15T11:00:00Z",
  "agent": {
    "runner": "codex",
    "model": "gpt-5.3-codex",
    "model_effort": "xhigh",
    "iterations": 3,
    "followup_reasoning_effort": "high",
    "runner_cli": {
      "approval_mode": "auto_edits",
      "sandbox": "enabled"
    }
  },
  "tags": ["performance", "algorithm"],
  "scope": ["src/optimizer.rs"]
}
```

### Task Hierarchy

```json
{
  "id": "RQ-0010",
  "title": "Implement feature X",
  "status": "doing",
  "priority": "high",
  "created_at": "2026-01-15T12:00:00Z",
  "updated_at": "2026-01-15T14:00:00Z",
  "started_at": "2026-01-15T14:00:00Z",
  "tags": ["epic", "feature-x"]
}
```

```json
{
  "id": "RQ-0011",
  "title": "Implement feature X - Backend API",
  "status": "todo",
  "priority": "high",
  "parent_id": "RQ-0010",
  "created_at": "2026-01-15T12:30:00Z",
  "updated_at": "2026-01-15T12:30:00Z",
  "depends_on": ["RQ-0001"]
}
```

```json
{
  "id": "RQ-0012",
  "title": "Implement feature X - Frontend UI",
  "status": "todo",
  "priority": "medium",
  "parent_id": "RQ-0010",
  "created_at": "2026-01-15T12:30:00Z",
  "updated_at": "2026-01-15T12:30:00Z",
  "depends_on": ["RQ-0011"]
}
```

### Task with Custom Fields

```json
{
  "id": "RQ-0020",
  "title": "Fix critical security vulnerability",
  "status": "doing",
  "priority": "critical",
  "created_at": "2026-01-15T13:00:00Z",
  "updated_at": "2026-01-15T13:30:00Z",
  "started_at": "2026-01-15T13:30:00Z",
  "tags": ["security", "urgent"],
  "custom_fields": {
    "cve_id": "CVE-2026-1234",
    "severity": "9.8",
    "owner": "security-team",
    "sprint": "24.01",
    "story_points": "8"
  }
}
```

---

## See Also

- [Task Relationships](task-relationships.md) — relationship field semantics and graph constraints.
- [Queue and Tasks](../queue-and-tasks.md) — legacy combined queue and task reference.
- [Queue Schema](../../schemas/queue.schema.json) — generated JSON schema.
