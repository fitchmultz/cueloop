# Plugin Operations
Status: Active
Owner: Maintainers
Source of truth: this document for plugin configuration and lifecycle commands
Parent: [CueLoop Plugin System](../plugins.md)

Purpose: Show how operators scaffold, install, enable, configure, inspect, validate, and remove plugins.

---

## Configure and Enable

Plugins are discovered automatically but **disabled by default**.

```json
{
  "version": 1,
  "plugins": {
    "plugins": {
      "my.plugin": {
        "enabled": true,
        "config": {
          "endpoint": "https://api.example.com",
          "timeout": 30
        }
      }
    }
  }
}
```

- Plugin config is passed through `RALPH_PLUGIN_CONFIG_JSON`.
- Runner/processor executable paths come from `plugin.json`, not config.
- Config-level binary overrides are not supported.
- Project-scope plugin runtime execution depends on repository trust.

## Scaffold

```bash
cueloop plugin init my.plugin
cueloop plugin init my.plugin --with-runner
cueloop plugin init my.plugin --with-processor
cueloop plugin init my.plugin --scope global
cueloop plugin init my.plugin --dry-run
```

## Install

```bash
cueloop plugin install ./my-plugin --scope project
cueloop plugin install ./my-plugin --scope global
```

Install does **not** auto-enable plugins.

## List

```bash
cueloop plugin list
cueloop plugin list --json
```

## Validate

```bash
cueloop plugin validate
cueloop plugin validate --id my.plugin
```

Validation includes API version, ID format, required manifest fields, and supported hooks.

## Uninstall

```bash
cueloop plugin uninstall my.plugin --scope project
cueloop plugin uninstall my.plugin --scope global
```

## Related Docs

- [Configuration](../../configuration/plugins-and-profiles.md#plugin-configuration)
- [CLI Reference](../../cli.md)
- [Plugin Security](security.md)
