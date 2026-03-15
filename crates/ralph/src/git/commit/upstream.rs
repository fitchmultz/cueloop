//! Upstream and push helpers.
//!
//! Purpose:
//! - Implement upstream discovery, rev-list comparisons, fetch/rebase, and direct push helpers.
//!
//! Responsibilities:
//! - Query upstream configuration and ahead/behind status.
//! - Execute push, fetch, rebase, abort, and conflict-listing commands.
//! - Provide shared reference helpers for rebase-aware push logic.
//!
//! Scope:
//! - Upstream/ref inspection and simple push operations only.
//! - Retry loops for non-fast-forward recovery live in `rebase_push.rs`.
//!
//! Usage:
//! - Re-exported through `crate::git::commit` and consumed by command workflows.
//!
//! Invariants/assumptions:
//! - `rev-list --left-right --count` output must always parse into two integers.
//! - Push failures are classified through `GitError` helpers.

use anyhow::Context;
use std::path::Path;

use crate::git::error::{GitError, classify_push_error, git_output, git_run};

/// Get the configured upstream for the current branch.
///
/// Returns the upstream reference (e.g. "origin/main") or an error if not configured.
pub fn upstream_ref(repo_root: &Path) -> Result<String, GitError> {
    let output = git_output(
        repo_root,
        &["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"],
    )
    .with_context(|| {
        format!(
            "run git rev-parse --abbrev-ref --symbolic-full-name @{{u}} in {}",
            repo_root.display()
        )
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(classify_push_error(&stderr));
    }

    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if value.is_empty() {
        return Err(GitError::NoUpstreamConfigured);
    }
    Ok(value)
}

/// Check if HEAD is ahead of the configured upstream.
///
/// Returns true if there are local commits that haven't been pushed.
pub fn is_ahead_of_upstream(repo_root: &Path) -> Result<bool, GitError> {
    let upstream = upstream_ref(repo_root)?;
    let (_behind, ahead) = rev_list_left_right_counts(repo_root, &format!("{upstream}...HEAD"))?;
    Ok(ahead > 0)
}

/// Push HEAD to the configured upstream.
///
/// Returns an error if push fails due to authentication, missing upstream,
/// or other git errors.
pub fn push_upstream(repo_root: &Path) -> Result<(), GitError> {
    let output = git_output(repo_root, &["push"])
        .with_context(|| format!("run git push in {}", repo_root.display()))?;

    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    Err(classify_push_error(&stderr))
}

/// Push HEAD to origin and create upstream tracking.
///
/// Intended for new branches that do not have an upstream configured yet.
pub fn push_upstream_allow_create(repo_root: &Path) -> Result<(), GitError> {
    let output = git_output(repo_root, &["push", "-u", "origin", "HEAD"])
        .with_context(|| format!("run git push -u origin HEAD in {}", repo_root.display()))?;

    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    Err(classify_push_error(&stderr))
}

/// Fetch a specific branch from origin.
pub fn fetch_branch(repo_root: &Path, remote: &str, branch: &str) -> Result<(), GitError> {
    git_run(repo_root, &["fetch", remote, branch])
        .with_context(|| format!("fetch {} {} in {}", remote, branch, repo_root.display()))?;
    Ok(())
}

/// Check if the current branch is behind its upstream.
///
/// Returns true if the upstream has commits that are not in the current branch.
pub fn is_behind_upstream(repo_root: &Path, branch: &str) -> Result<bool, GitError> {
    fetch_branch(repo_root, "origin", branch)?;

    let upstream = format!("origin/{}", branch);
    let (_ahead, behind) = rev_list_left_right_counts(repo_root, &format!("HEAD...{upstream}"))?;
    Ok(behind > 0)
}

/// Rebase current branch onto a target reference.
pub fn rebase_onto(repo_root: &Path, target: &str) -> Result<(), GitError> {
    git_run(repo_root, &["fetch", "origin", "--prune"])
        .with_context(|| format!("fetch before rebase in {}", repo_root.display()))?;
    git_run(repo_root, &["rebase", target])
        .with_context(|| format!("rebase onto {} in {}", target, repo_root.display()))?;
    Ok(())
}

/// Abort an in-progress rebase.
pub fn abort_rebase(repo_root: &Path) -> Result<(), GitError> {
    git_run(repo_root, &["rebase", "--abort"])
        .with_context(|| format!("abort rebase in {}", repo_root.display()))?;
    Ok(())
}

/// List files with merge conflicts.
///
/// Returns a list of file paths that have unresolved merge conflicts.
pub fn list_conflict_files(repo_root: &Path) -> Result<Vec<String>, GitError> {
    let output =
        git_output(repo_root, &["diff", "--name-only", "--diff-filter=U"]).with_context(|| {
            format!(
                "run git diff --name-only --diff-filter=U in {}",
                repo_root.display()
            )
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(GitError::CommandFailed {
            args: "diff --name-only --diff-filter=U".to_string(),
            code: output.status.code(),
            stderr: stderr.trim().to_string(),
        });
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout
        .lines()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect())
}

/// Push the current branch to a remote.
///
/// This pushes HEAD to the current branch on the specified remote.
pub fn push_current_branch(repo_root: &Path, remote: &str) -> Result<(), GitError> {
    let output = git_output(repo_root, &["push", remote, "HEAD"])
        .with_context(|| format!("run git push {} HEAD in {}", remote, repo_root.display()))?;

    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    Err(classify_push_error(&stderr))
}

/// Push HEAD to a specific branch on a remote.
///
/// This pushes HEAD to the specified branch on the remote, creating the branch if needed.
/// Used in direct-push parallel mode to push directly to the base branch.
pub fn push_head_to_branch(repo_root: &Path, remote: &str, branch: &str) -> Result<(), GitError> {
    let refspec = format!("HEAD:{}", branch);
    let output = git_output(repo_root, &["push", remote, &refspec]).with_context(|| {
        format!(
            "run git push {} HEAD:{} in {}",
            remote,
            branch,
            repo_root.display()
        )
    })?;

    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    Err(classify_push_error(&stderr))
}

pub(super) fn reference_exists(repo_root: &Path, reference: &str) -> Result<bool, GitError> {
    let output = git_output(repo_root, &["rev-parse", "--verify", "--quiet", reference])
        .with_context(|| {
            format!(
                "run git rev-parse --verify --quiet {} in {}",
                reference,
                repo_root.display()
            )
        })?;
    if output.status.success() {
        return Ok(true);
    }
    if output.status.code() == Some(1) {
        return Ok(false);
    }
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    Err(GitError::CommandFailed {
        args: format!("rev-parse --verify --quiet {}", reference),
        code: output.status.code(),
        stderr: stderr.trim().to_string(),
    })
}

pub(super) fn is_ahead_of_ref(repo_root: &Path, reference: &str) -> Result<bool, GitError> {
    let (_behind, ahead) = rev_list_left_right_counts(repo_root, &format!("{reference}...HEAD"))?;
    Ok(ahead > 0)
}

pub(super) fn set_upstream_to(repo_root: &Path, upstream: &str) -> Result<(), GitError> {
    git_run(repo_root, &["branch", "--set-upstream-to", upstream])
        .with_context(|| format!("set upstream to {} in {}", upstream, repo_root.display()))?;
    Ok(())
}

pub(super) fn rev_list_left_right_counts(
    repo_root: &Path,
    range: &str,
) -> Result<(u32, u32), GitError> {
    let output = git_output(repo_root, &["rev-list", "--left-right", "--count", range])
        .with_context(|| {
            format!(
                "run git rev-list --left-right --count {} in {}",
                range,
                repo_root.display()
            )
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(GitError::CommandFailed {
            args: format!("rev-list --left-right --count {}", range),
            code: output.status.code(),
            stderr: stderr.trim().to_string(),
        });
    }

    let counts = String::from_utf8_lossy(&output.stdout);
    let parts: Vec<&str> = counts.split_whitespace().collect();
    if parts.len() != 2 {
        return Err(GitError::UnexpectedRevListOutput(counts.trim().to_string()));
    }

    let left: u32 = parts[0].parse().context("parse left count")?;
    let right: u32 = parts[1].parse().context("parse right count")?;
    Ok((left, right))
}
