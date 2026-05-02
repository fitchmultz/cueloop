# CueLoop Configuration Feature Guide
Status: Active
Owner: Maintainers
Source of truth: this document for feature-level configuration navigation and operator workflows
Parent: [Feature Documentation](README.md)

CueLoop configuration is documented in two layers:

- [Main Configuration Reference](../configuration.md): canonical schema, defaults, exact precedence, and validation details.
- This feature guide: how operators choose the right configuration surface and where feature-specific guidance lives.

Use this page to decide where a setting belongs. Use the main reference when you need exact field names, defaults, and validation rules.

---

## Configuration Map

| Need | Start here | Canonical reference |
|------|------------|---------------------|
| Trust a repo for project commands | [Trust and safety](#trust-and-safety) | [Repo execution trust](../configuration/trust-and-precedence.md#repo-execution-trust) |
| Choose a runner/model/phases | [Agent and runner settings](configuration-agent.md) | [Agent Configuration](../configuration/agent-and-runners.md#agent-configuration) |
| Configure queue paths/aging/parallel workers | [Queue and parallel settings](configuration-operations.md) | [Queue Configuration](../configuration/queue-and-parallel.md#queue-configuration) |
| Configure plugins/webhooks/notifications/profiles | [Integrations and profiles](configuration-integrations.md) | [Plugin Configuration](../configuration/plugins-and-profiles.md#plugin-configuration) |
| Keep a complete example nearby | [Complete Configuration Example](configuration-example.md) | [Config Schema](../../schemas/config.schema.json) |

---

## Config Files and Locations

CueLoop loads configuration from global and project scopes. The executable is `cueloop`.

- Global (user): `~/.config/cueloop/config.jsonc`
- Project (repo): `.cueloop/config.jsonc`
- Local trust gate (repo): `.cueloop/trust.jsonc` (must remain untracked)

Useful commands:

```bash
cueloop config show
cueloop config show --format json
cueloop config paths
cueloop config schema
cueloop config profiles list
```

---

## Precedence at a Glance

For ordinary resolved config, the canonical order is CLI flags, project config, global config, then schema defaults. Selected profiles and task-level agent overrides participate in execution-specific resolution.

For exact behavior and edge cases, see:

- [Precedence](../configuration/trust-and-precedence.md#precedence)
- [Profiles](../configuration/plugins-and-profiles.md#profiles)

---

## Trust and Safety

Project-local execution settings are applied only after explicit local trust opt-in.

- Use `cueloop init` while bootstrapping; it creates or updates `.cueloop/trust.jsonc` by default.
- Use `cueloop config trust init` only for trust-only repair in an already-initialized repo.
- Do not commit `.cueloop/trust.jsonc`.

Execution-sensitive project settings include:

- Runner binary overrides
- `agent.ci_gate`
- Plugin runner selections
- `plugins.*`
- `parallel.ignored_file_allowlist` for additional ignored local files copied to parallel worker workspaces

In untrusted repos, keep these settings in global config or trust the repo locally before expecting project values to apply.

For the allowed file/glob contract and invalid examples such as `node_modules/*` and `dir/`, see [Ignored local file sync](../configuration/queue-and-parallel.md#ignored-local-file-sync).

Canonical details: [Repo execution trust](../configuration/trust-and-precedence.md#repo-execution-trust).

---

## JSONC and Validation Basics

CueLoop supports JSONC (`.jsonc`) for runtime config and queue files:

- Comments and trailing commas are accepted on load.
- CueLoop may rewrite files as standard JSON formatting when saving.
- Use `cueloop config show` and `cueloop config schema` to verify effective values.

Current configuration version is `2`. Prefer canonical validation/error details from [Configuration](../configuration.md).

---

## Feature Guides

Use these focused pages for feature-level configuration decisions:

- [Agent and Runner Configuration](configuration-agent.md)
- [Queue and Parallel Configuration](configuration-operations.md)
- [Integration and Profile Configuration](configuration-integrations.md)
- [Complete Configuration Example](configuration-example.md)

---

## See Also

- [Main Configuration Documentation](../configuration.md)
- [CLI Reference](../cli.md)
- [Runners](./runners.md)
- [Phases](./phases.md)
- [Parallel](./parallel.md)
- [Queue](./queue.md)
- [Webhooks](./webhooks.md)
- [Plugins](./plugins.md)
- [Profiles](./profiles.md)
- [JSON Schema](../../schemas/config.schema.json)
