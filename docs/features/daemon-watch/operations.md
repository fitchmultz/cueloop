# Daemon and Watch Operations
Status: Active
Owner: Maintainers
Source of truth: this document for combined daemon/watch operating patterns
Parent: [Daemon and Watch](../daemon-and-watch.md)

This guide covers running daemon mode and watch mode together for automated task discovery and execution.

## Combined Usage Patterns

### Full Auto-Development Setup

Run both daemon and watch for fully automated task management:

```bash
# Terminal 1: Start daemon for task execution
cueloop daemon start --notify-when-unblocked

# Terminal 2: Start watch for task detection
cueloop watch --auto-queue --close-removed --notify

# Terminal 3: Regular development
# - Add TODOs → Tasks auto-created
# - Daemon executes tasks
# - Complete work → Tasks auto-closed
```

### Service-Based Setup

```bash
# Start daemon as user service
systemctl --user start cueloop

# Run watch in tmux/screen session
tmux new-session -d -s cueloop-watch "cueloop watch --auto-queue --close-removed"

# Check on it later
tmux attach -t cueloop-watch
```

---

## See Also

- [Daemon and Watch overview](../daemon-and-watch.md)
- [CLI Reference](../../cli.md)
- [Queue and Tasks](../../queue-and-tasks.md)
- [Daemon Mode](./daemon.md)
- [Watch Mode](./watch.md)
- [Troubleshooting](./troubleshooting.md)
