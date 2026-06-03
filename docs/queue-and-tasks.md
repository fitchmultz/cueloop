# Queue and Tasks
Status: Legacy navigation bridge
Owner: Maintainers
Source of truth: canonical pages linked below
Parent: [CueLoop Documentation](index.md)

This URL is kept for older links, test evidence, and search results. It is no longer the place to add new queue or task reference material.

CueLoop now splits queue and task documentation into focused pages so human readers can find the right concept without scanning one oversized combined reference.

## Where to go now

| Need | Canonical page |
| --- | --- |
| Active queue file operations, ordering, locking, repair, archive, import/export, and queue validation | [Queue](features/queue.md) |
| Task-system overview and ownership map | [Task System](features/tasks.md) |
| Task JSON fields, queue envelope, per-task agent overrides, validation, and examples | [Task Schema and Field Reference](features/task-schema.md) |
| Status values, lifecycle timestamps, transition behavior, and priority semantics | [Task Lifecycle and Priority](features/task-lifecycle.md) |
| `depends_on`, `blocks`, `relates_to`, `duplicates`, `parent_id`, and relationship validation | [Task Relationships](features/task-relationships.md) |
| Task creation, editing, templating, cloning, imports, batches, and quick CLI workflows | [Task Operations](features/task-operations.md) |
| Dependency graph execution and critical path analysis | [Dependencies](features/dependencies.md) |
| Complete command syntax | [CLI Reference](cli.md) |
| Agent-safe task insertion (`cueloop machine task insert`) | [Agent Usage Guide](guides/agent-usage.md), [CLI Reference](cli.md#recovery-and-continuation), and [Machine Contract](machine-contract.md) |
| Follow-up proposal files (`followups@v1`) | [Agent Usage Guide](guides/agent-usage.md), [CLI Reference](cli.md#recovery-and-continuation), and [Machine Contract](machine-contract.md) |
| Generated queue schema | [Queue Schema](../schemas/queue.schema.json) |

## Quick human summary

- Active work lives in `.cueloop/queue.jsonc`.
- Completed and rejected work moves to `.cueloop/done.jsonc`.
- Queue file order is the default execution order.
- `status` describes lifecycle (`draft`, `todo`, `doing`, `done`, `rejected`).
- `kind` describes actionability (`work_item` runs; `group` organizes work and is skipped by execution).
- Dependencies block execution until referenced tasks are terminal.
- `cueloop queue validate` is the first check after manual queue edits.

Starter commands:

```bash
cueloop queue list
cueloop queue next --with-title
cueloop queue validate
cueloop task "Add regression tests for queue repair"
cueloop task ready <TASK_ID>
```

## Legacy deep links

The old combined reference exposed many section anchors. These headings remain as lightweight redirects so existing deep links keep landing on useful guidance.

## Queue File

Moved to [Queue](features/queue.md#queue-file-format) and [Task Schema and Field Reference](features/task-schema.md#minimum-queue-envelope).

## Task Fields

Moved to [Task Schema and Field Reference](features/task-schema.md#task-fields).

## Example Task

Moved to [Task Schema and Field Reference](features/task-schema.md#complete-task-examples).

## Lifecycle Notes

Moved to [Task Lifecycle and Priority](features/task-lifecycle.md#task-status-lifecycle) and [Queue](features/queue.md#task-lifecycle).

## Discovery Follow-Ups

Moved to [Agent Usage Guide](guides/agent-usage.md), [CLI Reference](cli.md#recovery-and-continuation), and [Machine Contract](machine-contract.md).

## Atomic Task Insertion for Agents and Scripts

Moved to [Agent Usage Guide](guides/agent-usage.md), [CLI Reference](cli.md#recovery-and-continuation), and [Machine Contract](machine-contract.md).

## Dependency Validation

Moved to [Task Relationships](features/task-relationships.md#relationship-validation-summary) and [Dependencies](features/dependencies.md).

### Hard Errors (blocking)

Moved to [Task Relationships](features/task-relationships.md#relationship-validation-summary).

### Relationship Validation

Moved to [Task Relationships](features/task-relationships.md#relationship-validation-summary).

## Hierarchy (parent_id)

Moved to [Task Relationships](features/task-relationships.md).

### How it Works

Moved to [Task Relationships](features/task-relationships.md).

### Example

Moved to [Task Relationships](features/task-relationships.md).

### CLI Navigation Commands

Moved to [Task Operations](features/task-operations.md#cli-quick-reference) and [CLI Reference](cli.md).

### Hierarchy vs Dependencies

Moved to [Task Relationships](features/task-relationships.md).

### Parent ID Validation

Moved to [Task Relationships](features/task-relationships.md#relationship-validation-summary).

### Warnings (non-blocking)

Moved to [Task Relationships](features/task-relationships.md#relationship-validation-summary).

### Configuration

Moved to [Configuration](configuration.md).

## Task ID Validation

Moved to [Task Schema and Field Reference](features/task-schema.md#task-validation).

### Duplicate Task ID Errors

Moved to [Task Schema and Field Reference](features/task-schema.md#task-validation).

### Fixing ID Collisions

Moved to [Queue](features/queue.md#queue-operations) and [Task Operations](features/task-operations.md#task-editing).

### Prevention

Moved to [Task Schema and Field Reference](features/task-schema.md#task-validation).

## Dependency Visualization

Moved to [Dependencies](features/dependencies.md).

### CLI Graph Command

Moved to [Dependencies](features/dependencies.md) and [CLI Reference](cli.md).

### macOS App

Moved to [App (macOS)](features/app.md).

### Critical Path

Moved to [Dependencies](features/dependencies.md).


### List Child Tasks

Moved to [Task Relationships](features/task-relationships.md) and [CLI Reference](cli.md).

### Include Done Archive in Search

Moved to [Task Relationships](features/task-relationships.md) and [CLI Reference](cli.md).

### Graphviz DOT Format for External Rendering

Moved to [Dependencies](features/dependencies.md) and [CLI Reference](cli.md).

## Import and Export

Moved to [Import/Export](features/import-export.md).

### Export

Moved to [Import/Export](features/import-export.md).

### Import

Moved to [Import/Export](features/import-export.md).

## Maintainer note

Keep this page short. When behavior changes, update the canonical page from the table above and leave this bridge as a stable pointer for old links.
