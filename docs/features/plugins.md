# Ralph Plugin System
Status: Active
Owner: Maintainers
Source of truth: this document is the canonical entry point for plugin-system documentation; linked child pages are canonical for their named topic.
Parent: [Feature Documentation](README.md)

![Plugin Architecture](../assets/images/2026-03-10-12-05-00-plugin-architecture.png)

Purpose: Entry point for Ralph's plugin system, including custom runners and task processors.

> ⚠️ **Critical security warning:** Plugins are not sandboxed. Enabling a plugin grants full system access equivalent to running arbitrary code. Only enable plugins from trusted sources. See [Plugin Security](plugins/security.md).

---

## Overview

Ralph plugins extend the CLI without modifying core code.

| Type | Purpose | Primary reference |
|------|---------|-------------------|
| Runner | Execute prompts against custom AI backends | [Runner Protocol](plugins/runner-protocol.md) |
| Processor | Hook into task lifecycle events | [Processor Protocol](plugins/processor-protocol.md) |

## Documentation Map

| Topic | Use this when... |
|-------|------------------|
| [Architecture and Manifest](plugins/architecture.md) | You need discovery paths, directory layout, manifest fields, or precedence rules. |
| [Runner Protocol](plugins/runner-protocol.md) | You are implementing or debugging a custom runner. |
| [Processor Protocol](plugins/processor-protocol.md) | You are implementing validation, prompt, or post-run hooks. |
| [Operations](plugins/operations.md) | You need to scaffold, install, enable, list, validate, configure, or uninstall plugins. |
| [Plugin Security](plugins/security.md) | You need trust, sandboxing, path, or redaction guidance. |
| [Examples](plugins/examples.md) | You want concrete runner and processor plugin examples. |
| [Troubleshooting and Compatibility](plugins/troubleshooting.md) | You are debugging discovery, execution, manifests, or API compatibility. |

## Quick Start

```bash
ralph plugin init my.plugin
ralph plugin validate --id my.plugin
ralph plugin list
```

Plugins are discovered but disabled by default. Enable a plugin explicitly in config:

```json
{
  "version": 1,
  "plugins": {
    "plugins": {
      "my.plugin": {
        "enabled": true
      }
    }
  }
}
```

## See Also

- [Plugin Development Guide](../plugin-development.md)
- [Configuration](../configuration.md#plugin-configuration)
- [CLI Reference](../cli.md)
- [Security Features](security.md)
