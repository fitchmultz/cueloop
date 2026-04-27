# Task Relationships
Status: Active
Owner: Maintainers
Source of truth: this document for task relationship fields and validation semantics
Parent: [Task System](tasks.md)

This page defines Ralph task relationships: `depends_on`, `blocks`, `relates_to`, `duplicates`, and `parent_id`. For graph visualization, critical path analysis, and advanced dependency workflows, see [Dependencies](dependencies.md).

## Related Task Docs

- [Task System](tasks.md) — overview and task documentation index.
- [Task Schema and Field Reference](task-schema.md) — task JSON fields, queue-file basics, per-task agent overrides, examples, and schema validation.
- [Task Lifecycle and Priority](task-lifecycle.md) — statuses, lifecycle timestamps, runnability basics, and priority semantics.
- [Task Operations](task-operations.md) — creation, editing, templates, batch operations, and CLI workflows.
- [Queue](queue.md) — queue file operations, ordering, archive, repair, import/export, and locks.

---

## Task Relationships

### Dependencies (`depends_on`)

**Semantic Meaning**: "I need X before I can run"

**Execution Constraint**: A task is blocked until all tasks in `depends_on` have status `done` or `rejected`.

```json
{
  "id": "RQ-0003",
  "title": "Implement API endpoint",
  "depends_on": ["RQ-0001", "RQ-0002"]
}
```

**Validation Rules:**
- Self-dependency: **Hard error** (cannot depend on yourself)
- Missing dependency: **Hard error** (target must exist in queue or done)
- Circular dependency: **Hard error** (must form a DAG)
- Dependency on rejected task: **Warning** (will never be satisfied)

### Blocking (`blocks`)

**Semantic Meaning**: "I prevent X from running"

**Execution Constraint**: Tasks in `blocks` cannot run until this task is `done` or `rejected`.

```json
{
  "id": "RQ-0001",
  "title": "Design database schema",
  "blocks": ["RQ-0002", "RQ-0003"]
}
```

**Validation Rules:**
- Self-blocking: **Hard error**
- Missing blocked task: **Hard error**
- Circular blocking: **Hard error** (must form a DAG)

**Relationship to `depends_on`:**
- `blocks` is semantically inverse of `depends_on`
- If A `blocks` B, then B should logically `depends_on` A
- Ralph validates consistency but does not enforce bidirectional links

### Related Tasks (`relates_to`)

**Semantic Meaning**: "This work is related to X" (loose coupling)

**Execution Constraint**: None. Purely informational.

```json
{
  "id": "RQ-0005",
  "title": "Refactor auth module",
  "relates_to": ["RQ-0003", "RQ-0004"]
}
```

**Validation Rules:**
- Self-reference: **Hard error**
- Missing related task: **Hard error**

### Duplicates

**Semantic Meaning**: "This task is a duplicate of X"

**Execution Constraint**: None. Informational for cleanup.

```json
{
  "id": "RQ-0006",
  "title": "Fix login bug",
  "duplicates": "RQ-0005"
}
```

**Validation Rules:**
- Self-duplication: **Hard error**
- Missing duplicated task: **Hard error**
- Duplicate of done/rejected task: **Warning**

### Parent/Child Hierarchy (`parent_id`)

**Semantic Meaning**: "This task is a subtask of X"

**Execution Constraint**: None. Used for organizational structure.

```json
{
  "id": "RQ-0002",
  "title": "Implement Part A",
  "parent_id": "RQ-0001"
}
```

**Key Characteristics:**
- A task can have at most one parent
- A parent can have multiple children
- Cycles are not allowed (A → B → A)
- Does not affect execution order (unlike `depends_on`)

**Validation Rules:**
- Self-parent: **Warning**
- Missing parent: **Warning** (orphaned task)
- Circular parent chain: **Warning**

**CLI Navigation:**
```bash
# List children
ralph task children RQ-0001
ralph task children RQ-0001 --recursive

# Show parent
ralph task parent RQ-0002

# Visualize hierarchy
ralph queue tree
ralph queue tree --root RQ-0001
```

### Relationship Comparison

| Feature | `depends_on` | `blocks` | `relates_to` | `duplicates` | `parent_id` |
|---------|--------------|----------|--------------|--------------|-------------|
| Execution constraint | Yes | Yes (inverse) | No | No | No |
| Must form DAG | Yes | Yes | No | N/A | Yes (warnings) |
| Self-reference allowed | No | No | No | No | No |
| Validation severity | Error | Error | Error | Error | Warning |
| Visualization | `queue graph` | `queue graph` | None | None | `queue tree` |

---

## Relationship Validation Summary

Relationship validation is enforced with the same hard-error and warning model described in [Task Schema and Field Reference](task-schema.md#task-validation). Relationship-specific outcomes are:

### Hard Errors

- `depends_on` self-dependency, missing dependency targets, and circular dependency chains.
- `blocks` self-blocking, missing blocked tasks, and circular blocking chains.
- `relates_to` self-references and missing related task targets.
- `duplicates` self-duplication and missing duplicated task targets.

### Warnings

- `depends_on` references to rejected tasks.
- Dependency chains deeper than `queue.max_dependency_depth` (default: 10).
- Blocked execution paths where all dependency paths lead to incomplete or rejected tasks.
- `duplicates` references to terminal tasks.
- `parent_id` missing parents, self-parent references, and circular parent chains.

Run `ralph queue validate` after relationship edits. Most queue mutation commands also validate after writing.

## See Also

- [Dependencies](dependencies.md) — graph visualization, DAG execution, and critical path analysis.
- [Task Operations](task-operations.md) — commands for editing relationship fields.
