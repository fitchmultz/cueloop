//! Rebase-aware push helpers.
//!
//! Purpose:
//! - Implement bounded retry logic for pushing with non-fast-forward recovery.
//!
//! Responsibilities:
//! - Detect non-fast-forward push failures.
//! - Rebase onto the appropriate upstream reference and retry.
//! - Set local upstream tracking when remote-only branches already exist.
//!
//! Scope:
//! - Retry-oriented push orchestration only.
//! - Lower-level git commands live in sibling modules.
//!
//! Usage:
//! - Re-exported as `crate::git::push_upstream_with_rebase`.
//!
//! Invariants/assumptions:
//! - Retries stay bounded.
//! - Upstream fallback targets use `origin/<current-branch>` when explicit tracking is absent.

use std::path::Path;

use crate::git::current_branch;
use crate::git::error::GitError;

use super::upstream::{
    is_ahead_of_ref, is_ahead_of_upstream, push_upstream, push_upstream_allow_create, rebase_onto,
    reference_exists, set_upstream_to, upstream_ref,
};

/// Push HEAD to upstream, rebasing on non-fast-forward rejections.
///
/// If the branch has no upstream yet, this will create one via `git push -u origin HEAD`.
/// When the push is rejected because the remote has new commits, this will:
/// - `git fetch origin --prune`
/// - `git rebase <upstream>`
/// - retry push with a bounded number of attempts
pub fn push_upstream_with_rebase(repo_root: &Path) -> Result<(), GitError> {
    const MAX_PUSH_ATTEMPTS: usize = 4;

    let branch = current_branch(repo_root).map_err(GitError::Other)?;
    let fallback_upstream = format!("origin/{}", branch);
    let ahead = match is_ahead_of_upstream(repo_root) {
        Ok(ahead) => ahead,
        Err(GitError::NoUpstream) | Err(GitError::NoUpstreamConfigured) => {
            if reference_exists(repo_root, &fallback_upstream)? {
                is_ahead_of_ref(repo_root, &fallback_upstream)?
            } else {
                true
            }
        }
        Err(err) => return Err(err),
    };

    if !ahead {
        if upstream_ref(repo_root).is_err() && reference_exists(repo_root, &fallback_upstream)? {
            set_upstream_to(repo_root, &fallback_upstream)?;
        }
        return Ok(());
    }

    let mut last_non_fast_forward: Option<GitError> = None;
    for _attempt in 0..MAX_PUSH_ATTEMPTS {
        let push_result = match push_upstream(repo_root) {
            Ok(()) => return Ok(()),
            Err(GitError::NoUpstream) | Err(GitError::NoUpstreamConfigured) => {
                push_upstream_allow_create(repo_root)
            }
            Err(err) => Err(err),
        };

        match push_result {
            Ok(()) => return Ok(()),
            Err(err) if is_non_fast_forward_error(&err) => {
                let upstream = match upstream_ref(repo_root) {
                    Ok(upstream) => upstream,
                    Err(_) => fallback_upstream.clone(),
                };
                rebase_onto(repo_root, &upstream)?;
                if !is_ahead_of_ref(repo_root, &upstream)? {
                    if upstream_ref(repo_root).is_err() {
                        set_upstream_to(repo_root, &upstream)?;
                    }
                    return Ok(());
                }
                last_non_fast_forward = Some(err);
            }
            Err(err) => return Err(err),
        }
    }

    Err(last_non_fast_forward
        .unwrap_or_else(|| GitError::PushFailed("rebase-aware push exhausted retries".to_string())))
}

fn is_non_fast_forward_error(err: &GitError) -> bool {
    let GitError::PushFailed(detail) = err else {
        return false;
    };
    let lower = detail.to_lowercase();
    lower.contains("non-fast-forward")
        || lower.contains("non fast-forward")
        || lower.contains("fetch first")
        || lower.contains("rejected")
        || lower.contains("updates were rejected")
}
