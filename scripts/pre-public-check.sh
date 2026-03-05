#!/usr/bin/env bash
#
# Purpose: Run a repeatable pre-publication audit for the Ralph repository.
# Responsibilities:
# - Validate required public-facing metadata files are present.
# - Detect tracked runtime/build artifacts that should stay local.
# - Run lightweight link checks on key reviewer-facing docs.
# - Optionally run the local CI gate (`make ci`).
# Scope:
# - This script may run `make ci`, which can update formatted/generated files.
# - This script does not create tags/releases.
# Usage:
# - scripts/pre-public-check.sh
# - scripts/pre-public-check.sh --skip-ci
# - scripts/pre-public-check.sh --skip-links --skip-secrets
# Invariants/assumptions:
# - Run from any location; script resolves repo root automatically.
# - Git worktree should be clean before publication; `--skip-clean` is for iterative local audits only.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

SKIP_CI=0
SKIP_LINKS=0
SKIP_SECRETS=0
SKIP_CLEAN=0

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() {
    echo -e "${BLUE}ℹ${NC} $1"
}

log_success() {
    echo -e "${GREEN}✓${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}⚠${NC} $1"
}

log_error() {
    echo -e "${RED}✗${NC} $1"
}

usage() {
    cat << 'EOF'
Pre-publication audit for Ralph.

Usage:
  scripts/pre-public-check.sh [OPTIONS]

Options:
  --skip-ci       Skip `make ci` (use this for non-mutating audits)
  --skip-links    Skip markdown link checks
  --skip-secrets  Skip secret-pattern scan
  --skip-clean    Skip clean-worktree checks (for iterative local audits)
  -h, --help      Show this help message

Exit codes:
  0  Success (all enabled checks passed)
  1  One or more checks failed
  2  Invalid usage

Examples:
  scripts/pre-public-check.sh
  scripts/pre-public-check.sh --skip-ci
  scripts/pre-public-check.sh --skip-links --skip-secrets
  scripts/pre-public-check.sh --skip-clean --skip-ci
EOF
}

resolve_make_cmd() {
    if [ -n "${RALPH_MAKE_CMD:-}" ]; then
        echo "$RALPH_MAKE_CMD"
        return
    fi

    if command -v gmake >/dev/null 2>&1; then
        echo "gmake"
        return
    fi

    if command -v make >/dev/null 2>&1 && make --version 2>/dev/null | grep -q "GNU Make"; then
        echo "make"
        return
    fi

    log_error "GNU Make is required (install with 'brew install make' and use gmake)."
    exit 1
}

check_required_files() {
    log_info "Checking required public-facing files"

    local required=(
        "README.md"
        "LICENSE"
        "CHANGELOG.md"
        "CONTRIBUTING.md"
        "SECURITY.md"
        "CODE_OF_CONDUCT.md"
        "PORTFOLIO.md"
        "docs/guides/public-readiness.md"
        ".github/ISSUE_TEMPLATE/bug_report.md"
        ".github/ISSUE_TEMPLATE/feature_request.md"
        ".github/PULL_REQUEST_TEMPLATE.md"
    )

    local missing=0
    for path in "${required[@]}"; do
        if [ ! -f "$REPO_ROOT/$path" ]; then
            log_error "Missing required file: $path"
            missing=1
        fi
    done

    if [ "$missing" -ne 0 ]; then
        return 1
    fi

    log_success "Required files are present"
}

check_tracked_runtime_artifacts() {
    log_info "Checking for tracked runtime/build artifacts"

    local tracked_xcode
    tracked_xcode=$(git -C "$REPO_ROOT" ls-files apps/RalphMac/build || true)
    if [ -n "$tracked_xcode" ]; then
        log_error "Tracked Xcode build artifacts detected under apps/RalphMac/build"
        return 1
    fi

    local tracked_cache
    tracked_cache=$(git -C "$REPO_ROOT" ls-files -- '.ralph/cache' || true)
    if [ -n "$tracked_cache" ]; then
        log_error "Tracked .ralph/cache artifacts detected"
        return 1
    fi

    local tracked_lock
    tracked_lock=$(git -C "$REPO_ROOT" ls-files -- '.ralph/lock' || true)
    if [ -n "$tracked_lock" ]; then
        log_error "Tracked .ralph/lock artifacts detected"
        return 1
    fi

    local tracked_logs
    tracked_logs=$(git -C "$REPO_ROOT" ls-files -- '.ralph/logs' || true)
    if [ -n "$tracked_logs" ]; then
        log_error "Tracked .ralph/logs artifacts detected"
        return 1
    fi

    local tracked_workspaces
    tracked_workspaces=$(git -C "$REPO_ROOT" ls-files -- '.ralph/workspaces' || true)
    if [ -n "$tracked_workspaces" ]; then
        log_error "Tracked .ralph/workspaces artifacts detected"
        return 1
    fi

    local tracked_undo
    tracked_undo=$(git -C "$REPO_ROOT" ls-files -- '.ralph/undo' || true)
    if [ -n "$tracked_undo" ]; then
        log_error "Tracked .ralph/undo artifacts detected"
        return 1
    fi

    local tracked_webhooks
    tracked_webhooks=$(git -C "$REPO_ROOT" ls-files -- '.ralph/webhooks' || true)
    if [ -n "$tracked_webhooks" ]; then
        log_error "Tracked .ralph/webhooks artifacts detected"
        return 1
    fi

    local tracked_ralph
    tracked_ralph=$(git -C "$REPO_ROOT" ls-files -- '.ralph' || true)
    if [ -n "$tracked_ralph" ]; then
        local -a allowlist=(
            ".ralph/README.md"
            ".ralph/queue.jsonc"
            ".ralph/queue.json"
            ".ralph/done.jsonc"
            ".ralph/done.json"
            ".ralph/config.jsonc"
            ".ralph/config.json"
        )

        local -a unexpected=()
        local path
        while IFS= read -r path; do
            [ -z "$path" ] && continue
            local allowed=0
            local keep
            for keep in "${allowlist[@]}"; do
                if [ "$path" = "$keep" ]; then
                    allowed=1
                    break
                fi
            done
            if [ "$allowed" -eq 0 ]; then
                unexpected+=("$path")
            fi
        done <<< "$tracked_ralph"

        if [ "${#unexpected[@]}" -ne 0 ]; then
            log_error "Tracked .ralph files outside allowlist detected"
            printf '  %s\n' "${unexpected[@]}"
            return 1
        fi
    fi

    log_success "No tracked runtime/build artifacts detected"
}

check_env_tracking() {
    log_info "Checking .env tracking"

    local tracked_env
    tracked_env=$(git -C "$REPO_ROOT" ls-files | grep -E '(^|/)\.env($|\.)' | grep -Ev '(^|/)\.env\.example$' || true)
    if [ -n "$tracked_env" ]; then
        log_error "Tracked env files detected; remove with: git rm --cached <path>"
        printf '  %s\n' "$tracked_env"
        return 1
    fi

    log_success "No tracked env files detected"
}

check_worktree_clean() {
    if [ "$SKIP_CLEAN" -eq 1 ]; then
        log_warn "Skipping clean-worktree check"
        return 0
    fi

    log_info "Checking git worktree cleanliness"

    local dirty
    dirty=$(git -C "$REPO_ROOT" status --porcelain || true)
    if [ -n "$dirty" ]; then
        log_error "Working tree is not clean"
        echo "$dirty" | sed 's/^/  /'
        return 1
    fi

    log_success "Working tree is clean"
}

is_allowlisted_secret_match() {
    local match="$1"

    case "$match" in
        crates/ralph/tests/*)
            return 0
            ;;
        docs/features/security.md:*)
            return 0
            ;;
        crates/ralph/src/fsutil.rs:*AKIA*EXAMPLE*)
            return 0
            ;;
        crates/ralph/src/fsutil.rs:*BEGIN\ OPENSSH\ PRIVATE\ KEY*)
            return 0
            ;;
    esac

    return 1
}

check_secret_patterns() {
    if [ "$SKIP_SECRETS" -eq 1 ]; then
        log_warn "Skipping secret-pattern scan"
        return 0
    fi

    log_info "Scanning for common secret patterns"

    local pattern
    pattern='AKIA[0-9A-Z]{16}|ghp_[A-Za-z0-9]{20,}|xox[baprs]-[A-Za-z0-9-]{10,}|BEGIN (RSA|OPENSSH|EC) PRIVATE KEY'

    local raw_matches
    raw_matches=$(git -C "$REPO_ROOT" grep -nE "$pattern" || true)

    if [ -z "$raw_matches" ]; then
        log_success "No obvious secret patterns found"
        return 0
    fi

    local line
    local disallowed=()
    while IFS= read -r line; do
        [ -z "$line" ] && continue
        if is_allowlisted_secret_match "$line"; then
            continue
        fi
        disallowed+=("$line")
    done <<< "$raw_matches"

    if [ ${#disallowed[@]} -ne 0 ]; then
        log_error "Potential secret material found"
        printf '  %s\n' "${disallowed[@]}"
        return 1
    fi

    log_success "No obvious secret patterns found outside approved test/docs fixtures"
}

check_markdown_links() {
    if [ "$SKIP_LINKS" -eq 1 ]; then
        log_warn "Skipping markdown link checks"
        return 0
    fi

    log_info "Checking key markdown links"

    local files=(
        "$REPO_ROOT/README.md"
        "$REPO_ROOT/PORTFOLIO.md"
        "$REPO_ROOT/CONTRIBUTING.md"
        "$REPO_ROOT/docs/index.md"
        "$REPO_ROOT/docs/releasing.md"
        "$REPO_ROOT/docs/features/app.md"
        "$REPO_ROOT/docs/guides/public-readiness.md"
    )

    python3 - "$REPO_ROOT" "${files[@]}" << 'PY'
import os
import re
import sys
from pathlib import Path

repo_root = Path(sys.argv[1])
files = [Path(p) for p in sys.argv[2:]]
pattern = re.compile(r'!?\[[^\]]*\]\(([^)]+)\)')

missing = []
for source in files:
    text = source.read_text(encoding="utf-8")
    for raw_target in pattern.findall(text):
        target = raw_target.strip()
        if not target:
            continue
        target = target.split()[0].strip('<>')
        if target.startswith(("http://", "https://", "mailto:", "#")):
            continue
        target = target.split("#", 1)[0]
        target = target.split("?", 1)[0]
        if not target:
            continue
        resolved = (source.parent / target).resolve()
        if not resolved.exists():
            missing.append((str(source.relative_to(repo_root)), raw_target))

if missing:
    for src, target in missing:
        print(f"{src}: missing target -> {target}")
    sys.exit(1)
PY

    log_success "Markdown links look valid"
}

run_ci_gate() {
    if [ "$SKIP_CI" -eq 1 ]; then
        log_warn "Skipping CI gate"
        return 0
    fi

    local make_cmd
    make_cmd=$(resolve_make_cmd)

    log_info "Running local CI gate via ${make_cmd} ci"
    "$make_cmd" -C "$REPO_ROOT" ci
    log_success "CI gate passed"
}

main() {
    while [ $# -gt 0 ]; do
        case "$1" in
            --skip-ci)
                SKIP_CI=1
                ;;
            --skip-links)
                SKIP_LINKS=1
                ;;
            --skip-secrets)
                SKIP_SECRETS=1
                ;;
            --skip-clean)
                SKIP_CLEAN=1
                ;;
            -h|--help)
                usage
                exit 0
                ;;
            *)
                log_error "Unknown option: $1"
                usage
                exit 2
                ;;
        esac
        shift
    done

    echo ""
    echo "Pre-public readiness checks"
    echo "=========================="

    check_required_files
    check_tracked_runtime_artifacts
    check_env_tracking
    check_worktree_clean
    check_secret_patterns
    check_markdown_links
    run_ci_gate
    check_worktree_clean

    echo ""
    log_success "Pre-public checks passed"
}

main "$@"
