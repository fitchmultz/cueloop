#!/usr/bin/env bash
#
# Purpose: Run focused public-readiness scans for the CueLoop repository.
# Responsibilities:
# - Reuse the shared repo-wide markdown-link and secret-pattern scan policy.
# - Guard documented session-cache paths against stale `.json` references.
# - Provide lightweight entrypoints for docs-only and targeted safety gates.
# - Resolve the CueLoop repository root and exclusion policy consistently.
# Scope:
# - Focused scan execution only; required-file checks, worktree checks, and CI gating stay in pre-public-check.sh.
# Usage:
# - scripts/lib/public_readiness_scan.sh links
# - scripts/lib/public_readiness_scan.sh session-paths
# - scripts/lib/public_readiness_scan.sh secrets
# - scripts/lib/public_readiness_scan.sh docs
# - scripts/lib/public_readiness_scan.sh all
# - scripts/lib/public_readiness_scan.sh --help
# Invariants/assumptions:
# - Run from any location; the script resolves the repo root automatically.
# - Scan excludes come from scripts/lib/release_policy.sh.

set -euo pipefail

LIB_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SCRIPT_DIR="$(cd "$LIB_DIR/.." && pwd)"
source "$SCRIPT_DIR/lib/cueloop-shell.sh"
REPO_ROOT="$(cueloop_repo_root)"
source "$SCRIPT_DIR/lib/release_policy.sh"

usage() {
    cat <<'EOF'
Run a focused public-readiness scan for CueLoop.

Usage:
  scripts/lib/public_readiness_scan.sh <links|secrets|session-paths|docs|all>
  scripts/lib/public_readiness_scan.sh -h
  scripts/lib/public_readiness_scan.sh --help

Examples:
  scripts/lib/public_readiness_scan.sh links
  scripts/lib/public_readiness_scan.sh session-paths
  scripts/lib/public_readiness_scan.sh secrets
  scripts/lib/public_readiness_scan.sh docs
  scripts/lib/public_readiness_scan.sh all

Exit codes:
  0  Scan passed
  1  Scan failed
  2  Invalid usage
EOF
}

run_scan() {
    local mode="$1"
    local scan_py_path="${CUELOOP_PUBLIC_READINESS_SCAN_PY:-$SCRIPT_DIR/lib/public_readiness_scan.py}"
    local repo_root="$REPO_ROOT"

    export CUELOOP_PUBLIC_SCAN_EXCLUDES
    export CUELOOP_PUBLIC_SCAN_LOCAL_ONLY_BASENAMES
    export CUELOOP_PUBLIC_SCAN_LOCAL_ONLY_BASENAME_PREFIXES
    CUELOOP_PUBLIC_SCAN_EXCLUDES="$(printf '%s\n' "${PUBLIC_SCAN_EXCLUDES[@]}")"
    CUELOOP_PUBLIC_SCAN_LOCAL_ONLY_BASENAMES="$(printf '%s\n' "${PUBLIC_LOCAL_ONLY_BASENAMES[@]}")"
    CUELOOP_PUBLIC_SCAN_LOCAL_ONLY_BASENAME_PREFIXES="$(printf '%s\n' "${PUBLIC_LOCAL_ONLY_BASENAME_PREFIXES[@]}")"

    case "$mode" in
        links)
            cueloop_log_info "Checking repo-wide working-tree markdown links"
            python3 "$scan_py_path" links "$repo_root"
            cueloop_log_success "Markdown links look valid"
            ;;
        session-paths)
            cueloop_log_info "Checking documented session-cache paths"
            python3 "$scan_py_path" session-paths "$repo_root"
            cueloop_log_success "Session-cache path references use session.jsonc"
            ;;
        secrets)
            cueloop_log_info "Scanning repo-wide working-tree text files for high-confidence secret patterns"
            python3 "$scan_py_path" secrets "$repo_root"
            cueloop_log_success "No high-confidence secret patterns found"
            ;;
        docs)
            cueloop_log_info "Checking repo-wide working-tree markdown links and documented session-cache paths"
            python3 "$scan_py_path" docs "$repo_root"
            cueloop_log_success "Markdown links and session-cache path references look valid"
            ;;
        all)
            cueloop_log_info "Running combined repo-wide public-readiness content scan"
            python3 "$scan_py_path" all "$repo_root"
            cueloop_log_success "Public-readiness content scan passed"
            ;;
        *)
            usage >&2
            exit 2
            ;;
    esac
}

case "${1:-}" in
    links|secrets|session-paths|docs|all)
        if [ "$#" -ne 1 ]; then
            usage >&2
            exit 2
        fi
        run_scan "$1"
        ;;
    -h|--help)
        if [ "$#" -ne 1 ]; then
            usage >&2
            exit 2
        fi
        usage
        ;;
    *)
        usage >&2
        exit 2
        ;;
esac
