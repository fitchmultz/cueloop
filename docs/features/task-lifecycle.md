# Task Lifecycle and Priority
Status: Active
Owner: Maintainers
Source of truth: this document for task status transitions, lifecycle timestamps, and priority semantics
Parent: [Task System](tasks.md)

This page explains how task statuses and priorities describe work state in CueLoop. Queue ordering and archive mechanics are covered in [Queue](queue.md); dependency graph runnability is covered in [Dependencies](dependencies.md).

## Related Task Docs

- [Task System](tasks.md) — overview and task documentation index.
- [Task Schema and Field Reference](task-schema.md) — task JSON fields, queue-file basics, per-task agent overrides, examples, and schema validation.
- [Task Relationships](task-relationships.md) — dependency, blocking, relation, duplicate, and hierarchy semantics.
- [Task Operations](task-operations.md) — creation, editing, templates, batch operations, and CLI workflows.
- [Queue](queue.md) — queue file operations, ordering, archive, repair, import/export, and locks.

---

## Task Status Lifecycle

### Status Values

| Status | Description |
|--------|-------------|
| `draft` | Work in progress definition, skipped by default in execution |
| `todo` | Ready to work, pending dependency resolution |
| `doing` | Currently being worked on |
| `done` | Completed successfully |
| `rejected` | Will not be completed (duplicate, obsolete, out of scope) |

### Status Transitions

```
                    ┌─────────┐
         ┌─────────▶│  draft  │◀────────┐
         │          └────┬────┘         │
         │               │              │
         │               ▼              │
    ┌────┴────┐     ┌─────────┐    ┌────┴────┐
    │rejected │◀────│   todo  │───▶│  doing  │
    └────┬────┘     └────┬────┘    └────┬────┘
         │               │              │
         │               ▼              │
         └─────────▶│  done   │◀─────────┘
                    └─────────┘
```

### Transition Rules

**INTENDED BEHAVIOR:**
- `draft` → `todo`: Task definition finalized, ready for execution
- `todo` → `doing`: Work begins, `started_at` timestamp set
- `doing` → `done`: Work completed, `completed_at` timestamp set
- `doing` → `rejected`: Work abandoned
- `todo` → `rejected`: Task cancelled before starting
- Any → `draft`: Task needs redefinition

**CURRENTLY IMPLEMENTED BEHAVIOR:**
- Status cycling via CLI (`cueloop task edit RQ-0001 status` with no value) cycles: `todo` → `doing` → `done` → `rejected` → `draft` → `todo`
- Direct status setting validates the target status is valid
- `started_at` is automatically set when transitioning to `doing`
- `completed_at` is automatically set when transitioning to `done` or `rejected`

### Status Policy Enforcement

```rust
// When transitioning to 'doing'
if next_status == TaskStatus::Doing && task.started_at.is_none() {
    task.started_at = Some(now.to_string());
}

// When transitioning to terminal status
if next_status.is_terminal() {
    task.completed_at = Some(now.to_string());
    // Trigger auto-archive if configured
}
```

---

## Task Priority

### Priority Levels

| Priority | Weight | Use Case |
|----------|--------|----------|
| `critical` | 3 | Blockers, security fixes, data loss prevention |
| `high` | 2 | Important features, significant improvements |
| `medium` | 1 | Normal work (default) |
| `low` | 0 | Nice-to-have, backlog items |

### Priority Ordering

Priority follows natural ordering: `Critical > High > Medium > Low`

```rust
// Comparison: Critical is "greater than" High
assert!(TaskPriority::Critical > TaskPriority::High);
assert!(TaskPriority::High > TaskPriority::Medium);
assert!(TaskPriority::Medium > TaskPriority::Low);
```

### Priority Cycling

When editing priority in an interactive UI, an empty value cycles through levels:
```
low → medium → high → critical → low
```

### Effect on Execution

**INTENDED BEHAVIOR:**
- Priority affects task ordering within the queue
- Higher priority tasks should be suggested first when multiple tasks are runnable
- Critical priority tasks may bypass normal scheduling

**CURRENTLY IMPLEMENTED BEHAVIOR:**
- Priority is stored and displayed but does not affect automatic execution order
- Tasks execute in file order (top to bottom)
- Priority can be used for manual filtering and UI sorting

---

## See Also

- [Task Operations](task-operations.md) — CLI workflows that create, edit, and complete tasks.
- [Queue](queue.md) — queue ordering, locking, repair, and archive mechanics.
