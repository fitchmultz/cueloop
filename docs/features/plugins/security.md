# Plugin Security
Status: Active
Owner: Maintainers
Source of truth: this document for plugin-specific trust, sandboxing, path, and redaction guidance
Parent: [CueLoop Plugin System](../plugins.md)

Purpose: Explain the risks and required safety practices for enabling and running plugins.

---

> ⚠️ **Plugins are not sandboxed.** Enabling a plugin is equivalent to trusting it with full system access.

Plugins can execute arbitrary commands, access files/environment variables, make network requests, and modify repository contents.

## Core Safety Rules

1. **Explicit enablement is required**
   - Discovery alone does not activate plugins.
   - Plugins default to disabled.
2. **Trust boundaries matter**
   - Project plugins override global plugins with the same ID.
   - Project-scope plugins run only in trusted repos; untrusted repos ignore `.ralph/plugins/*` at runtime.
3. **Path confinement is enforced**
   - Manifest executable paths must be plugin-dir-relative.
   - Absolute paths and `..` escapes are rejected.
   - Existing symlinked files and ancestor directories must canonicalize within the plugin directory.
4. **Treat plugin config as sensitive**
   - CueLoop redacts sensitive plugin stderr before display.
   - Do not log full `RALPH_PLUGIN_CONFIG_JSON` from plugin scripts.

## Operator Checklist

```bash
cueloop plugin validate --id suspicious.plugin
cueloop plugin list --json
```

Review plugin source before enabling it.

## Related Docs

- [Plugin Architecture and Manifest](architecture.md)
- [Plugin Operations](operations.md)
- [Security Features](../security.md)
- [Security Policy](../../../SECURITY.md)
