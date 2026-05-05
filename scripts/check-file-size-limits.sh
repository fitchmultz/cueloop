#!/usr/bin/env bash
#
# Purpose: Enforce CueLoop's repository file-size policy for human-authored files.
# Responsibilities:
# - Resolve the repository root from this script path.
# - Delegate deterministic policy checks to the Python helper.
# - Expose a stable operator/CI entrypoint with documented exit codes.
# Scope:
# - Wrapper/entrypoint behavior only; scan policy and reporting live in scripts/lib/file_size_limits.py.
# Usage:
# - scripts/check-file-size-limits.sh
# - scripts/check-file-size-limits.sh --help
# - scripts/check-file-size-limits.sh --exclude-glob 'docs/generated/**'
# Invariants/assumptions:
# - The helper script exists at scripts/lib/file_size_limits.py.
# - Maintainer policy is soft advisory at 1500 LOC, review advisory at 3000 LOC,
#   and blocking fail at 5000 LOC unless allowlisted.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

usage() {
    cat <<'USAGE'
Enforce CueLoop file-size limits for human-authored repository files.

Usage:
  scripts/check-file-size-limits.sh
  scripts/check-file-size-limits.sh --help
  scripts/check-file-size-limits.sh [helper-options]

Examples:
  scripts/check-file-size-limits.sh
  scripts/check-file-size-limits.sh --exclude-glob 'docs/generated/**'
  scripts/check-file-size-limits.sh --soft-limit 1500 --review-limit 3000 --fail-limit 5000

Notes:
  Additional arguments are forwarded to scripts/lib/file_size_limits.py.

Exit codes:
  0  No fail-threshold violations (advisories may still be reported)
  1  One or more fail-threshold violations found
  2  Usage, argument, or allowlist error
USAGE
}

if [ "${1:-}" = "-h" ] || [ "${1:-}" = "--help" ]; then
    usage
    exit 0
fi

exec python3 "$SCRIPT_DIR/lib/file_size_limits.py" "$REPO_ROOT" "$@"
