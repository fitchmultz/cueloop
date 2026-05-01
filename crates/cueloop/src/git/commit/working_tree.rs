//! Working-tree commit helpers.
//!
//! Purpose:
//! - Implement commit creation and tracked-path restore helpers for git workflows.
//!
//! Responsibilities:
//! - Revert uncommitted changes while preserving local `.env*` files.
//! - Create commits after staging repository changes.
//! - Force-add or restore specific repo-relative paths safely.
//!
//! Scope:
//! - Working-tree and index mutations only.
//! - Upstream queries and push flows live in sibling modules.
//!
//! Usage:
//! - Re-exported through `crate::git::commit` and consumed by supervision/runtime helpers.
//!
//! Invariants/assumptions:
//! - Empty commit messages and empty commits are rejected.
//! - Path restores only operate on tracked files under the repo root.

use anyhow::Context;
use std::path::{Path, PathBuf};

use crate::git::error::{GitError, git_output, git_run};
use crate::git::status::status_porcelain;

/// Revert uncommitted changes, restoring the working tree to current HEAD.
///
/// This discards ONLY uncommitted changes. It does NOT reset to a pre-run SHA.
pub fn revert_uncommitted(repo_root: &Path) -> Result<(), GitError> {
    if git_run(repo_root, &["restore", "--staged", "--worktree", "."]).is_err() {
        git_run(repo_root, &["checkout", "--", "."]).context("fallback git checkout -- .")?;
        git_run(repo_root, &["reset", "--quiet", "HEAD"]).context("git reset --quiet HEAD")?;
    }

    git_run(repo_root, &["clean", "-fd", "-e", ".env", "-e", ".env.*"])
        .context("git clean -fd -e .env*")?;
    Ok(())
}

/// Create a commit with all changes.
///
/// Stages everything and creates a single commit with the given message.
/// Returns an error if the message is empty or there are no changes to commit.
pub fn commit_all(repo_root: &Path, message: &str) -> Result<(), GitError> {
    let message = message.trim();
    if message.is_empty() {
        return Err(GitError::EmptyCommitMessage);
    }

    git_run(repo_root, &["add", "-A"]).context("git add -A")?;
    let status = status_porcelain(repo_root)?;
    if status.trim().is_empty() {
        return Err(GitError::NoChangesToCommit);
    }

    git_run(repo_root, &["commit", "-m", message]).context("git commit")?;
    Ok(())
}

/// Force-add existing paths, even if they are ignored.
///
/// Paths must be under the repo root; missing or outside paths are skipped.
pub fn add_paths_force(repo_root: &Path, paths: &[PathBuf]) -> Result<(), GitError> {
    let rel_paths = existing_repo_relative_paths(repo_root, paths);
    if rel_paths.is_empty() {
        return Ok(());
    }

    run_path_command(repo_root, &["add", "-f", "--"], &rel_paths)
        .context("git add -f -- <paths>")?;
    Ok(())
}

/// Restore tracked paths to the current HEAD (index + working tree).
///
/// Paths must be under the repo root; untracked paths are skipped.
pub fn restore_tracked_paths_to_head(repo_root: &Path, paths: &[PathBuf]) -> Result<(), GitError> {
    let rel_paths = tracked_repo_relative_paths(repo_root, paths)?;
    if rel_paths.is_empty() {
        return Ok(());
    }

    if run_path_command(
        repo_root,
        &["restore", "--staged", "--worktree", "--"],
        &rel_paths,
    )
    .is_err()
    {
        run_path_command(repo_root, &["checkout", "--"], &rel_paths)
            .context("fallback git checkout -- <paths>")?;
        run_path_command(repo_root, &["reset", "--quiet", "HEAD", "--"], &rel_paths)
            .context("git reset --quiet HEAD -- <paths>")?;
    }

    Ok(())
}

fn existing_repo_relative_paths(repo_root: &Path, paths: &[PathBuf]) -> Vec<String> {
    repo_relative_paths(repo_root, paths, true)
}

fn tracked_repo_relative_paths(
    repo_root: &Path,
    paths: &[PathBuf],
) -> Result<Vec<String>, GitError> {
    let mut rel_paths = Vec::new();
    for rel_path in repo_relative_paths(repo_root, paths, false) {
        if is_tracked_path(repo_root, &rel_path)? {
            rel_paths.push(rel_path);
        } else {
            log::debug!("Skipping restore for untracked path: {}", rel_path);
        }
    }
    Ok(rel_paths)
}

fn repo_relative_paths(repo_root: &Path, paths: &[PathBuf], require_exists: bool) -> Vec<String> {
    let mut rel_paths = Vec::new();
    for path in paths {
        if require_exists && !path.exists() {
            continue;
        }
        let rel = match path.strip_prefix(repo_root) {
            Ok(rel) => rel,
            Err(_) => {
                log::debug!("Skipping repo path outside repo root: {}", path.display());
                continue;
            }
        };
        if rel.as_os_str().is_empty() {
            continue;
        }
        rel_paths.push(rel.to_string_lossy().to_string());
    }
    rel_paths
}

fn run_path_command(
    repo_root: &Path,
    base_args: &[&str],
    rel_paths: &[String],
) -> Result<(), GitError> {
    let mut args: Vec<&str> = base_args.to_vec();
    args.extend(rel_paths.iter().map(String::as_str));
    git_run(repo_root, &args)?;
    Ok(())
}

fn is_tracked_path(repo_root: &Path, rel_path: &str) -> Result<bool, GitError> {
    let output = git_output(repo_root, &["ls-files", "--error-unmatch", "--", rel_path])
        .with_context(|| {
            format!(
                "run git ls-files --error-unmatch for {} in {}",
                rel_path,
                repo_root.display()
            )
        })?;

    if output.status.success() {
        return Ok(true);
    }

    let stderr = String::from_utf8_lossy(&output.stderr).to_lowercase();
    if stderr.contains("pathspec") || stderr.contains("did not match any file") {
        return Ok(false);
    }

    Err(GitError::CommandFailed {
        args: format!("ls-files --error-unmatch -- {}", rel_path),
        code: output.status.code(),
        stderr: stderr.trim().to_string(),
    })
}
