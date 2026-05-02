#!/usr/bin/env bash
# Cursor SDK smoke test for CueLoop integration assumptions (local run + resume).
#
# Requirements:
# - `node` on PATH, or set CURSOR_SDK_NODE_BIN
# - `@cursor/sdk@1.0.11` resolvable from WORKDIR, or set CUELOOP_CURSOR_SDK_MODULE_PATH
# - CURSOR_API_KEY exported in the environment
#
# Model: composer-2 only (project policy for this smoke script).
#
# Usage:
#   ./scripts/cursor-agent-runner-smoke.sh [WORKDIR]
#
set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  scripts/cursor-agent-runner-smoke.sh [WORKDIR]

Smoke-tests CueLoop's Cursor SDK assumptions with a local run and resume.

Requirements:
  - node on PATH, or CURSOR_SDK_NODE_BIN set to a Node 18+ executable
  - @cursor/sdk@1.0.11 installed in WORKDIR, or CUELOOP_CURSOR_SDK_MODULE_PATH set
  - CURSOR_API_KEY exported

Examples:
  scripts/cursor-agent-runner-smoke.sh
  scripts/cursor-agent-runner-smoke.sh .
  CURSOR_SDK_NODE_BIN=/opt/homebrew/bin/node scripts/cursor-agent-runner-smoke.sh /path/to/repo

Exit codes:
  0  Smoke passed
  1  Smoke ran but failed
  2  Required binary, environment, or argument is missing
EOF
}

case "${1:-}" in
  -h|--help)
    usage
    exit 0
    ;;
esac

WORKDIR_INPUT="${1:-$(pwd)}"
WORKDIR="$(cd "$WORKDIR_INPUT" && pwd -P)"
MODEL="composer-2"
EXPECTED_SDK_VERSION="1.0.11"
BIN="${CURSOR_SDK_NODE_BIN:-node}"

if ! command -v "$BIN" >/dev/null 2>&1; then
  echo "error: '$BIN' not found on PATH (set CURSOR_SDK_NODE_BIN to override)" >&2
  exit 2
fi

if [[ -z "${CURSOR_API_KEY:-}" ]]; then
  echo "error: CURSOR_API_KEY is required for Cursor SDK local runs" >&2
  exit 2
fi

OUT="$(mktemp -t cueloop-cursor-smoke-out.XXXXXX)"
ERR="$(mktemp -t cueloop-cursor-smoke-err.XXXXXX)"
SCRIPT="$(mktemp -t cueloop-cursor-sdk-smoke.XXXXXX.mjs)"
cleanup() {
  rm -f "$OUT" "$ERR" "$SCRIPT"
}
trap cleanup EXIT

cat >"$SCRIPT" <<'JS'
import { createRequire } from "node:module";
import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import { pathToFileURL } from "node:url";

async function loadSdk(cwd, expectedVersion) {
  if (process.env.CUELOOP_CURSOR_SDK_MODULE_PATH) {
    const configured = path.resolve(process.env.CUELOOP_CURSOR_SDK_MODULE_PATH);
    assertSdkVersion(configured, expectedVersion);
    return normalizeSdkModule(await import(pathToFileURL(configured).href));
  }
  const requireFromCwd = createRequire(path.join(cwd, "package.json"));
  const resolved = requireFromCwd.resolve("@cursor/sdk", { paths: [cwd] });
  assertSdkVersion(resolved, expectedVersion);
  return normalizeSdkModule(await import(pathToFileURL(resolved).href));
}

function assertSdkVersion(entrypoint, expectedVersion) {
  const packageJsonPath = findSdkPackageJson(entrypoint);
  if (!packageJsonPath) {
    throw new Error(`Unable to find @cursor/sdk package metadata for ${entrypoint}; install @cursor/sdk@${expectedVersion}`);
  }
  const pkg = JSON.parse(fs.readFileSync(packageJsonPath, "utf8"));
  if (pkg.version !== expectedVersion) {
    throw new Error(`@cursor/sdk ${pkg.version ?? "unknown"} is unsupported; install @cursor/sdk@${expectedVersion}`);
  }
}

function findSdkPackageJson(entrypoint) {
  let current = path.resolve(entrypoint);
  if (!fs.existsSync(current)) {
    return null;
  }
  if (!fs.statSync(current).isDirectory()) {
    current = path.dirname(current);
  }
  while (true) {
    const candidate = path.join(current, "package.json");
    if (fs.existsSync(candidate)) {
      try {
        const pkg = JSON.parse(fs.readFileSync(candidate, "utf8"));
        if (pkg.name === "@cursor/sdk") {
          return candidate;
        }
      } catch {
        return null;
      }
    }
    const parent = path.dirname(current);
    if (parent === current) {
      return null;
    }
    current = parent;
  }
}

function normalizeSdkModule(moduleNamespace) {
  const candidates = [
    moduleNamespace,
    moduleNamespace?.default,
    moduleNamespace?.default?.default,
  ];
  const sdk = candidates.find((candidate) => candidate?.Agent);
  if (!sdk) {
    throw new Error("Loaded @cursor/sdk module does not expose Agent");
  }
  return sdk;
}

function assistantText(event) {
  if (event?.type !== "assistant" || !Array.isArray(event.message?.content)) {
    return "";
  }
  return event.message.content
    .filter((block) => block?.type === "text" && typeof block.text === "string")
    .map((block) => block.text)
    .join("");
}

const cwd = process.argv[2];
const model = process.argv[3];
const expectedVersion = process.argv[4];
const { Agent } = await loadSdk(cwd, expectedVersion);
const agent = await Agent.create({
  apiKey: process.env.CURSOR_API_KEY,
  model: { id: model },
  local: { cwd, settingSources: ["project", "user", "plugins"] },
});
console.log(JSON.stringify({ type: "system", subtype: "init", session_id: agent.agentId }));
const run = await agent.send("Reply with exactly: CURSOR_SMOKE_SESSION");
let firstStreamText = "";
for await (const event of run.stream()) {
  firstStreamText += assistantText(event);
  console.log(JSON.stringify(event));
}
const result = await run.wait();
console.log(JSON.stringify({ type: "result", session_id: agent.agentId, status: result.status, stream_text: firstStreamText }));

const resumed = await Agent.resume(agent.agentId, {
  apiKey: process.env.CURSOR_API_KEY,
  model: { id: model },
  local: { cwd, settingSources: ["project", "user", "plugins"] },
});
const run2 = await resumed.send("Reply with exactly: CURSOR_SMOKE_RESUME");
let resumeStreamText = "";
for await (const event of run2.stream()) {
  resumeStreamText += assistantText(event);
  console.log(JSON.stringify(event));
}
const result2 = await run2.wait();
console.log(JSON.stringify({ type: "result", session_id: resumed.agentId, status: result2.status, stream_text: resumeStreamText }));
if (!resumeStreamText.includes("CURSOR_SMOKE_RESUME")) {
  process.exit(1);
}
JS

echo "== Node version"
"$BIN" --version

echo "== Cursor SDK local run + resume (model=$MODEL)"
cd "$WORKDIR"
set +e
"$BIN" "$SCRIPT" "$WORKDIR" "$MODEL" "$EXPECTED_SDK_VERSION" >"$OUT" 2>"$ERR"
status=$?
set -e
if [[ "$status" -ne 0 ]]; then
  echo "error: Cursor SDK smoke failed (exit $status)" >&2
  cat "$ERR" >&2 || true
  exit 1
fi

SESSION_ID="$(
  python3 - "$OUT" <<'PY'
import json
import sys

path = sys.argv[1]
last = None
with open(path, "r", encoding="utf-8") as handle:
    for raw in handle:
        line = raw.strip()
        if not line:
            continue
        try:
            payload = json.loads(line)
        except json.JSONDecodeError:
            continue
        session_id = payload.get("session_id")
        if isinstance(session_id, str) and session_id.strip():
            last = session_id.strip()

if not last:
    sys.exit(2)
print(last)
PY
)"

if [[ "$SESSION_ID" == "" ]]; then
  echo "error: could not extract session_id from stream-json output" >&2
  tail -n 50 "$OUT" >&2 || true
  exit 1
fi

if ! grep -q "CURSOR_SMOKE_RESUME" "$OUT"; then
  echo "error: expected resume output to mention CURSOR_SMOKE_RESUME" >&2
  tail -n 50 "$OUT" >&2 || true
  exit 1
fi

echo "ok: cursor SDK local run + resume smoke passed ($SESSION_ID)"
