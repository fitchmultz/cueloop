# Getting Started with CueLoop
Status: Active
Owner: Maintainers
Source of truth: this document for guided human onboarding
Parent: [CueLoop Documentation](../index.md)

This guide is for humans who want enough context to run CueLoop safely for the first time. If you only need the shortest command path, use the [Quick Start](../quick-start.md). If you are an already-running coding agent using CueLoop as a ledger, use the [Agent Usage Guide](agent-usage.md) instead.

## What CueLoop does

CueLoop keeps AI-agent work in repo-local files instead of hidden chat state:

- active work: `.cueloop/queue.jsonc`
- completed/rejected work: `.cueloop/done.jsonc`
- project settings: `.cueloop/config.jsonc`
- optional prompt overrides: `.cueloop/prompts/*.md`

A typical loop is:

```text
write task → inspect queue → run supervised phases → validate locally → archive result
```

## 1. Install

From crates.io:

```bash
cargo install cueloop
```

From this repository:

```bash
git clone https://github.com/fitchmultz/cueloop cueloop
cd cueloop
make install
```

On macOS, install GNU Make with `brew install make` and use `gmake` if Apple `make` is first on `PATH`.

Check the binary:

```bash
cueloop version
cueloop --help
```

## 2. Initialize a project

Run this from the repository where you want CueLoop state:

```bash
cd your-project
cueloop init
```

Interactive init helps choose a runner, workflow mode, queue tracking policy, and optional first task. For scripts or CI fixtures:

```bash
cueloop init --non-interactive
```

After init, run:

```bash
cueloop queue validate
cueloop queue list
```

## 3. Add one task

```bash
cueloop task "Add regression tests for webhook delivery failures"
cueloop queue list
cueloop queue show <TASK_ID>
```

Use the task ID printed by `cueloop task` or `cueloop queue list`.

Good first tasks are small, specific, and easy to verify. Avoid asking the first run to redesign the whole repo.

## 4. Inspect before running

Before starting an agent, check what CueLoop will select and whether your runner setup is ready:

```bash
cueloop queue next --with-title
cueloop run one --dry-run
cueloop runner list
cueloop doctor
```

If no runner is configured yet, stop at the dry run and use the [Local Smoke Test](local-smoke-test.md) to verify the CLI and queue model without invoking an external model.

## 5. Run supervised work

Run one task:

```bash
cueloop run one
```

Or cap a loop explicitly:

```bash
cueloop run loop --max-tasks 1
```

Useful variants:

```bash
# Single-pass mode for simple work
cueloop run one --quick

# Full plan → implement → review flow
cueloop run one --phases 3

# Select and explain without executing
cueloop run one --dry-run
```

Do not start with an unlimited loop. Learn queue state, runner behavior, and local validation first.

## 6. Review and validate

After a run, inspect both Git and CueLoop state:

```bash
git status --short
cueloop queue validate
cueloop queue list
```

In this repository, the normal branch gate is:

```bash
make agent-ci
```

For another project, use that project’s local CI/test command.

## 7. Choose deeper docs by question

| Question | Read |
| --- | --- |
| What are all the commands? | [CLI Reference](../cli.md) |
| How do queue files work? | [Queue](../features/queue.md) |
| What fields can a task have? | [Task Schema and Field Reference](../features/task-schema.md) |
| How do statuses and priorities work? | [Task Lifecycle and Priority](../features/task-lifecycle.md) |
| How do dependencies work? | [Task Relationships](../features/task-relationships.md) and [Dependencies](../features/dependencies.md) |
| What happens in each execution phase? | [Phases](../features/phases.md) |
| How do I configure runners and models? | [Runners](../features/runners.md) and [Configuration](../configuration.md) |
| How do CI gates and review safeguards work? | [Supervision](../features/supervision.md) |
| How do I recover interrupted work? | [Session Management](../features/session-management.md) |
| How do I use the macOS app? | [App (macOS)](../features/app.md) |
| How do I debug setup problems? | [Troubleshooting](../troubleshooting.md) |

## Daily operator checklist

1. `cueloop queue validate`
2. `cueloop queue list`
3. `cueloop queue next --with-title`
4. `cueloop run one --dry-run` when unsure
5. `cueloop run one` or a capped `cueloop run loop --max-tasks <N>`
6. Review Git diff and queue state
7. Run the project’s local validation gate

## Next step

If you are evaluating CueLoop, run the [Evaluator Path](evaluator-path.md). If you are adopting it in a project, complete the [Local Smoke Test](local-smoke-test.md) before wiring up a real runner.

## Legacy deep links

The previous version of this page was a long combined tutorial. These headings remain as lightweight redirects so old links and search results still land on useful current guidance.

## What is CueLoop?

See [What CueLoop does](#what-cueloop-does) and the [README](../../README.md).

## Table of Contents

Use [Pick Your Path](../index.md#pick-your-path) in the documentation index.

## 1. Installation

See [Install](#1-install).

### From crates.io (Recommended)

See [Install](#1-install).

### From Source

See [Install](#1-install).

### Verify Installation

See [Install](#1-install).

### Add to PATH

See [Install](#1-install) and [Troubleshooting](../troubleshooting.md).

## 2. Quick Initialization

See [Initialize a project](#2-initialize-a-project).

### Interactive Wizard

See [Initialize a project](#2-initialize-a-project) and [Quick Start](../quick-start.md#2-initialize-a-repository).

### Example Walkthrough

See [Quick Start](../quick-start.md).

### Non-Interactive Mode

See [Initialize a project](#2-initialize-a-project) and [Quick Start](../quick-start.md#2-initialize-a-repository).

### Force Reinitialization

See [CLI Reference](../cli.md).

## 3. Your First Task

See [Add one task](#3-add-one-task).

### macOS: Open the App (SwiftUI)

See [App (macOS)](../features/app.md).

### Run Your First Task

See [Run supervised work](#5-run-supervised-work).

### View the Queue

See [Inspect before running](#4-inspect-before-running) and [Queue](../features/queue.md).

### Creating Tasks

See [Add one task](#3-add-one-task) and [Task Operations](../features/task-operations.md).

### Example Decomposition Session

See [Task Operations](../features/task-operations.md) and [CLI Reference](../cli.md).

### Example Task Session

See [Daily operator checklist](#daily-operator-checklist).

## 4. Understanding the Workflow

See [Workflow in one page](../workflow.md#workflow-in-one-page), [Architecture Overview](../architecture.md), and [Phases](../features/phases.md).

### The 3 Phases

See [Phases](../features/phases.md).

### Phase Mode Comparison

See [Phases](../features/phases.md).

### Choosing the Right Mode

See [Phases](../features/phases.md).

### Changing Modes

See [Configuration](../configuration.md) and [Phases](../features/phases.md).

## 5. Runner Selection

See [Runners](../features/runners.md).

### Runner Comparison

See [Runners](../features/runners.md).

### Recommended Models by Runner

See [Runners](../features/runners.md) and [Agent and Runner Configuration](../features/configuration-agent.md).

### Switching Runners

See [Configuration](../configuration.md) and [Runners](../features/runners.md).

### Checking Runner Availability

See [Inspect before running](#4-inspect-before-running) and [Runners](../features/runners.md).

### Installing Runners

See [Runners](../features/runners.md).

## 6. Configuration Basics

See [Configuration](../configuration.md).

### Configuration Locations

See [Configuration](../configuration.md).

### Essential Configuration

See [Configuration](../configuration.md).

### Key Configuration Options

See [Configuration](../configuration.md).

### Viewing Current Configuration

See [CLI Reference](../cli.md).

### Configuration Profiles

See [Profiles](../features/profiles.md).

## 7. Daily Workflow

See [Daily operator checklist](#daily-operator-checklist).

### Typical Daily Session

See [Daily operator checklist](#daily-operator-checklist).

### CLI Quick Reference

See [CLI Reference](../cli.md).

### Managing Tasks

See [Task Operations](../features/task-operations.md).

### Git Workflow Integration

See [Supervision](../features/supervision.md).

## 8. Next Steps

See [Next step](#next-step).

### Learn More

See [Choose deeper docs by question](#7-choose-deeper-docs-by-question).

### Advanced Features

See [Advanced Usage Guide](advanced.md).

### Best Practices

See [Daily operator checklist](#daily-operator-checklist) and [Project Operating Constitution](project-operating-constitution.md).

### Getting Help

See [Troubleshooting](../troubleshooting.md).

### Community

See [Support Policy](../support-policy.md).

## Quick Reference Card

See [Daily operator checklist](#daily-operator-checklist) and [CLI Reference](../cli.md).
