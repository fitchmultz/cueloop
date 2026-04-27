# Plugin Architecture and Manifest
Status: Active
Owner: Maintainers
Source of truth: this document for plugin discovery, layout, manifest, and precedence rules
Parent: [Ralph Plugin System](../plugins.md)

Purpose: Explain how Ralph discovers plugins and validates plugin manifests.

---

## Discovery Layout

Plugins are discovered from two locations:

```text
~/.config/ralph/plugins/          # Global plugins (all projects)
├── my.plugin/
│   ├── plugin.json               # Required: Plugin manifest
│   ├── runner.sh                 # Optional: Runner executable
│   └── processor.sh              # Optional: Processor executable

./.ralph/plugins/                 # Project plugins (override global)
├── my.plugin/
│   ├── plugin.json
│   ├── runner.sh
│   └── processor.sh
```

Project plugins override global plugins with the same ID.

## Plugin Types

| Type | Purpose |
|------|---------|
| Runner | Execute prompts against custom AI backends |
| Processor | Hook into task lifecycle events |

## Plugin Manifest (`plugin.json`)

Every plugin requires a `plugin.json` manifest file:

```json
{
  "api_version": 1,
  "id": "my.plugin",
  "version": "1.0.0",
  "name": "My Plugin",
  "description": "A custom plugin for Ralph",
  "runner": {
    "bin": "runner.sh",
    "supports_resume": true,
    "default_model": "custom-model-v1"
  },
  "processors": {
    "bin": "processor.sh",
    "hooks": ["validate_task", "pre_prompt", "post_run"]
  }
}
```

### Manifest Field Reference

| Field | Required | Description |
|-------|----------|-------------|
| `api_version` | Yes | Must be `1` (current API version) |
| `id` | Yes | Unique identifier (no spaces, no path separators: `/` or `\`) |
| `version` | Yes | Semantic version (for example `1.0.0`) |
| `name` | Yes | Human-readable display name |
| `description` | No | Brief description of plugin functionality |
| `runner` | No | Runner configuration (required for runner plugins) |
| `runner.bin` | Yes* | Path to runner executable (relative to plugin directory) |
| `runner.supports_resume` | No | Whether runner supports session resumption (default: `false`) |
| `runner.default_model` | No | Default model when none is specified |
| `processors` | No | Processor configuration (required for processor plugins) |
| `processors.bin` | Yes* | Path to processor executable (relative to plugin directory) |
| `processors.hooks` | Yes* | Supported hooks: `validate_task`, `pre_prompt`, `post_run` |

\* Required if the parent section is present.

## Path and Precedence Rules

- Manifest `runner.bin` / `processors.bin` paths must be plugin-dir-relative.
- Absolute paths are rejected.
- Path escape (`..`) is rejected.
- Existing symlinked files and ancestor directories must canonicalize inside the plugin directory.
- Project plugin IDs shadow global plugins with the same ID.

## Related Docs

- [Plugin Operations](operations.md)
- [Plugin Security](security.md)
