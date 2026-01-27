#!/usr/bin/env bash
# runner_cli_inventory.sh
#
# Responsible for capturing `--help` (and best-effort `--version`) output for the
# runner CLIs Ralph uses: `codex`, `opencode`, `gemini`, `claude`, and Cursor's
# `agent`. Outputs are written into a stable directory structure to support
# Phase 2 discovery and later Phase 3 runner unification work.
#
# Does NOT:
# - Validate that a runner is correctly configured/authenticated
# - Execute any runner workloads beyond help/version commands
# - Parse or interpret help text (that belongs in docs/runner_cli_inventory.md)
#
# Assumptions / invariants:
# - Runner binaries are either on PATH or provided via `--bin NAME=PATH`
# - Captured help/version output may be written to stdout or stderr; we capture
#   both (`2>&1`)
# - The script is dependency-free beyond bash + coreutils

set -euo pipefail

usage() {
  cat <<'EOF'
runner_cli_inventory.sh

Capture `--help` (and best-effort `--version`) outputs for runner binaries used by Ralph.

USAGE:
  scripts/runner_cli_inventory.sh [--out DIR] [--bin NAME=PATH]...

OPTIONS:
  --out DIR
      Output directory for captured help/version text.
      Default: target/tmp/runner_cli_inventory

  --bin NAME=PATH
      Override a runner binary name/path.
      May be provided multiple times.

      NAME must be one of: codex, opencode, gemini, claude, agent

  -h, --help
      Print this help.

OUTPUT:
  Writes per-runner files under:
    <out>/<runner>/

  Including:
    resolved_path.txt
    version.txt (best effort)
    help.*.txt (one per captured help command)

EXAMPLES:
  scripts/runner_cli_inventory.sh
  scripts/runner_cli_inventory.sh --out target/tmp/runner_cli_inventory
  scripts/runner_cli_inventory.sh --bin agent=/Applications/Cursor.app/Contents/Resources/app/bin/agent
EOF
}

OUT_DIR="target/tmp/runner_cli_inventory"

BIN_OVERRIDE_CODEX=""
BIN_OVERRIDE_OPENCODE=""
BIN_OVERRIDE_GEMINI=""
BIN_OVERRIDE_CLAUDE=""
BIN_OVERRIDE_AGENT=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --out)
      OUT_DIR="${2:-}"
      shift 2
      ;;
    --bin)
      kv="${2:-}"
      if [[ -z "$kv" || "$kv" != *"="* ]]; then
        echo "ERROR: --bin requires NAME=PATH (got: '$kv')" >&2
        exit 2
      fi
      name="${kv%%=*}"
      path="${kv#*=}"
      case "$name" in
        codex|opencode|gemini|claude|agent) ;;
        *)
          echo "ERROR: --bin NAME must be one of: codex, opencode, gemini, claude, agent (got: '$name')" >&2
          exit 2
          ;;
      esac
      case "$name" in
        codex) BIN_OVERRIDE_CODEX="$path" ;;
        opencode) BIN_OVERRIDE_OPENCODE="$path" ;;
        gemini) BIN_OVERRIDE_GEMINI="$path" ;;
        claude) BIN_OVERRIDE_CLAUDE="$path" ;;
        agent) BIN_OVERRIDE_AGENT="$path" ;;
      esac
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "ERROR: unknown argument: $1" >&2
      echo >&2
      usage >&2
      exit 2
      ;;
  esac
done

mkdir -p "$OUT_DIR"

run_and_capture() {
  local runner="$1"
  local label="$2"
  shift 2
  local dir="$OUT_DIR/$runner"
  local file="$dir/help.${label}.txt"

  mkdir -p "$dir"

  (
    set +e
    echo "=== runner: $runner"
    echo "=== label: $label"
    echo "=== cmd: $*"
    echo "=== captured_at: $(date -u +"%Y-%m-%dT%H:%M:%SZ")"
    echo
    "$@" 2>&1
    cmd_status=$?
    echo
    if [[ "$cmd_status" -ne 0 ]]; then
      echo "=== ERROR: command failed (exit=$cmd_status)"
    fi
    exit "$cmd_status"
  ) > "$file"
}

capture_version_best_effort() {
  local runner="$1"
  local bin="$2"
  local dir="$OUT_DIR/$runner"
  local file="$dir/version.txt"
  mkdir -p "$dir"

  # Try a handful of common version invocations; write the first that succeeds.
  local candidates=(
    "--version"
    "version"
    "-V"
  )

  {
    echo "=== runner: $runner"
    echo "=== bin: $bin"
    echo "=== captured_at: $(date -u +"%Y-%m-%dT%H:%M:%SZ")"
    echo
  } > "$file"

  for arg in "${candidates[@]}"; do
    if "$bin" $arg >/dev/null 2>&1; then
      {
        echo "=== cmd: $bin $arg"
        "$bin" $arg 2>&1
      } >> "$file"
      return 0
    fi
  done

  {
    echo "=== WARNING: no supported version flag detected (tried: ${candidates[*]})"
    echo "=== NOTE: this is not fatal; rely on help text headers if they include versions."
  } >> "$file"
  return 0
}

resolve_bin() {
  local runner="$1"
  case "$runner" in
    codex)
      if [[ -n "$BIN_OVERRIDE_CODEX" ]]; then
        echo "$BIN_OVERRIDE_CODEX"
        return 0
      fi
      ;;
    opencode)
      if [[ -n "$BIN_OVERRIDE_OPENCODE" ]]; then
        echo "$BIN_OVERRIDE_OPENCODE"
        return 0
      fi
      ;;
    gemini)
      if [[ -n "$BIN_OVERRIDE_GEMINI" ]]; then
        echo "$BIN_OVERRIDE_GEMINI"
        return 0
      fi
      ;;
    claude)
      if [[ -n "$BIN_OVERRIDE_CLAUDE" ]]; then
        echo "$BIN_OVERRIDE_CLAUDE"
        return 0
      fi
      ;;
    agent)
      if [[ -n "$BIN_OVERRIDE_AGENT" ]]; then
        echo "$BIN_OVERRIDE_AGENT"
        return 0
      fi
      ;;
  esac
  if command -v "$runner" >/dev/null 2>&1; then
    command -v "$runner"
    return 0
  fi
  # Fall back to raw name; capture attempt will fail and be recorded.
  echo "$runner"
}

write_resolved_path() {
  local runner="$1"
  local bin="$2"
  local dir="$OUT_DIR/$runner"
  mkdir -p "$dir"
  {
    echo "=== runner: $runner"
    echo "=== resolved_bin: $bin"
    echo "=== captured_at: $(date -u +"%Y-%m-%dT%H:%M:%SZ")"
  } > "$dir/resolved_path.txt"
}

# We intentionally include subcommand helps relevant to Ralph’s current usage:
# - codex: `exec`, `exec resume`
# - opencode: `run`
declare -a RUNNERS=("codex" "opencode" "gemini" "claude" "agent")

echo "Runner CLI inventory: start"
echo "Output dir: $OUT_DIR"
echo

failures=0

for runner in "${RUNNERS[@]}"; do
  bin="$(resolve_bin "$runner")"
  write_resolved_path "$runner" "$bin"

  echo "==> $runner"
  echo "    bin: $bin"

  # Version capture is best-effort and never fatal.
  capture_version_best_effort "$runner" "$bin" || true

  # Always capture base help (fatal only if we cannot execute at all).
  if ! run_and_capture "$runner" "base" "$bin" "--help"; then
    echo "    ERROR: failed to run '$bin --help' (see $OUT_DIR/$runner/help.base.txt)" >&2
    failures=$((failures + 1))
    echo
    continue
  fi

  # Runner-specific subcommand help captures (non-fatal if unsupported).
  case "$runner" in
    codex)
      run_and_capture "$runner" "exec" "$bin" "exec" "--help" || true
      run_and_capture "$runner" "exec_resume" "$bin" "exec" "resume" "--help" || true
      ;;
    opencode)
      run_and_capture "$runner" "run" "$bin" "run" "--help" || true
      ;;
    agent)
      # Cursor `agent` CLI may have subcommands; try common ones non-fatally.
      run_and_capture "$runner" "run" "$bin" "run" "--help" || true
      run_and_capture "$runner" "resume" "$bin" "resume" "--help" || true
      ;;
    gemini|claude)
      # Some CLIs expose subcommands; harmless to attempt non-fatally.
      run_and_capture "$runner" "resume" "$bin" "resume" "--help" || true
      ;;
  esac

  echo "    captured: $OUT_DIR/$runner/"
  echo
done

echo "Runner CLI inventory: complete"
if [[ "$failures" -gt 0 ]]; then
  echo "WARNING: $failures runner(s) failed base --help capture. See files under: $OUT_DIR" >&2
  exit 1
fi
