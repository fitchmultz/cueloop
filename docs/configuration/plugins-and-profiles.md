# Configuration: Plugins and Profiles
Status: Active
Owner: Maintainers
Source of truth: this document for plugin configuration, plugin command references, and configuration profiles
Parent: [Configuration](../configuration.md)

Purpose: Document CueLoop plugin settings and profile-based agent configuration patches.

## Plugin Configuration

`plugins` controls custom runner and processor plugins. Plugins enable extending CueLoop with custom runners without modifying the core codebase.

**Security warning:** Plugins are NOT sandboxed. Enabling a plugin is equivalent to trusting it with full system access. Only enable plugins from trusted sources.

Project-local plugin settings and project-scope plugin directories require repo trust (see [Repo execution trust](trust-and-precedence.md#repo-execution-trust)). In untrusted repos, CueLoop ignores project plugins during runtime discovery.

Supported fields:
- `plugins.plugins.<id>.enabled`: enable/disable the plugin (default: `false`).
- `plugins.plugins.<id>.config`: opaque configuration blob passed to the plugin.

Plugin directories are discovered with current paths preferred over legacy fallback, and project plugins overriding global plugins with the same id:

1. Legacy global fallback: `~/.config/ralph/plugins/<plugin_id>/plugin.json`
2. Current global: `~/.config/cueloop/plugins/<plugin_id>/plugin.json`
3. Legacy project fallback: `.ralph/plugins/<plugin_id>/plugin.json`
4. Current project: `.cueloop/plugins/<plugin_id>/plugin.json`

New `ralph plugin install` and `ralph plugin init` writes target `.cueloop/plugins` or `~/.config/cueloop/plugins`. Use `ralph migrate runtime-dir --apply` to move legacy project runtime state when ready.

Example:

```json
{
  "version": 2,
  "plugins": {
    "plugins": {
      "my.custom-runner": {
        "enabled": true,
        "config": {
          "api_key": "secret",
          "endpoint": "https://api.example.com"
        }
      }
    }
  }
}
```

Plugin management commands:
- `ralph plugin list`: List discovered plugins
- `ralph plugin validate`: Validate plugin manifests
- `ralph plugin install <path> --scope project|global`: Install a plugin
- `ralph plugin uninstall <id> --scope project|global`: Uninstall a plugin

See [Plugin Development Guide](../plugin-development.md) for creating custom plugins.

## Profiles

CueLoop always exposes two built-in profiles:

- `safe`: recommended default. Uses safer approval defaults and `git_publish_mode = "off"`.
- `power-user`: preserves the higher-autonomy path with `approval_mode = "yolo"` and `git_publish_mode = "commit_and_push"`.

You can inspect resolved profiles with:

```bash
ralph config profiles
```

User-defined profiles remain additive. `safe` and `power-user` are reserved names in `0.3`; defining either in config is a validation error.

Configuration profiles enable quick switching between different workflow presets without manually editing config or passing many CLI flags for each invocation.

A profile is an `AgentConfig`-shaped patch that is applied over the base `agent` configuration when selected via `--profile <NAME>`.

Define custom profiles in your config file under the `profiles` key:

```json
{
  "version": 2,
  "profiles": {
    "fast-local": {
      "runner": "codex",
      "model": "gpt-5.4",
      "phases": 1,
      "reasoning_effort": "low"
    },
    "deep-review": {
      "runner": "codex",
      "model": "gpt-5.4",
      "phases": 3,
      "reasoning_effort": "high"
    }
  }
}
```

### Profile Precedence

When a profile is selected, the final configuration is computed in this order (highest to lowest):

1. **CLI flags** (e.g., `--runner`, `--model`, `--phases`, `--effort`)
2. **Task overrides** (`task.agent.*` in the queue)
3. **Selected profile** (config-defined)
4. **Base config** (merged global + project config)

This means:
- CLI flags always win
- A profile can be partially overridden by CLI flags

### Using Profiles

Select a profile using the `--profile` flag:

```bash
# Run with a custom fast-local profile
ralph run one --profile fast-local

# Scan with a deep-review profile
ralph scan --profile deep-review "security audit"

# Override specific settings while using a profile
ralph run one --profile fast-local --phases 2 --runner claude

# List available profiles
ralph config profiles list

# Inspect a specific profile
ralph config profiles show fast-local
```

### Profile Inheritance

Profiles are merged into the base config using the same leaf-wise merge semantics as config layers:

- `Some(value)` in the profile overrides the base config
- `None` or omitted fields inherit from the base config

This means a profile only needs to specify the fields it wants to change:

```json
{
  "profiles": {
    "fast-local": {
      "phases": 1
    }
  }
}
```

The above profile only changes `phases`, leaving all other `agent` settings at their base values.

### Migration from Retired Default Names

`quick` and `thorough` are no longer built in. If you want those names back for your team, define them explicitly:

```json
{
  "profiles": {
    "quick": {
      "runner": "codex",
      "model": "gpt-5.4",
      "phases": 1,
      "reasoning_effort": "low"
    },
    "thorough": {
      "runner": "codex",
      "model": "gpt-5.4",
      "phases": 3,
      "reasoning_effort": "high"
    }
  }
}
```
