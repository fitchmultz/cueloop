#!/usr/bin/env bash
#
# Purpose: Execute the remote-facing Ralph release transaction phases.
# Responsibilities:
# - Create the release commit/tag only after a verified snapshot is accepted.
# - Publish reversible remote state before the crates.io cutover.
# - Finalize the GitHub release only after crate publication succeeds.
# Scope:
# - Execute/reconcile remote publication phases only; local verification lives elsewhere.
# Usage:
# - source "$(dirname "$0")/lib/release_publish_pipeline.sh"
# Invariants/assumptions:
# - Caller already initialized transaction state before invoking these helpers.
# - crates.io publish is the only intentionally irreversible phase in the transaction.

if [ -n "${RALPH_RELEASE_PUBLISH_PIPELINE_SOURCED:-}" ]; then
    return 0
fi
RALPH_RELEASE_PUBLISH_PIPELINE_SOURCED=1

release_query_github_release_state() {
    local draft_state
    draft_state=$(gh release view "v$VERSION" --json isDraft --jq '.isDraft' 2>/dev/null | tr -d '[:space:]' || true)

    case "$draft_state" in
        true)
            echo "draft"
            ;;
        false)
            echo "published"
            ;;
        *)
            echo "missing"
            ;;
    esac
}

release_sync_github_release_state() {
    local remote_state
    remote_state=$(release_query_github_release_state)

    case "$remote_state" in
        draft)
            GITHUB_RELEASE_DRAFT_CREATED=1
            ;;
        published)
            GITHUB_RELEASE_DRAFT_CREATED=1
            GITHUB_RELEASE_PUBLISHED=1
            ;;
    esac
}

release_crate_is_published() {
    cargo info --quiet "${CRATE_PACKAGE_NAME}@${VERSION}" >/dev/null 2>&1
}

release_upload_github_release_artifacts() {
    local artifact
    for artifact in "$RELEASE_ARTIFACTS_DIR"/ralph-"${VERSION}"-*.tar.gz; do
        [ -f "$artifact" ] || continue
        gh release upload "v$VERSION" "$artifact" "${artifact}.sha256" --clobber
    done
}

release_create_commit_and_tag() {
    if [ "$LOCAL_TAG_CREATED" = "1" ]; then
        ralph_log_info "Local release commit/tag already recorded for v$VERSION"
        return 0
    fi

    if git -C "$REPO_ROOT" rev-parse "v$VERSION" >/dev/null 2>&1; then
        RELEASE_COMMIT=$(git -C "$REPO_ROOT" rev-parse "v$VERSION^{commit}")
        LOCAL_TAG_CREATED=1
        RELEASE_STATUS="prepared"
        release_state_write
        ralph_log_info "Local tag v$VERSION already exists"
        return 0
    fi

    ralph_log_step "Creating release commit and tag"
    cd "$REPO_ROOT"

    git add "${RELEASE_METADATA_PATHS[@]}"
    git commit -m "Release v$VERSION"
    RELEASE_COMMIT=$(git rev-parse HEAD)
    git tag -a "v$VERSION" -m "Release v$VERSION"
    LOCAL_TAG_CREATED=1
    RELEASE_STATUS="prepared"
    release_state_write
    ralph_log_success "Created release commit $RELEASE_COMMIT and tag v$VERSION"
}

release_remote_main_matches_release_commit() {
    [ -n "${RELEASE_COMMIT:-}" ] || return 1
    git -C "$REPO_ROOT" ls-remote origin "refs/heads/main" | grep -q "^$RELEASE_COMMIT[[:space:]]"
}

release_remote_tag_exists() {
    git -C "$REPO_ROOT" ls-remote --tags origin "refs/tags/v$VERSION" | grep -q "refs/tags/v$VERSION"
}

release_push_remote_main() {
    if [ "$REMOTE_MAIN_PUSHED" = "1" ]; then
        ralph_log_info "Remote main push already recorded for v$VERSION"
        return 0
    fi

    if release_remote_main_matches_release_commit; then
        REMOTE_MAIN_PUSHED=1
        RELEASE_STATUS="main_pushed"
        release_state_write
        ralph_log_info "Remote main already matches release commit for v$VERSION"
        return 0
    fi

    ralph_log_step "Pushing release commit to origin/main"
    cd "$REPO_ROOT"
    git push origin main
    REMOTE_MAIN_PUSHED=1
    RELEASE_STATUS="main_pushed"
    release_state_write
    ralph_log_success "Pushed release commit to origin/main"
}

release_push_remote_tag() {
    if [ "$REMOTE_TAG_PUSHED" = "1" ]; then
        ralph_log_info "Remote tag push already recorded for v$VERSION"
        return 0
    fi

    if release_remote_tag_exists; then
        REMOTE_TAG_PUSHED=1
        RELEASE_STATUS="tag_pushed"
        release_state_write
        ralph_log_info "Remote tag v$VERSION already exists"
        return 0
    fi

    ralph_log_step "Pushing release tag"
    cd "$REPO_ROOT"
    git push origin "v$VERSION"
    REMOTE_TAG_PUSHED=1
    RELEASE_STATUS="tag_pushed"
    release_state_write
    ralph_log_success "Pushed tag v$VERSION"
}

release_create_or_update_github_release_draft() {
    release_sync_github_release_state
    if [ "$GITHUB_RELEASE_PUBLISHED" = "1" ]; then
        ralph_log_warn "GitHub release v$VERSION is already public"
        RELEASE_STATUS="completed"
        release_state_write
        return 0
    fi
    if [ "$GITHUB_RELEASE_DRAFT_CREATED" = "1" ]; then
        RELEASE_STATUS="github_release_drafted"
        release_state_write
        ralph_log_info "GitHub draft release already recorded for v$VERSION"
        return 0
    fi

    ralph_log_step "Preparing GitHub draft release"
    local remote_state
    remote_state=$(release_query_github_release_state)
    if [ "$remote_state" = "missing" ]; then
        gh release create "v$VERSION" \
            --draft \
            --title "v$VERSION" \
            --verify-tag \
            --notes-file "$RELEASE_NOTES_FILE"
    fi

    release_upload_github_release_artifacts
    GITHUB_RELEASE_DRAFT_CREATED=1
    RELEASE_STATUS="github_release_drafted"
    release_state_write
    ralph_log_success "GitHub draft release prepared for v$VERSION"
}

release_publish_crate() {
    if [ "$CRATE_PUBLISHED" = "1" ]; then
        ralph_log_info "crates.io publication already recorded for v$VERSION"
        return 0
    fi

    if release_crate_is_published; then
        CRATE_PUBLISHED=1
        RELEASE_STATUS="crate_published"
        release_state_write
        ralph_log_info "$CRATE_PACKAGE_NAME v$VERSION already exists on crates.io"
        return 0
    fi

    ralph_log_step "Publishing crate to crates.io"
    cd "$REPO_ROOT"
    cargo package --list -p "$CRATE_PACKAGE_NAME"
    cargo publish --dry-run -p "$CRATE_PACKAGE_NAME" --locked
    cargo publish -p "$CRATE_PACKAGE_NAME" --locked
    CRATE_PUBLISHED=1
    RELEASE_STATUS="crate_published"
    release_state_write
    ralph_log_success "Published $CRATE_PACKAGE_NAME v$VERSION"
}

release_publish_github_release() {
    if [ "$GITHUB_RELEASE_PUBLISHED" = "1" ]; then
        ralph_log_info "GitHub release already finalized for v$VERSION"
        return 0
    fi

    release_sync_github_release_state
    if [ "$GITHUB_RELEASE_PUBLISHED" = "1" ]; then
        RELEASE_STATUS="completed"
        release_state_write
        ralph_log_success "GitHub release v$VERSION is already public"
        return 0
    fi

    if [ "$GITHUB_RELEASE_DRAFT_CREATED" != "1" ]; then
        ralph_log_error "GitHub draft release is missing for v$VERSION"
        echo "  Re-run: scripts/release.sh reconcile $VERSION" >&2
        return 1
    fi

    ralph_log_step "Publishing GitHub release"
    gh release edit "v$VERSION" --draft=false --title "v$VERSION" --notes-file "$RELEASE_NOTES_FILE"
    GITHUB_RELEASE_PUBLISHED=1
    RELEASE_STATUS="completed"
    release_state_write
    ralph_log_success "GitHub release v$VERSION published"
}
