# CueLoop Feature Guides
Status: Active
Owner: Maintainers
Source of truth: this document for feature-guide navigation
Parent: [CueLoop Documentation](../index.md)

This page is the feature-area map. If you are new to CueLoop, start with the [Quick Start](../quick-start.md) or the main [documentation index](../index.md) first.

## Task tracking core

These features work without configuring or spawning a runner.

| Feature | Use it for |
| --- | --- |
| [Queue](queue.md) | Active queue file operations, ordering, locking, repair, archive, import/export, and validation |
| [Task System](tasks.md) | Task docs ownership map and task-system overview |
| [Task Schema and Field Reference](task-schema.md) | Task JSON fields, per-task agent overrides, examples, and schema validation |
| [Task Lifecycle and Priority](task-lifecycle.md) | Status transitions, lifecycle timestamps, and priority semantics |
| [Task Relationships](task-relationships.md) | Dependencies, blocking, related tasks, duplicates, and hierarchy |
| [Task Operations](task-operations.md) | Creating, editing, templating, cloning, importing, batching, and CLI workflows |
| [Dependencies](dependencies.md) | Dependency graph execution, analysis, and critical paths |
| [Context](context.md) | RepoPrompt/context-building behavior |
| [Agent Usage Guide](../guides/agent-usage.md) | Durable task ledger commands for already-running agents |

## Execution automation

These features are optional. Use them when CueLoop should dispatch, supervise, or integrate with external runners and automation.

| Feature | Use it for |
| --- | --- |
| [Phases](phases.md) | Plan → implement → review execution behavior |
| [Runners](runners.md) | AI runner orchestration and runner-specific setup |
| [Supervision](supervision.md) | CI gates, git operations, completion checks, and human-in-the-loop oversight |
| [Parallel](parallel.md) | Parallel task execution and worker workspace integration |
| [Session Management](session-management.md) | Crash recovery, resume behavior, and session IDs |
| [Prompts](prompts.md) | Embedded prompts, repo overrides, and prompt sync/diff workflows |

## App and workflow tools

| Feature | Use it for |
| --- | --- |
| [App (macOS)](app.md) | SwiftUI app behavior and CLI parity |
| [Scan](scan.md) | AI-assisted repository scanning for task discovery |
| [Daemon and Watch](daemon-and-watch.md) | Background execution and automatic task detection overview |
| [Daemon](daemon-watch/daemon.md) | Daemon command details |
| [Watch](daemon-watch/watch.md) | TODO/FIXME/HACK/XXX watcher behavior |
| [Daemon/Watch Operations](daemon-watch/operations.md) | Operations and runbook guidance |
| [Daemon/Watch Troubleshooting](daemon-watch/troubleshooting.md) | Failure and recovery notes |

## Integrations and extension points

| Feature | Use it for |
| --- | --- |
| [Webhooks](webhooks.md) | HTTP event notifications and delivery behavior |
| [Notifications](notifications.md) | Desktop notifications and sounds |
| [Plugins](plugins.md) | Plugin overview and lifecycle |
| [Plugin Architecture](plugins/architecture.md) | Plugin boundaries and runtime architecture |
| [Plugin Examples](plugins/examples.md) | Example plugin shapes |
| [Plugin Operations](plugins/operations.md) | Install, configure, and operate plugins |
| [Processor Protocol](plugins/processor-protocol.md) | Processor plugin contract |
| [Runner Protocol](plugins/runner-protocol.md) | Runner plugin contract |
| [Plugin Security](plugins/security.md) | Plugin trust and safety model |
| [Plugin Troubleshooting](plugins/troubleshooting.md) | Plugin failure recovery |
| [Import/Export](import-export.md) | Queue import/export formats and workflows |

## Configuration and migrations

| Feature | Use it for |
| --- | --- |
| [Feature Configuration Map](configuration.md) | Feature-level configuration map and operator guidance |
| [Agent and Runner Configuration](configuration-agent.md) | Runner, model, phase, CI gate, permission, and retry settings |
| [Queue and Parallel Configuration](configuration-operations.md) | Queue paths, task aging, auto-archive, and parallel workspace settings |
| [Integration and Profile Configuration](configuration-integrations.md) | Notifications, webhooks, plugins, profiles, and environment variables |
| [Complete Configuration Example](configuration-example.md) | Long assembled configuration sample |
| [Profiles](profiles.md) | Configuration profiles for workflow switching |
| [Migrations](migrations.md) | Configuration and data migration guide |

## Security and schemas

| Feature | Use it for |
| --- | --- |
| [Security Features](security.md) | Feature-level security behavior and configuration |
| [Security Policy](../../SECURITY.md) | Vulnerability reporting policy |
| [Config Schema](../../schemas/config.schema.json) | Generated configuration schema |
| [Queue Schema](../../schemas/queue.schema.json) | Generated queue/task schema |
| [Machine Schema](../../schemas/machine.schema.json) | Generated machine integration schema |

## Common goals

| Goal | Start with |
| --- | --- |
| Use CueLoop as an already-running agent | [Agent Usage Guide](../guides/agent-usage.md), then [Task Schema and Field Reference](task-schema.md) |
| Configure a runner | [Runners](runners.md), then [Agent and Runner Configuration](configuration-agent.md) |
| Set up parallel execution | [Parallel](parallel.md), then [Queue and Parallel Configuration](../configuration/queue-and-parallel.md#parallel-configuration) |
| Integrate Slack, Discord, or CI notifications | [Webhooks](webhooks.md), then [Notifications](notifications.md) |
| Automate task detection | [Watch](daemon-watch/watch.md), then [Scan](scan.md) |
| Recover from an interrupted run | [Session Management](session-management.md), then [Troubleshooting](../troubleshooting.md) |
| Customize prompts | [Prompts](prompts.md), then [Phases](phases.md) |
| Understand task dependencies | [Task Relationships](task-relationships.md), then [Dependencies](dependencies.md) |

## Contributing to feature docs

When adding or changing feature behavior:

1. Update the canonical feature page from the tables above.
2. Update this index only when a feature page is added, removed, renamed, or materially reclassified.
3. Update the main [documentation index](../index.md) only for new top-level entry points or ownership changes.
4. Run `make agent-ci` or the routed docs gate before opening a PR.
