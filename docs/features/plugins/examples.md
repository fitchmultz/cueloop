# Plugin Examples
Status: Active
Owner: Maintainers
Source of truth: this document for example plugin implementations
Parent: [CueLoop Plugin System](../plugins.md)

Purpose: Provide concrete runner and processor plugin examples that demonstrate the documented protocols.

---

## Example 1: Custom Runner Plugin

Directory:

```text
~/.config/cueloop/plugins/custom-api/
├── plugin.json
└── runner.sh
```

`plugin.json`:

```json
{
  "api_version": 1,
  "id": "custom.api",
  "version": "1.0.0",
  "name": "Custom API Runner",
  "description": "Forwards prompts to custom HTTP API",
  "runner": {
    "bin": "runner.sh",
    "supports_resume": false,
    "default_model": "gpt-4"
  }
}
```

`runner.sh` (safe JSON emission with `jq -n`):

```bash
#!/bin/bash
set -euo pipefail

MODEL=""
OUTPUT_FORMAT=""
SESSION_ID=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    run|resume) shift ;;
    --model) MODEL="$2"; shift 2 ;;
    --output-format) OUTPUT_FORMAT="$2"; shift 2 ;;
    --session) SESSION_ID="$2"; shift 2 ;;
    *) shift ;;
  esac
done

API_ENDPOINT=$(echo "$CUELOOP_PLUGIN_CONFIG_JSON" | jq -r '.endpoint // "https://api.example.com/v1"')
API_KEY=$(echo "$CUELOOP_PLUGIN_CONFIG_JSON" | jq -r '.api_key // empty')
PROMPT=$(cat)

if [ -n "$SESSION_ID" ]; then
  jq -cn --arg id "$SESSION_ID" '{type:"session", id:$id}'
fi

RESPONSE=$(curl -fsS -X POST "$API_ENDPOINT/chat" \
  -H "Authorization: Bearer $API_KEY" \
  -H "Content-Type: application/json" \
  -d "$(jq -cn --arg model "$MODEL" --arg prompt "$PROMPT" '{model:$model,messages:[{role:"user",content:$prompt}]}')")

CONTENT=$(echo "$RESPONSE" | jq -r '.choices[0].message.content // empty')
if [ -n "$CONTENT" ]; then
  jq -cn --arg text "$CONTENT" '{role:"assistant", content:[{type:"text", text:$text}]}'
fi
```

## Example 2: Task Validation Processor (`validate_task`)

```bash
#!/bin/bash
set -euo pipefail
HOOK="$1"
TASK_ID="$2"
FILE="$3"

if [ "$HOOK" = "validate_task" ]; then
  TITLE=$(jq -r '.title // empty' "$FILE")
  SCOPE=$(jq -c '.scope // []' "$FILE")

  [ -n "$TITLE" ] || { echo "Error: Task $TASK_ID has no title" >&2; exit 1; }
  [ "$SCOPE" != "[]" ] || { echo "Error: Task $TASK_ID must have scope defined" >&2; exit 1; }
fi
```

## Example 3: Pre-Prompt Enhancement (`pre_prompt`)

```bash
#!/bin/bash
set -euo pipefail
HOOK="$1"
TASK_ID="$2"
FILE="$3"

if [ "$HOOK" = "pre_prompt" ]; then
  BRANCH=$(git branch --show-current 2>/dev/null || echo "unknown")
  RULES=$(echo "$CUELOOP_PLUGIN_CONFIG_JSON" | jq -r '.rules // ""')

  cat >> "$FILE" <<EOF

---
Repository Context:
- Current Branch: $BRANCH

Coding Rules:
$RULES
EOF
fi
```

## Example 4: Post-Run Logger (`post_run`)

```bash
#!/bin/bash
set -euo pipefail
HOOK="$1"
TASK_ID="$2"
FILE="$3"

if [ "$HOOK" = "post_run" ]; then
  LOG_PATH=$(echo "$CUELOOP_PLUGIN_CONFIG_JSON" | jq -r --arg home "$HOME" '.log_path // ($home + "/.cueloop/task-completions.log")')
  TIMESTAMP=$(date -Iseconds)
  TOOL_COUNT=$(grep -c '"type": "tool_use"' "$FILE" 2>/dev/null || echo "0")

  mkdir -p "$(dirname "$LOG_PATH")"
  echo "[$TIMESTAMP] Task $TASK_ID completed" >> "$LOG_PATH"
  echo "  - Tool calls: $TOOL_COUNT" >> "$LOG_PATH"
fi
```

Choose log paths intentionally: repo-local paths can make the workspace dirty.

## Related Docs

- [Runner Protocol](runner-protocol.md)
- [Processor Protocol](processor-protocol.md)
- [Plugin Security](security.md)
