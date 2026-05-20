# Queue and tasks (legacy hub)

Status: Active  
Owner: Maintainers  
Source of truth: navigation only — detailed content lives in linked guides  
Parent: [Documentation index](index.md)

This page exists for older links and tests that reference `docs/queue-and-tasks.md`. Use the focused guides below instead of treating this file as a full reference.

## Where to look

| Topic | Document |
| --- | --- |
| Queue file, validation, operations | [Queue](features/queue.md) |
| Task index and operations | [Tasks](features/tasks.md) |
| Field definitions and schema | [Task schema](features/task-schema.md) |
| Status transitions | [Task lifecycle](features/task-lifecycle.md) |
| `depends_on`, `blocks`, graph | [Dependencies](features/dependencies.md), [Task relationships](features/task-relationships.md) |
| Machine-readable schema | [schemas/queue.schema.json](../schemas/queue.schema.json) |

## Quick reference

- Active work: `.cueloop/queue.jsonc`  
- Archive: `.cueloop/done.jsonc` (terminal statuses only)  
- Default executable tasks: `kind: work_item` (omit `kind` on save)  
- Non-runnable grouping nodes: `kind: group`  

Validate after edits:

```bash
cueloop queue validate
```
