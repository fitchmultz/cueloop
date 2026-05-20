# Feature guides

Status: Active  
Owner: Maintainers  
Source of truth: index for `docs/features/`  
Parent: [Documentation index](../index.md)

Feature docs describe one capability each. Start from [Documentation index](../index.md) if you are new; use this page to jump by topic.

## Core

| Guide | Topics |
| --- | --- |
| [Phases](phases.md) | 1/2/3-phase execution |
| [Queue](queue.md) | `queue.jsonc`, validation, operations |
| [Tasks](tasks.md) | Task index → schema, lifecycle, operations |
| [Dependencies](dependencies.md) | `depends_on`, DAG, critical path |
| [Context](context.md) | RepoPrompt / context building |

Task detail pages: [task-schema](task-schema.md), [task-lifecycle](task-lifecycle.md), [task-operations](task-operations.md), [task-relationships](task-relationships.md).

## Execution

| Guide | Topics |
| --- | --- |
| [Runners](runners.md) | Runner CLIs and overrides |
| [Parallel](parallel.md) | Parallel workers and integration |
| [Session management](session-management.md) | Resume after crash |
| [Supervision](supervision.md) | CI gates, git, human-in-the-loop |

## Integrations

| Guide | Topics |
| --- | --- |
| [Webhooks](webhooks.md) | HTTP events |
| [Plugins](plugins.md) | Custom runners/processors |
| [Notifications](notifications.md) | Desktop alerts |
| [Import/export](import-export.md) | Queue interchange formats |

Plugin detail: [plugins/](plugins/) (architecture, protocols, operations, security).

## Workflow tools

| Guide | Topics |
| --- | --- |
| [App](app.md) | macOS SwiftUI client |
| [Scan](scan.md) | Repository discovery |
| [Daemon and watch](daemon-and-watch.md) | Background automation |

Daemon/watch detail: [daemon-watch/](daemon-watch/).

## Configuration (feature-level)

| Guide | Topics |
| --- | --- |
| [Configuration](configuration.md) | Feature config map |
| [Configuration agent](configuration-agent.md) | Runners, phases, CI gate |
| [Configuration operations](configuration-operations.md) | Queue paths, parallel |
| [Configuration integrations](configuration-integrations.md) | Webhooks, plugins, profiles |
| [Profiles](profiles.md) | Named presets |
| [Prompts](prompts.md) | Templates and overrides |
| [Migrations](migrations.md) | Config/data migration |

Repo-wide config hub: [Configuration](../configuration.md). Example: [configuration-example](configuration-example.md).

## Security and reference

| Guide | Topics |
| --- | --- |
| [Security](security.md) | Feature security controls |
| [SECURITY.md](../../SECURITY.md) | Vulnerability reporting |

Also: [CLI](../cli.md), [Error handling](../error-handling.md), [Environment](../environment.md), [Machine contract](../machine-contract.md).

## Schemas

- [config.schema.json](../../schemas/config.schema.json)  
- [queue.schema.json](../../schemas/queue.schema.json)  
- [machine.schema.json](../../schemas/machine.schema.json)  

Regenerate after source changes: `make generate`.

## By goal

| I want to… | Read |
| --- | --- |
| Run my first task | [Quick start](../quick-start.md) |
| Configure a runner | [Runners](runners.md), [Configuration](../configuration.md) |
| Run tasks in parallel | [Parallel](parallel.md) |
| Notify Slack/Discord | [Webhooks](webhooks.md) |
| Auto-detect TODOs | [Watch](daemon-watch/watch.md) |
| Resume a crashed run | [Session management](session-management.md) |
| Customize prompts | [Prompts](prompts.md) |

Legacy hubs: [Queue and tasks](../queue-and-tasks.md), [Workflow](../workflow.md).
