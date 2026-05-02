# Configuration
Status: Active
Owner: Maintainers
Source of truth: this document for configuration navigation; linked chapter docs are source of truth for their stated configuration domains
Parent: [CueLoop Documentation](index.md)

![Configuration Layers](assets/images/2026-03-10-12-00-08-config-layers.png)

Purpose: Document CueLoop's JSON configuration layout, defaults, override precedence, and where to find detailed reference material for each configuration domain.

## Overview

CueLoop reads JSONC configuration from two locations, with project config taking precedence over global config only after repo trust rules are applied where required. The executable is `cueloop`, and the package is `cueloop-agent-loop`.

- Global: `~/.config/cueloop/config.jsonc`
- Project: `.cueloop/config.jsonc`

CLI flags override both for a single run. Defaults are defined by `schemas/config.schema.json`.

## Configuration chapters

| Chapter | Scope |
|---------|-------|
| [Trust and precedence](configuration/trust-and-precedence.md) | repo execution trust, JSONC support, config locations, merge order, macOS safety warnings |
| [Agent and runners](configuration/agent-and-runners.md) | `agent.*`, runner CLI normalization, retry policy, phase overrides, CI gate, runner session notes |
| [Queue and parallel](configuration/queue-and-parallel.md) | `queue.*`, archive/aging policy, `parallel.*`, workspace and queue-path restrictions |
| [Notifications and webhooks](configuration/notifications-and-webhooks.md) | desktop notifications, webhook delivery, payloads, testing, security |
| [Plugins and profiles](configuration/plugins-and-profiles.md) | plugin configuration/security, plugin commands, built-in and custom profiles |
| [Migration notes](configuration/migration-notes.md) | breaking config changes and migration commands relevant to current config files |

## Top-level fields

- `version` (number): Config schema version. Default: `2`.
- `project_type` (string or null): `code` or `docs`. Default: `code`.
- `agent` (object): Runner defaults, CLI binaries, prompt enforcement, notifications, and webhook settings. See [Agent and runners](configuration/agent-and-runners.md) and [Notifications and webhooks](configuration/notifications-and-webhooks.md).
- `parallel` (object): Parallel run-loop configuration, including trusted ignored local file sync policy. See [Queue and parallel](configuration/queue-and-parallel.md).
- `queue` (object): Queue file locations, task ID formatting, archive policy, and aging thresholds. See [Queue and parallel](configuration/queue-and-parallel.md).
- `plugins` (object): Plugin enablement and per-plugin settings. See [Plugins and profiles](configuration/plugins-and-profiles.md).
- `profiles` (object, optional): Named agent configuration patches. See [Plugins and profiles](configuration/plugins-and-profiles.md).

## Precedence summary

Detailed trust and profile rules live in [Trust and precedence](configuration/trust-and-precedence.md) and [Plugins and profiles](configuration/plugins-and-profiles.md).

1. CLI flags for the current command.
2. Task-specific overrides where supported.
3. Selected profile patches where supported.
4. Project config (`.cueloop/config.jsonc`), subject to repo trust for execution-sensitive values.
5. Global config (`~/.config/cueloop/config.jsonc`).
6. Schema defaults (`schemas/config.schema.json`).

## Minimal example

```jsonc
{
  "version": 2,
  "agent": {
    "runner": "codex",
    "model": "gpt-5.4",
    "phases": 3
  },
  "queue": {
    "file": ".cueloop/queue.jsonc",
    "done_file": ".cueloop/done.jsonc"
  }
}
```

## Related documentation

- [Configuration feature guide](features/configuration.md)
- [Advanced profiles and configuration](guides/advanced-profiles-and-configuration.md)
- [CLI reference](cli.md)
- [Schema defaults](../schemas/config.schema.json)
