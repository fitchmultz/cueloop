#!/usr/bin/env bash
#
# Purpose: Classify the current change set into the correct CI surface for agents.
# Responsibilities:
# - Inspect local working-tree changes plus committed branch delta versus trunk.
# - Route docs/community-only surfaces to `ci-docs`.
# - Route non-app executable changes to `ci-fast`.
# - Escalate CLI/build/runtime/app contract changes to `macos-ci`.
# Scope:
# - Classification only; it does not execute make targets itself.
# Usage:
# - scripts/agent-ci-surface.sh --target
# - scripts/agent-ci-surface.sh --reason
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

Outputs:
  --target   Print the target name (`ci-docs`, `ci-fast`, or `macos-ci`)
  --reason   Print a short routing explanation
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
    if [ "$MODE" = "reason" ]; then
        echo "not in a git worktree"
    else
        echo "macos-ci"
    fi
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
    if [ "$MODE" = "reason" ]; then
        echo "no trunk baseline found; conservatively running macos-ci"
    else
        echo "macos-ci"
    fi
    exit 0
fi

if [ -z "$changed_paths" ]; then
    if [ "$MODE" = "reason" ]; then
        echo "no local or branch-delta changes; defaulting to ci-fast"
    else
        echo "ci-fast"
    fi
    exit 0
fi

target="ci-docs"
reason="docs/community metadata only"
while IFS= read -r path; do
    [ -z "$path" ] && continue
    if public_requires_macos_ci_for_path "$path"; then
        target="macos-ci"
        reason="dependency-surface change touched app/CLI/build/runtime contract: $path"
        break
    fi
    if ! public_is_docs_only_path "$path"; then
        target="ci-fast"
        reason="non-app executable change requires Rust/CLI verification: $path"
    fi
done <<< "$changed_paths"

if [ "$MODE" = "reason" ]; then
    echo "$reason"
else
    echo "$target"
fi
