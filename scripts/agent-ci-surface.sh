#!/usr/bin/env bash
#
# Purpose: Classify the current change set into the correct CI surface for agents.
# Responsibilities:
# - Inspect local working-tree changes plus committed branch delta versus trunk.
# - Route docs/community-only surfaces to `ci-docs`.
# - Route ancillary non-code surfaces to `ci-fast`.
# - Route Rust crate work to `ci` (release-shaped Rust gate).
# - Escalate app/toolchain/bundling/schema/script surfaces to `macos-ci`.
# Scope:
# - Classification only; it does not execute make targets itself.
# Usage:
# - scripts/agent-ci-surface.sh --target
# - scripts/agent-ci-surface.sh --reason
# - scripts/agent-ci-surface.sh --emit-eval   # shell-evaluable RALPH_AGENT_CI_* assignments
# Invariants/assumptions:
# - When no git worktree or trunk baseline is available, callers should conservatively run `macos-ci`.
# - `ci-docs` is reserved for changes that cannot alter executable behavior.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/lib/ralph-shell.sh"
REPO_ROOT="$(ralph_repo_root)"
source "$SCRIPT_DIR/lib/release_policy.sh"

MODE="target"

usage() {
    cat <<'EOF'
Usage:
  scripts/agent-ci-surface.sh --target
  scripts/agent-ci-surface.sh --reason
  scripts/agent-ci-surface.sh --emit-eval

Outputs:
  --target     Print the target name (`ci-docs`, `ci-fast`, `ci`, or `macos-ci`)
  --reason     Print a short routing explanation
  --emit-eval  Print `RALPH_AGENT_CI_TARGET=...` and `RALPH_AGENT_CI_REASON=...` for `eval` in bash
EOF
}

while [ $# -gt 0 ]; do
    case "$1" in
        --target)
            MODE="target"
            ;;
        --reason)
            MODE="reason"
            ;;
        --emit-eval)
            MODE="emit-eval"
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            ralph_log_error "Unknown option: $1"
            usage
            exit 2
            ;;
    esac
    shift
done

if ! git -C "$REPO_ROOT" rev-parse --is-inside-work-tree >/dev/null 2>&1; then
    target="macos-ci"
    reason="not in a git worktree"
    case "$MODE" in
        target) printf '%s\n' "$target" ;;
        reason) printf '%s\n' "$reason" ;;
        emit-eval)
            printf 'RALPH_AGENT_CI_TARGET=%q\n' "$target"
            printf 'RALPH_AGENT_CI_REASON=%q\n' "$reason"
            ;;
    esac
    exit 0
fi

resolve_trunk_ref() {
    local candidate
    for candidate in refs/remotes/origin/main refs/heads/main refs/remotes/origin/master refs/heads/master; do
        if git -C "$REPO_ROOT" show-ref --verify --quiet "$candidate"; then
            case "$candidate" in
                refs/remotes/*)
                    printf '%s\n' "${candidate#refs/remotes/}"
                    ;;
                refs/heads/*)
                    printf '%s\n' "${candidate#refs/heads/}"
                    ;;
            esac
            return 0
        fi
    done
    return 1
}

branch_delta_paths() {
    local trunk_ref="$1"
    local merge_base
    merge_base=$(git -C "$REPO_ROOT" merge-base HEAD "$trunk_ref" 2>/dev/null || true)
    if [ -z "$merge_base" ]; then
        return 0
    fi

    git -C "$REPO_ROOT" diff --name-only --relative "$merge_base"...HEAD
}

trunk_ref="$(resolve_trunk_ref || true)"

changed_paths="$(
    {
        git -C "$REPO_ROOT" diff --name-only --relative
        git -C "$REPO_ROOT" diff --cached --name-only --relative
        git -C "$REPO_ROOT" ls-files --others --exclude-standard
        if [ -n "$trunk_ref" ]; then
            branch_delta_paths "$trunk_ref"
        fi
    } | sed '/^$/d' | sort -u
)"

if [ -z "$trunk_ref" ]; then
    target="macos-ci"
    reason="no trunk baseline found; conservatively running macos-ci"
    case "$MODE" in
        target) printf '%s\n' "$target" ;;
        reason) printf '%s\n' "$reason" ;;
        emit-eval)
            printf 'RALPH_AGENT_CI_TARGET=%q\n' "$target"
            printf 'RALPH_AGENT_CI_REASON=%q\n' "$reason"
            ;;
    esac
    exit 0
fi

if [ -z "$changed_paths" ]; then
    target="ci-fast"
    reason="no local or branch-delta changes; defaulting to ci-fast"
    case "$MODE" in
        target) printf '%s\n' "$target" ;;
        reason) printf '%s\n' "$reason" ;;
        emit-eval)
            printf 'RALPH_AGENT_CI_TARGET=%q\n' "$target"
            printf 'RALPH_AGENT_CI_REASON=%q\n' "$reason"
            ;;
    esac
    exit 0
fi

all_docs_only=1
while IFS= read -r path; do
    [ -z "$path" ] && continue
    if ! public_is_docs_only_path "$path"; then
        all_docs_only=0
        break
    fi
done <<< "$changed_paths"

if [ "$all_docs_only" = "1" ]; then
    target="ci-docs"
    reason="docs/community metadata only"
    case "$MODE" in
        target) printf '%s\n' "$target" ;;
        reason) printf '%s\n' "$reason" ;;
        emit-eval)
            printf 'RALPH_AGENT_CI_TARGET=%q\n' "$target"
            printf 'RALPH_AGENT_CI_REASON=%q\n' "$reason"
            ;;
    esac
    exit 0
fi

while IFS= read -r path; do
    [ -z "$path" ] && continue
    if public_requires_macos_ship_gate_for_path "$path"; then
        target="macos-ci"
        reason="dependency-surface change requires macOS ship gate (app/toolchain/bundle/scripts/schemas): $path"
        case "$MODE" in
            target) printf '%s\n' "$target" ;;
            reason) printf '%s\n' "$reason" ;;
            emit-eval)
                printf 'RALPH_AGENT_CI_TARGET=%q\n' "$target"
                printf 'RALPH_AGENT_CI_REASON=%q\n' "$reason"
                ;;
        esac
        exit 0
    fi
done <<< "$changed_paths"

while IFS= read -r path; do
    [ -z "$path" ] && continue
    if public_requires_rust_release_gate_for_path "$path"; then
        target="ci"
        reason="Rust crate change requires release-shaped verification: $path"
        case "$MODE" in
            target) printf '%s\n' "$target" ;;
            reason) printf '%s\n' "$reason" ;;
            emit-eval)
                printf 'RALPH_AGENT_CI_TARGET=%q\n' "$target"
                printf 'RALPH_AGENT_CI_REASON=%q\n' "$reason"
                ;;
        esac
        exit 0
    fi
done <<< "$changed_paths"

target="ci-fast"
reason="non-docs change requires fast Rust/CLI verification"
case "$MODE" in
    target) printf '%s\n' "$target" ;;
    reason) printf '%s\n' "$reason" ;;
    emit-eval)
        printf 'RALPH_AGENT_CI_TARGET=%q\n' "$target"
        printf 'RALPH_AGENT_CI_REASON=%q\n' "$reason"
        ;;
esac
