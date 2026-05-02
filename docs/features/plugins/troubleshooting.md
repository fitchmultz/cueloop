# Plugin Troubleshooting and Compatibility
Status: Active
Owner: Maintainers
Source of truth: this document for plugin debugging, common failures, API compatibility, and general best practices
Parent: [CueLoop Plugin System](../plugins.md)

Purpose: Help operators and plugin authors diagnose plugin discovery, manifest, runtime, and compatibility issues.

---

## Debugging Commands

```bash
RUST_LOG=debug cueloop plugin list
RUST_LOG=trace cueloop run one
cueloop plugin list --json
cueloop plugin validate --id my.plugin
```

## Common Issues

- **Plugin not discovered**
  - Confirm `<plugin_root>/<plugin_id>/plugin.json` exists.
  - Confirm executable bits are set for scripts.
- **Plugin not executing**
  - Confirm `plugins.plugins.<id>.enabled: true`.
  - Confirm repo trust allows project-scope plugins.
- **Runner not found**
  - Verify manifest executable paths and confinement rules in [Architecture and Manifest](architecture.md).
  - Re-check safety/path constraints in [Plugin Security](security.md).
- **Processor hook failing**
  - Check non-zero exit code and redacted stderr.
  - Reproduce manually using hook arguments from [Processor Protocol](processor-protocol.md).
- **Operational command confusion**
  - Re-check command usage in [Plugin Operations](operations.md).

## API Version Compatibility

The current plugin API version is **`1`**.

CueLoop rejects incompatible plugin manifests, for example: `got 2, expected 1`.

| API Version | CueLoop Versions | Status |
|-------------|----------------|--------|
| 1 | Current | ✅ Supported |

## Best Practices

- Use semantic versioning in plugin manifests.
- Handle missing config (`CUELOOP_PLUGIN_CONFIG_JSON` may be `{}`).
- Keep runner/processor behaviors idempotent where possible.
- Emit clear stderr on failures.
- Stream output incrementally for runners.

## Related Docs

- [Plugin Operations](operations.md)
- [Runner Protocol](runner-protocol.md)
- [Processor Protocol](processor-protocol.md)
- [Plugin Development Guide](../../plugin-development.md)
