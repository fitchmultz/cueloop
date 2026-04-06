#!/usr/bin/env bash
#
# Purpose: Provide the shared release transaction facade for Ralph.
# Responsibilities:
# - Validate prerequisites and repository state before release mutation.
# - Verify release-note/changelog contract before snapshot preparation.
# - Source the local-verify and remote-publish pipeline modules.
# Scope:
# - Shared orchestration helpers only; verify/publish phase details live in adjacent modules.
# Usage:
# - source "$(dirname "$0")/lib/release_pipeline.sh"
# Invariants/assumptions:
# - Caller sets VERSION, REPO_ROOT, and release paths before invoking functions.

if [ -n "${RALPH_RELEASE_PIPELINE_SOURCED:-}" ]; then
    return 0
fi
RALPH_RELEASE_PIPELINE_SOURCED=1
set -euo pipefail

source "$SCRIPT_DIR/lib/release_verify_pipeline.sh"
source "$SCRIPT_DIR/lib/release_publish_pipeline.sh"

release_check_prerequisites() {
    local require_publish_credentials="${1:-1}"

    ralph_log_step "Checking release prerequisites"

    local tool
    for tool in git cargo gh python3; do
        if ! command -v "$tool" >/dev/null 2>&1; then
            ralph_log_error "Required tool not found: $tool"
            return 1
        fi
        ralph_log_success "$tool found"
    done

    if ! gh auth status >/dev/null 2>&1; then
        ralph_log_error "GitHub CLI is not authenticated"
        echo "  Run: gh auth login" >&2
        return 1
    fi
    ralph_log_success "GitHub CLI authenticated"

    if [ "$require_publish_credentials" = "1" ]; then
        local cargo_token_file="${CARGO_HOME:-$HOME/.cargo}/credentials.toml"
        if [ -z "${CARGO_REGISTRY_TOKEN:-}" ] && [ ! -f "$cargo_token_file" ]; then
            ralph_log_error "crates.io publish credentials not found"
            echo "  Run: cargo login" >&2
            echo "  Or set CARGO_REGISTRY_TOKEN for this release" >&2
            return 1
        fi
        ralph_log_success "crates.io publish credentials found"
    fi
}

release_validate_repo_state() {
    local allow_existing_tag="${1:-0}"
    local allow_release_metadata_drift="${2:-0}"

    ralph_log_step "Validating repository state"
    cd "$REPO_ROOT"

    local current_branch
    current_branch=$(git branch --show-current)
    if [ "$current_branch" != "main" ]; then
        ralph_log_error "Not on main branch (currently on: $current_branch)"
        return 1
    fi
    ralph_log_success "On main branch"

    local collected_dirty_files
    collected_dirty_files=$(release_collect_dirty_lines "$REPO_ROOT") || return 1
    local dirty_files
    dirty_files=$(release_filter_dirty_lines "$collected_dirty_files")
    if [ -n "$dirty_files" ]; then
        if [ "$allow_release_metadata_drift" = "1" ] && release_assert_dirty_paths_allowed "$dirty_files"; then
            ralph_log_success "Working directory contains only release metadata drift"
        else
            ralph_log_error "Working directory is not clean"
            echo "$dirty_files" | sed 's/^/  /' >&2
            return 1
        fi
    else
        ralph_log_success "Working directory is clean"
    fi

    if ! git ls-remote origin >/dev/null 2>&1; then
        ralph_log_error "Cannot access git remote"
        return 1
    fi
    ralph_log_success "Git remote is accessible"

    if git rev-parse "v$VERSION" >/dev/null 2>&1; then
        if [ "$allow_existing_tag" = "1" ]; then
            ralph_log_warn "Local tag v$VERSION already exists; verify mode allows that"
        else
            ralph_log_error "Local tag v$VERSION already exists"
            echo "  Continue the recorded transaction with: scripts/release.sh reconcile $VERSION" >&2
            return 1
        fi
    else
        ralph_log_success "Local tag v$VERSION does not exist"
    fi

    if git ls-remote --tags origin "refs/tags/v$VERSION" | grep -q "refs/tags/v$VERSION"; then
        if [ "$allow_existing_tag" = "1" ]; then
            ralph_log_warn "Remote tag v$VERSION already exists; verify mode allows that"
        else
            ralph_log_error "Remote tag v$VERSION already exists"
            return 1
        fi
    else
        ralph_log_success "Remote tag v$VERSION does not exist"
    fi
}

release_verify_plan() {
    ralph_log_step "Verifying release transaction contract"
    if ! release_validate_changelog_shape "$CHANGELOG"; then
        ralph_log_error "CHANGELOG.md is missing a release-compatible Unreleased section"
        return 1
    fi

    local preview_file
    local preview_changelog
    local preview_checksums
    preview_file=$(ralph_mktemp_file "ralph-release-preview")
    preview_changelog=$(ralph_mktemp_file "ralph-release-preview-changelog")
    preview_checksums=$(ralph_mktemp_file "ralph-release-preview-checksums")
    printf 'Preview changelog entry\n' > "$preview_changelog"
    printf 'ralph-%s-sample.tar.gz  abcdef\n' "$VERSION" > "$preview_checksums"

    local preview_repo_url
    preview_repo_url=$(ralph_get_repo_http_url)
    release_render_notes_template \
        "$RELEASE_NOTES_TEMPLATE" \
        "$preview_file" \
        "$VERSION" \
        "$preview_changelog" \
        "$preview_checksums" \
        "$preview_repo_url"

    if ! grep -q "$VERSION" "$preview_file"; then
        ralph_log_error "Rendered release notes preview is missing the version marker"
        rm -f "$preview_file" "$preview_changelog" "$preview_checksums"
        return 1
    fi

    rm -f "$preview_file" "$preview_changelog" "$preview_checksums"
    ralph_log_success "Release transaction contract is valid"
}
