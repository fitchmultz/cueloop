#!/usr/bin/env bash
# Purpose: Final CueLoop cutover acceptance gate.
# Responsibilities: scan tracked filenames and text for old-brand identifiers, report every remaining hit, and permit only one README naming-switch note.
# Scope: repository hygiene only; it does not rewrite files or validate build/test behavior.
# Usage: run from the repository root via this script or the make target.
# Invariants/Assumptions: git tracked files define the release surface; binary files are skipped for content scanning.
set -euo pipefail

MODE="report"
ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"

usage() {
  cat <<'USAGE'
Usage: scripts/check-cueloop-cutover.sh [--report|--enforce]

Scans tracked repository filenames and text for old-brand identifiers after the
CueLoop cutover. The final accepted state permits exactly one README naming
switch note and no other hits.

Options:
  --report    Print findings and exit 0. Default while the cutover is underway.
  --enforce   Print findings and exit 1 when forbidden or excess allowed hits remain.
  -h, --help  Show this help.

Examples:
  scripts/check-cueloop-cutover.sh --report
  scripts/check-cueloop-cutover.sh --enforce

Exit codes:
  0  No enforced failure, or report mode completed.
  1  Enforce mode found forbidden hits or more than one README naming-switch note.
  2  Invalid arguments or not run inside a git worktree.
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --report) MODE="report"; shift ;;
    --enforce) MODE="enforce"; shift ;;
    -h|--help) usage; exit 0 ;;
    *) echo "Unknown argument: $1" >&2; usage >&2; exit 2 ;;
  esac
done

if [[ ! -d "$ROOT/.git" ]]; then
  echo "Not inside a git worktree: $ROOT" >&2
  exit 2
fi

cd "$ROOT"

python3 - "$MODE" <<'PY'
import os
import re
import subprocess
import sys
from pathlib import Path

mode = sys.argv[1]
old_lower = "ra" + "lph"
old_title = old_lower[:1].upper() + old_lower[1:]
old_upper = old_lower.upper()
old_domain = "com.mitchfultz." + old_lower
old_scheme = old_lower + "://"
old_env_prefix = old_upper + "_"
old_readme_marker = old_upper + "_README_VERSION"

needle_re = re.compile(re.escape(old_lower), re.IGNORECASE)
extra_needles = [old_domain, old_scheme, old_env_prefix, old_readme_marker]

tracked = subprocess.check_output(["git", "ls-files", "-z"]).split(b"\0")
paths = [Path(p.decode("utf-8", "surrogateescape")) for p in tracked if p]

filename_hits = []
content_hits = []
allowed_hits = []

for path in paths:
    path_text = str(path)
    if needle_re.search(path_text):
        filename_hits.append((path_text, 0, path_text))

    try:
        data = path.read_bytes()
    except OSError as exc:
        content_hits.append((path_text, 0, f"<read error: {exc}>"))
        continue
    if b"\0" in data:
        continue
    text = data.decode("utf-8", "ignore")
    for line_number, line in enumerate(text.splitlines(), 1):
        matched = needle_re.search(line) or any(needle in line for needle in extra_needles)
        if not matched:
            continue
        rendered = line.strip()
        is_allowed_readme_note = (
            path_text == "README.md"
            and ("formerly" in line.lower() or "renamed" in line.lower() or "naming" in line.lower())
            and (old_lower in line.lower() or old_title in line)
        )
        if is_allowed_readme_note:
            allowed_hits.append((path_text, line_number, rendered))
        else:
            content_hits.append((path_text, line_number, rendered))

print("CueLoop final cutover acceptance scan")
print("======================================")
print(f"tracked files scanned: {len(paths)}")
print(f"filename hits: {len(filename_hits)}")
print(f"content hits: {len(content_hits)}")
print(f"allowed README naming notes: {len(allowed_hits)}")

if filename_hits:
    print("\nForbidden filename hits:")
    for path, _, text in filename_hits:
        print(f"  {path}")

if content_hits:
    print("\nForbidden content hits:")
    for path, line_number, text in content_hits[:500]:
        location = f"{path}:{line_number}" if line_number else path
        print(f"  {location}: {text}")
    omitted = len(content_hits) - 500
    if omitted > 0:
        print(f"  ... {omitted} more content hits omitted")

if allowed_hits:
    print("\nAllowed README naming note candidates:")
    for path, line_number, text in allowed_hits:
        print(f"  {path}:{line_number}: {text}")

failed = bool(filename_hits or content_hits or len(allowed_hits) > 1)
if len(allowed_hits) > 1:
    print("\nToo many README naming-switch notes; final state allows at most one.")

if failed:
    print("\nResult: cutover incomplete")
    if mode == "enforce":
        sys.exit(1)
else:
    print("\nResult: cutover complete")
PY
