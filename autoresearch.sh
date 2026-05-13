#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "$0")" && pwd)"
cd "$repo_root"

probe_path="scripts/agent-ci-surface.sh"
probe_marker="# AUTORESEARCH_PROBE"

python3 - <<'PY'
from pathlib import Path
p = Path('scripts/agent-ci-surface.sh')
text = p.read_text()
marker = '# AUTORESEARCH_PROBE\n'
needle = 'set -euo pipefail\n'
if marker not in text:
    if needle not in text:
        raise SystemExit('missing insertion point in scripts/agent-ci-surface.sh')
    text = text.replace(needle, needle + marker, 1)
    p.write_text(text)
PY

surface_target="$(bash scripts/agent-ci-surface.sh --target)"
case "$surface_target" in
  noop) surface_target_code=0 ;;
  ci-docs) surface_target_code=1 ;;
  ci-fast) surface_target_code=2 ;;
  ci) surface_target_code=3 ;;
  macos-ci) surface_target_code=4 ;;
  *) echo "unknown surface target: $surface_target" >&2; exit 1 ;;
esac

classifier_ms="$(python3 - <<'PY'
import subprocess, time
start = time.perf_counter_ns()
subprocess.run(['bash','scripts/agent-ci-surface.sh','--target'], check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
end = time.perf_counter_ns()
print((end-start)//1_000_000)
PY
)"

run_log="$(mktemp "${TMPDIR:-/tmp}/autoresearch-agent-ci.XXXXXX")"
status_code=0
start_ns="$(python3 - <<'PY'
import time
print(time.perf_counter_ns())
PY
)"
if ! make agent-ci >"$run_log" 2>&1; then
  status_code=$?
fi
end_ns="$(python3 - <<'PY'
import time
print(time.perf_counter_ns())
PY
)"
agent_ci_ms="$(python3 - <<PY
start_ns = int('$start_ns')
end_ns = int('$end_ns')
print((end_ns - start_ns)//1_000_000)
PY
)"
stdout_bytes="$(wc -c <"$run_log" | tr -d '[:space:]')"

echo "surface_target=$surface_target"
echo "status_code=$status_code"
echo "classifier_ms=$classifier_ms"
echo "agent_ci_ms=$agent_ci_ms"
echo "stdout_bytes=$stdout_bytes"
tail -n 20 "$run_log" || true
rm -f "$run_log"

if [ "$status_code" -ne 0 ]; then
  exit "$status_code"
fi

echo "METRIC agent_ci_ms=$agent_ci_ms"
echo "METRIC classifier_ms=$classifier_ms"
echo "METRIC surface_target_code=$surface_target_code"
echo "METRIC stdout_bytes=$stdout_bytes"
echo "METRIC status_code=$status_code"
