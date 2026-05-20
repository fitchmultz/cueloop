# Workflow (legacy hub)

Status: Active  
Owner: Maintainers  
Source of truth: navigation only — detailed content lives in linked guides  
Parent: [Documentation index](index.md)

This page exists for older links. Prefer the guides below for current workflow documentation.

![3-Phase Workflow](assets/images/2026-02-07-workflow-3phase.png)

## Runtime layout

| Path | Role |
| --- | --- |
| `.cueloop/queue.jsonc` | Active tasks |
| `.cueloop/done.jsonc` | Completed / rejected tasks |
| `.cueloop/config.jsonc` | Configuration |
| `.cueloop/prompts/*.md` | Optional prompt overrides |
| `.cueloop/cache/plans/<TASK_ID>.md` | Phase 1 plan cache |
| `.cueloop/cache/parallel/state.json` | Parallel run state |

Embedded defaults live under `crates/cueloop/assets/prompts/`. Overrides must keep required placeholders (for example `{{USER_REQUEST}}` in builder prompts).

## Execution model

| Topic | Document |
| --- | --- |
| Phase behavior (plan → implement → review) | [Phases](features/phases.md) |
| Architecture and trust boundaries | [Architecture](architecture.md) |
| Parallel workers and integration loop | [Parallel](features/parallel.md), [Architecture — parallel lifecycle](architecture.md#sequence-parallel-worker-lifecycle) |
| CI gates and git publish modes | [Supervision](features/supervision.md) |
| Prompt templates | [Prompts](features/prompts.md) |

## Common commands

```bash
cueloop run one --phases 3
cueloop run loop --max-tasks 5
cueloop run loop --parallel 2    # experimental; see Parallel guide
cueloop run loop --wait-when-blocked
```

Sequential loop with `--wait-when-blocked` polls the queue when tasks are blocked by dependencies or `scheduled_start` instead of exiting immediately.
