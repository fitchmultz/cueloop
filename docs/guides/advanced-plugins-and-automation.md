# Advanced Plugins and Automation
Status: Active
Owner: Maintainers
Source of truth: this document for advanced plugin development and automation setup guidance
Parent: [Advanced Usage Guide](advanced.md)


Purpose: Deep-dive guidance for creating custom CueLoop plugins, debugging plugin execution, running CueLoop as a daemon, integrating watch mode, and wiring automation surfaces such as CI/CD and webhooks.

---

## Table of Contents

1. [Plugin Development](#plugin-development)
2. [Automation](#automation)

---

## Plugin Development

### Creating a Custom Runner

**Step 1: Scaffold the plugin**
```bash
cueloop plugin init mycompany.custom-llm --with-runner --scope global
```

**Step 2: Implement the runner**

```python
#!/usr/bin/env python3
# ~/.config/cueloop/plugins/mycompany.custom-llm/runner.py

import json
import sys
import os
import urllib.request

def main():
    command = sys.argv[1]

    # Parse arguments
    args = {}
    i = 2
    while i < len(sys.argv):
        if sys.argv[i].startswith('--'):
            key = sys.argv[i][2:].replace('-', '_')
            if i + 1 < len(sys.argv) and not sys.argv[i + 1].startswith('--'):
                args[key] = sys.argv[i + 1]
                i += 2
            else:
                args[key] = True
                i += 1
        else:
            i += 1

    # Read prompt from stdin
    prompt = sys.stdin.read()

    # Get config from environment
    config = json.loads(os.environ.get('CUELOOP_PLUGIN_CONFIG_JSON', '{}'))

    if command == 'run':
        response = call_custom_api(
            config.get('api_url'),
            config.get('api_key'),
            prompt,
            args.get('model', 'default')
        )

        # Output NDJSON format
        print(json.dumps({"type": "text", "content": response}))
        print(json.dumps({"type": "finish", "session_id": None}))

    elif command == 'resume':
        # Implement resume logic if supported
        message = sys.argv[-1]  # Last argument is the message
        print(json.dumps({"type": "text", "content": f"Resumed: {message}"}))
        print(json.dumps({"type": "finish", "session_id": None}))

def call_custom_api(url, key, prompt, model):
    # Your API integration here
    headers = {
        'Authorization': f'Bearer {key}',
        'Content-Type': 'application/json'
    }
    data = json.dumps({'prompt': prompt, 'model': model}).encode()

    req = urllib.request.Request(url, data=data, headers=headers, method='POST')

    try:
        with urllib.request.urlopen(req) as resp:
            result = json.loads(resp.read().decode())
            return result.get('completion', '')
    except Exception as e:
        return f'Error: {e}'

if __name__ == '__main__':
    main()
```

**Step 3: Configure and enable**
```bash
chmod +x ~/.config/cueloop/plugins/mycompany.custom-llm/runner.py
```

```json
{
  "plugins": {
    "plugins": {
      "mycompany.custom-llm": {
        "enabled": true,
        "config": {
          "api_url": "https://api.mycompany.com/v1/complete",
          "api_key": "${CUSTOM_LLM_API_KEY}"
        }
      }
    }
  }
}
```

### Creating a Processor Plugin

**Task Validator Example:**

```python
#!/usr/bin/env python3
# .cueloop/plugins/acme.task-validator/validator.py

import json
import sys
import os

def main():
    hook = sys.argv[1]
    task_id = sys.argv[2]
    filepath = sys.argv[3]

    if hook != "validate_task":
        sys.exit(0)

    with open(filepath) as f:
        task = json.load(f)

    config = json.loads(os.environ.get('CUELOOP_PLUGIN_CONFIG_JSON', '{}'))
    required_tags = config.get('required_tags', [])

    errors = []

    # Policy: High priority tasks must have evidence
    if task.get('priority') == 'high' and not task.get('evidence'):
        errors.append("High priority tasks must have evidence")

    # Policy: Tasks must have at least one required tag
    task_tags = set(task.get('tags', []))
    if required_tags and not any(tag in task_tags for tag in required_tags):
        errors.append(f"Task must have one of: {', '.join(required_tags)}")

    # Policy: Task titles under 80 characters
    if len(task.get('title', '')) > 80:
        errors.append("Task title exceeds 80 characters")

    if errors:
        print(f"Validation failed for {task_id}:", file=sys.stderr)
        for error in errors:
            print(f"  - {error}", file=sys.stderr)
        sys.exit(1)

    print(f"Validation passed for {task_id}")

if __name__ == '__main__':
    main()
```

**Enable with config:**
```json
{
  "plugins": {
    "plugins": {
      "acme.task-validator": {
        "enabled": true,
        "config": {
          "required_tags": ["feature", "bugfix", "refactor"]
        }
      }
    }
  }
}
```

### Plugin Debugging

```bash
# Test runner directly
echo "Test prompt" | ~/.config/cueloop/plugins/my.plugin/runner.sh run --model test

# Test processor hook
~/.config/cueloop/plugins/my.plugin/processor.sh validate_task RQ-0001 /tmp/test-task.json

# Debug environment variables
CUELOOP_LOG=debug cueloop run one --id RQ-0001

# Check plugin discovery
cueloop plugin list
cueloop plugin validate --id my.plugin
```

---

## Automation

### Daemon Mode Setup

**Basic daemon management:**
```bash
# Start daemon
cueloop daemon start

# Check status
cueloop daemon status

# View daemon logs
cueloop daemon logs

# Stop daemon
cueloop daemon stop
```

**systemd Service (Linux):**

Create `~/.config/systemd/user/cueloop.service`:
```ini
[Unit]
Description=CueLoop Daemon
After=network.target

[Service]
Type=simple
WorkingDirectory=/path/to/repo
ExecStart=/home/username/.local/bin/cueloop daemon serve \
  --empty-poll-ms 10000 \
  --wait-poll-ms 1000 \
  --notify-when-unblocked
Restart=always
RestartSec=10

[Install]
WantedBy=default.target
```

Enable and start:
```bash
systemctl --user daemon-reload
systemctl --user enable cueloop
systemctl --user start cueloop
journalctl --user -u cueloop -f
```

**launchd Service (macOS):**

Create `~/Library/LaunchAgents/com.cueloop.daemon.plist`:
```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" 
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.cueloop.daemon</string>
    <key>ProgramArguments</key>
    <array>
        <string>/Users/username/.local/bin/cueloop</string>
        <string>daemon</string>
        <string>serve</string>
        <string>--notify-when-unblocked</string>
    </array>
    <key>WorkingDirectory</key>
    <string>/path/to/repo</string>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
</dict>
</plist>
```

Load and start:
```bash
launchctl load ~/Library/LaunchAgents/com.cueloop.daemon.plist
launchctl start com.cueloop.daemon
```

### Watch Mode Integration

**Auto-capture TODOs:**
```bash
# Terminal 1: Start daemon for execution
cueloop daemon start

# Terminal 2: Watch for TODO/FIXME comments
cueloop watch --auto-queue --close-removed --notify

# Terminal 3: Regular development
# Add "// TODO: refactor this" → Task auto-created
# Remove TODO comment → Task auto-closed
```

### CI/CD Integration

**GitHub Actions Example:**
```yaml
name: CueLoop Task Execution

on:
  schedule:
    - cron: '0 2 * * *'  # Daily at 2 AM
  workflow_dispatch:

jobs:
  cueloop:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Setup CueLoop
        run: |
          curl -fsSL https://cueloop.dev/install.sh | sh
          cueloop doctor --auto-fix

      - name: Run Tasks
        env:
          ANTHROPIC_API_KEY: ${{ secrets.ANTHROPIC_API_KEY }}
        run: |
          cueloop run loop \
            --non-interactive \
            --profile ci-safe \
            --max-tasks 5

      - name: Upload Logs
        if: always()
        uses: actions/upload-artifact@v4
        with:
          name: cueloop-logs
          path: .cueloop/logs/
```

**Pre-commit Hook:**
```bash
#!/bin/bash
# .git/hooks/pre-commit

# Validate queue before commit
if ! cueloop queue validate --quiet; then
    echo "Queue validation failed. Run 'cueloop queue validate' for details."
    exit 1
fi

# Run quick check on critical tasks
cueloop run loop --profile ci-check --max-tasks 1 --non-interactive
```

### Webhook Automation

**Slack notifications for task completion:**
```json
{
  "agent": {
    "webhook": {
      "enabled": true,
      "url": "https://hooks.slack.com/services/...",
      "secret": "${SLACK_WEBHOOK_SECRET}",
      "events": ["task_completed", "task_failed", "loop_stopped"],
      "timeout_secs": 30,
      "retry_count": 3
    }
  }
}
```

**Dashboard integration:**
```json
{
  "agent": {
    "webhook": {
      "enabled": true,
      "url": "https://dashboard.example.com/webhooks/cueloop",
      "events": ["*"],
      "queue_capacity": 500,
      "queue_policy": "block_with_timeout"
    }
  }
}
```
