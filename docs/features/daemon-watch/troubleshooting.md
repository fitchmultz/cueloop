# Daemon and Watch Troubleshooting
Status: Active
Owner: Maintainers
Source of truth: this document for daemon/watch troubleshooting workflows
Parent: [Daemon and Watch](../daemon-and-watch.md)

Use this guide to diagnose and recover common daemon and watch issues.

## Troubleshooting

### Daemon Issues

| Issue | Solution |
|-------|----------|
| `Daemon is already running` | Run `cueloop daemon status` to verify, then `cueloop daemon stop` if needed |
| `Daemon failed to start` | Check `.cueloop/logs/daemon.log` for errors |
| Stale state file | Run `cueloop daemon status` to auto-clean, or manually remove `.cueloop/cache/daemon.json` |
| Won't stop gracefully | Use `kill -9 <PID>` as last resort |

### Watch Issues

| Issue | Solution |
|-------|----------|
| Not detecting changes | Check `--patterns` match your files; verify file is in watched path |
| Too many tasks created | Enable deduplication by ensuring tasks have `watch.fingerprint` |
| High CPU usage | Increase `--debounce-ms` to reduce processing frequency |
| Missing comments | Check `--comments` includes the types you're using |

### Common Patterns

```bash
# Check daemon logs
tail -f .cueloop/logs/daemon.log

# Verify watch is detecting files
cueloop watch --patterns "*.rs"  # Run interactively to see output

# Clean up and restart
cueloop daemon stop
rm -f .cueloop/cache/daemon.json
rm -f .cueloop/cache/stop_requested
cueloop daemon start
```

---

## See Also

- [Daemon and Watch overview](../daemon-and-watch.md)
- [CLI Reference](../../cli.md)
- [Queue and Tasks](../../queue-and-tasks.md)
- [Daemon Mode](./daemon.md)
- [Watch Mode](./watch.md)
- [Operations](./operations.md)
