//! Repository cleanliness validation.
//!
//! Purpose:
//! - Repository cleanliness validation.
//!
//! Responsibilities:
//! - Provide focused implementation or regression coverage for this file's owning feature.
//!
//! Scope:
//! - Limited to this file's owning feature boundary.
//!
//! This module provides functions for validating that a repository is in a clean
//! state, with support for allowing specific paths to be dirty (for example,
//! CueLoop/CueLoop runtime files).
//!
//! # Invariants
//! - Allowed paths must be normalized before comparison
//! - Directory prefixes work with or without trailing slashes
//! - Force flag bypasses all checks
//!
//! # What this does NOT handle
//! - Actual git operations (see git/commit.rs)
//! - Status parsing details (see git/status.rs)
//! - LFS validation (see git/lfs.rs)
//!
//! Usage:
//! - Used through the crate module tree or integration test harness.

use crate::git::error::GitError;
use crate::git::status::{
    PathSnapshot, changed_paths_from_snapshots, parse_porcelain_z_entries, snapshot_paths,
    status_paths, status_porcelain,
};
use std::collections::BTreeMap;
use std::path::Path;

/// Paths that are allowed to be dirty during CLI runs.
///
/// These are CueLoop runtime files that may change during normal operation.
pub const CUELOOP_RUN_CLEAN_ALLOWED_PATHS: &[&str] = &[
    ".cueloop/queue.jsonc",
    ".cueloop/done.jsonc",
    ".cueloop/config.jsonc",
    ".cueloop/cache/",
];

/// Narrower allowlist for queue-only runner workflows.
///
/// These commands may shape queue state and write CueLoop cache/checkpoint files,
/// but they must not silently edit source, docs, or repo config.
pub const CUELOOP_QUEUE_ONLY_ALLOWED_PATHS: &[&str] = &[
    ".cueloop/queue.jsonc",
    ".cueloop/done.jsonc",
    ".cueloop/cache/",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirtyPathBaseline {
    snapshots: Vec<PathSnapshot>,
}

impl DirtyPathBaseline {
    pub(crate) fn has_revert_sensitive_disallowed_paths(&self) -> bool {
        self.snapshots
            .iter()
            .any(|snapshot| !snapshot.path.starts_with(".cueloop/lock/"))
    }
}

/// Require a clean repository, ignoring allowed paths.
///
/// Returns an error if the repository has uncommitted changes outside
/// the allowed paths. The force flag bypasses this check entirely.
///
/// # Arguments
/// * `repo_root` - Path to the repository root
/// * `force` - If true, bypass the check entirely
/// * `allowed_paths` - Paths that are allowed to be dirty
///
/// # Returns
/// * `Ok(())` - Repository is clean or force was true
/// * `Err(GitError::DirtyRepo)` - Repository has disallowed changes
pub fn require_clean_repo_ignoring_paths(
    repo_root: &Path,
    force: bool,
    allowed_paths: &[&str],
) -> Result<(), GitError> {
    let status = match status_porcelain(repo_root) {
        Ok(status) => status,
        Err(err) if is_not_git_worktree_error(&err) => return Ok(()),
        Err(err) => return Err(err),
    };
    if status.trim().is_empty() {
        return Ok(());
    }

    if force {
        return Ok(());
    }

    let mut tracked = Vec::new();
    let mut untracked = Vec::new();

    let entries = parse_porcelain_z_entries(&status)?;
    for entry in entries {
        let path = entry.path.as_str();
        if !path_is_allowed_for_dirty_check(repo_root, path, allowed_paths) {
            let display = format_porcelain_entry(&entry);
            if entry.xy == "??" {
                untracked.push(display);
            } else {
                tracked.push(display);
            }
        }
    }

    if tracked.is_empty() && untracked.is_empty() {
        return Ok(());
    }

    let mut details = String::new();

    if !tracked.is_empty() {
        details.push_str("\n\nTracked changes (suggest 'git stash' or 'git commit'):");
        for line in tracked.iter().take(10) {
            details.push_str("\n  ");
            details.push_str(line);
        }
        if tracked.len() > 10 {
            details.push_str(&format!("\n  ...and {} more", tracked.len() - 10));
        }
    }

    if !untracked.is_empty() {
        details.push_str("\n\nUntracked files (suggest 'git clean -fd' or 'git add'):");
        for line in untracked.iter().take(10) {
            details.push_str("\n  ");
            details.push_str(line);
        }
        if untracked.len() > 10 {
            details.push_str(&format!("\n  ...and {} more", untracked.len() - 10));
        }
    }

    details.push_str("\n\nUse --force to bypass this check if you are sure.");
    Err(GitError::DirtyRepo { details })
}

/// Returns true when the repo has dirty paths and every dirty path is allowed.
///
/// This is useful for detecting if only CueLoop's own files have changed.
pub fn repo_dirty_only_allowed_paths(
    repo_root: &Path,
    allowed_paths: &[&str],
) -> Result<bool, GitError> {
    let status_paths = status_paths(repo_root)?;
    if status_paths.is_empty() {
        return Ok(false);
    }

    let has_disallowed = status_paths
        .iter()
        .any(|path| !path_is_allowed_for_dirty_check(repo_root, path, allowed_paths));
    Ok(!has_disallowed)
}

pub fn capture_dirty_path_baseline_ignoring_paths(
    repo_root: &Path,
    allowed_paths: &[&str],
) -> Result<DirtyPathBaseline, GitError> {
    let dirty_paths = match status_paths(repo_root) {
        Ok(paths) => paths,
        Err(err) if is_not_git_worktree_error(&err) => Vec::new(),
        Err(err) => return Err(err),
    };
    let disallowed_paths: Vec<String> = dirty_paths
        .into_iter()
        .filter(|path| !path_is_allowed_for_dirty_check(repo_root, path, allowed_paths))
        .collect();
    let snapshots = snapshot_paths(repo_root, &disallowed_paths).map_err(GitError::Other)?;
    Ok(DirtyPathBaseline { snapshots })
}

pub fn require_no_unexpected_dirty_paths_since_baseline(
    repo_root: &Path,
    allowed_paths: &[&str],
    baseline: &DirtyPathBaseline,
) -> Result<(), GitError> {
    let status = match status_porcelain(repo_root) {
        Ok(status) => status,
        Err(err) if is_not_git_worktree_error(&err) => return Ok(()),
        Err(err) => return Err(err),
    };
    if status.trim().is_empty() {
        return if baseline.snapshots.is_empty() {
            Ok(())
        } else {
            let changed_paths = changed_paths_from_snapshots(repo_root, &baseline.snapshots)
                .map_err(GitError::Other)?;
            if changed_paths.is_empty() {
                Ok(())
            } else {
                Err(queue_only_dirty_repo_error(
                    repo_root,
                    allowed_paths,
                    Vec::new(),
                    Vec::new(),
                    changed_paths,
                ))
            }
        };
    }

    let baseline_changed =
        changed_paths_from_snapshots(repo_root, &baseline.snapshots).map_err(GitError::Other)?;
    let baseline_paths: std::collections::HashSet<&str> = baseline
        .snapshots
        .iter()
        .map(|snapshot| snapshot.path.as_str())
        .collect();

    let mut tracked = Vec::new();
    let mut untracked = Vec::new();
    let entries = parse_porcelain_z_entries(&status)?;
    for entry in entries {
        let path = entry.path.as_str();
        if path_is_allowed_for_dirty_check(repo_root, path, allowed_paths)
            || baseline_paths.contains(path)
        {
            continue;
        }
        let display = format_porcelain_entry(&entry);
        if entry.xy == "??" {
            untracked.push(display);
        } else {
            tracked.push(display);
        }
    }

    if tracked.is_empty() && untracked.is_empty() && baseline_changed.is_empty() {
        return Ok(());
    }

    Err(queue_only_dirty_repo_error(
        repo_root,
        allowed_paths,
        tracked,
        untracked,
        baseline_changed,
    ))
}

/// Check if a path is allowed to be dirty.
///
/// Handles normalization of paths and directory prefix matching.
pub(crate) fn path_is_allowed_for_dirty_check(
    repo_root: &Path,
    path: &str,
    allowed_paths: &[&str],
) -> bool {
    let Some(normalized) = normalize_path_value(path) else {
        return false;
    };

    let normalized_dir = if normalized.ends_with('/') {
        normalized.to_string()
    } else {
        format!("{}/", normalized)
    };
    let normalized_is_dir = repo_root.join(normalized).is_dir();

    allowed_paths.iter().any(|allowed| {
        let Some(allowed_norm) = normalize_path_value(allowed) else {
            return false;
        };

        if normalized == allowed_norm {
            return true;
        }

        let is_dir_prefix = allowed_norm.ends_with('/') || repo_root.join(allowed_norm).is_dir();
        if !is_dir_prefix {
            return false;
        }

        let allowed_dir = allowed_norm.trim_end_matches('/');
        if allowed_dir.is_empty() {
            return false;
        }

        if normalized == allowed_dir {
            return true;
        }

        let prefix = format!("{}/", allowed_dir);
        if normalized.starts_with(&prefix) || normalized_dir.starts_with(&prefix) {
            return true;
        }

        let allowed_dir_slash = prefix;
        normalized_is_dir && allowed_dir_slash.starts_with(&normalized_dir)
    })
}

/// Normalize a path value for comparison.
fn normalize_path_value(value: &str) -> Option<&str> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(trimmed.strip_prefix("./").unwrap_or(trimmed))
}

/// Format a porcelain entry for display.
fn format_porcelain_entry(entry: &crate::git::status::PorcelainZEntry) -> String {
    if let Some(old) = entry.old_path.as_deref() {
        format!("{} {} -> {}", entry.xy, old, entry.path)
    } else {
        format!("{} {}", entry.xy, entry.path)
    }
}

fn is_not_git_worktree_error(err: &GitError) -> bool {
    match err {
        GitError::CommandFailed { stderr, .. } => stderr.contains("not a git repository"),
        GitError::Other(inner) => inner.to_string().contains("not a git repository"),
        _ => false,
    }
}

fn queue_only_dirty_repo_error(
    repo_root: &Path,
    allowed_paths: &[&str],
    tracked: Vec<String>,
    untracked: Vec<String>,
    changed_baseline_paths: Vec<String>,
) -> GitError {
    let mut details = String::new();
    details.push_str("\n\nQueue-only runner modified unexpected paths.");
    details.push_str("\nAllowed paths:");
    for allowed in allowed_paths {
        details.push_str("\n  - ");
        details.push_str(allowed);
    }

    if !changed_baseline_paths.is_empty() {
        details.push_str("\n\nPreviously dirty disallowed paths changed during the run:");
        for path in changed_baseline_paths.iter().take(10) {
            details.push_str("\n  ");
            details.push_str(path);
        }
        if changed_baseline_paths.len() > 10 {
            details.push_str(&format!(
                "\n  ...and {} more",
                changed_baseline_paths.len() - 10
            ));
        }
    }

    if !tracked.is_empty() {
        details.push_str("\n\nNew tracked changes outside allowed queue-only paths:");
        for line in tracked.iter().take(10) {
            details.push_str("\n  ");
            details.push_str(line);
        }
        if tracked.len() > 10 {
            details.push_str(&format!("\n  ...and {} more", tracked.len() - 10));
        }
    }

    if !untracked.is_empty() {
        details.push_str("\n\nNew untracked files outside allowed queue-only paths:");
        for line in untracked.iter().take(10) {
            details.push_str("\n  ");
            details.push_str(line);
        }
        if untracked.len() > 10 {
            details.push_str(&format!("\n  ...and {} more", untracked.len() - 10));
        }
    }

    let normalized_allowed: Vec<&str> = allowed_paths
        .iter()
        .filter_map(|path| normalize_path_value(path))
        .collect();
    let mut examples_by_kind: BTreeMap<&str, Vec<String>> = BTreeMap::new();
    for path in [
        ".cueloop/queue.jsonc",
        ".cueloop/done.jsonc",
        ".cueloop/cache/execution_history.json",
        ".cueloop/config.jsonc",
        "README.md",
    ] {
        let bucket = if path_is_allowed_for_dirty_check(repo_root, path, &normalized_allowed) {
            "allowed"
        } else {
            "blocked"
        };
        examples_by_kind
            .entry(bucket)
            .or_default()
            .push(path.to_string());
    }
    if let Some(allowed) = examples_by_kind.get("allowed") {
        details.push_str("\n\nExamples still allowed:");
        for path in allowed {
            details.push_str("\n  - ");
            details.push_str(path);
        }
    }
    if let Some(blocked) = examples_by_kind.get("blocked") {
        details.push_str("\n\nExamples blocked by this guard:");
        for path in blocked {
            details.push_str("\n  - ");
            details.push_str(path);
        }
    }

    GitError::DirtyRepo { details }
}

#[cfg(test)]
mod clean_repo_tests {
    use super::*;
    use crate::testsupport::git as git_test;
    use tempfile::TempDir;

    #[test]
    fn run_clean_allowed_paths_include_current_jsonc_runtime_paths() {
        for required in [
            ".cueloop/queue.jsonc",
            ".cueloop/done.jsonc",
            ".cueloop/config.jsonc",
            ".cueloop/cache/",
        ] {
            assert!(
                CUELOOP_RUN_CLEAN_ALLOWED_PATHS.contains(&required),
                "missing required allowlisted path: {required}"
            );
        }
    }

    #[test]
    fn queue_only_allowed_paths_exclude_config() {
        for required in [
            ".cueloop/queue.jsonc",
            ".cueloop/done.jsonc",
            ".cueloop/cache/",
        ] {
            assert!(
                CUELOOP_QUEUE_ONLY_ALLOWED_PATHS.contains(&required),
                "missing required queue-only allowlisted path: {required}"
            );
        }
        assert!(
            !CUELOOP_QUEUE_ONLY_ALLOWED_PATHS.contains(&".cueloop/config.jsonc"),
            "queue-only guard should not allow config mutations"
        );
    }

    #[test]
    fn repo_dirty_only_allowed_paths_detects_config_only_changes() -> anyhow::Result<()> {
        let temp = TempDir::new()?;
        git_test::init_repo(temp.path())?;
        std::fs::create_dir_all(temp.path().join(".cueloop"))?;
        let config_path = temp.path().join(".cueloop/config.jsonc");
        std::fs::write(&config_path, "{ \"version\": 1 }")?;
        git_test::git_run(temp.path(), &["add", "-f", ".cueloop/config.jsonc"])?;
        git_test::git_run(temp.path(), &["commit", "-m", "init config"])?;

        std::fs::write(&config_path, "{ \"version\": 2 }")?;

        let dirty_allowed =
            repo_dirty_only_allowed_paths(temp.path(), CUELOOP_RUN_CLEAN_ALLOWED_PATHS)?;
        assert!(dirty_allowed, "expected config-only changes to be allowed");
        require_clean_repo_ignoring_paths(temp.path(), false, CUELOOP_RUN_CLEAN_ALLOWED_PATHS)?;
        Ok(())
    }

    #[test]
    fn repo_dirty_only_allowed_paths_detects_current_config_jsonc_only_changes()
    -> anyhow::Result<()> {
        let temp = TempDir::new()?;
        git_test::init_repo(temp.path())?;
        std::fs::create_dir_all(temp.path().join(".cueloop"))?;
        let config_path = temp.path().join(".cueloop/config.jsonc");
        std::fs::write(&config_path, "{ \"version\": 1 }")?;
        git_test::git_run(temp.path(), &["add", "-f", ".cueloop/config.jsonc"])?;
        git_test::git_run(temp.path(), &["commit", "-m", "init config jsonc"])?;

        std::fs::write(&config_path, "{ \"version\": 2 }")?;

        let dirty_allowed =
            repo_dirty_only_allowed_paths(temp.path(), CUELOOP_RUN_CLEAN_ALLOWED_PATHS)?;
        assert!(
            dirty_allowed,
            "expected config.jsonc-only changes to be allowed"
        );
        require_clean_repo_ignoring_paths(temp.path(), false, CUELOOP_RUN_CLEAN_ALLOWED_PATHS)?;
        Ok(())
    }

    #[test]
    fn repo_dirty_only_allowed_paths_rejects_other_changes() -> anyhow::Result<()> {
        let temp = TempDir::new()?;
        git_test::init_repo(temp.path())?;
        std::fs::write(temp.path().join("notes.txt"), "hello")?;

        let dirty_allowed =
            repo_dirty_only_allowed_paths(temp.path(), CUELOOP_RUN_CLEAN_ALLOWED_PATHS)?;
        assert!(!dirty_allowed, "expected untracked change to be disallowed");
        Ok(())
    }

    #[test]
    fn repo_dirty_only_allowed_paths_accepts_directory_prefix_with_trailing_slash()
    -> anyhow::Result<()> {
        let temp = TempDir::new()?;
        git_test::init_repo(temp.path())?;
        std::fs::create_dir_all(temp.path().join("cache/plans"))?;
        std::fs::write(temp.path().join("cache/plans/plan.md"), "plan")?;

        let dirty_allowed = repo_dirty_only_allowed_paths(temp.path(), &["cache/plans/"])?;
        assert!(dirty_allowed, "expected directory prefix to be allowed");
        require_clean_repo_ignoring_paths(temp.path(), false, &["cache/plans/"])?;
        Ok(())
    }

    #[test]
    fn repo_dirty_only_allowed_paths_accepts_existing_directory_prefix_without_slash()
    -> anyhow::Result<()> {
        let temp = TempDir::new()?;
        git_test::init_repo(temp.path())?;
        std::fs::create_dir_all(temp.path().join("cache"))?;
        std::fs::write(temp.path().join("cache/notes.txt"), "notes")?;

        let dirty_allowed = repo_dirty_only_allowed_paths(temp.path(), &["cache"])?;
        assert!(dirty_allowed, "expected existing directory to be allowed");
        require_clean_repo_ignoring_paths(temp.path(), false, &["cache"])?;
        Ok(())
    }

    #[test]
    fn repo_dirty_only_allowed_paths_rejects_paths_outside_allowed_directory() -> anyhow::Result<()>
    {
        let temp = TempDir::new()?;
        git_test::init_repo(temp.path())?;
        std::fs::create_dir_all(temp.path().join("cache"))?;
        std::fs::write(temp.path().join("cache/notes.txt"), "notes")?;
        std::fs::write(temp.path().join("other.txt"), "nope")?;

        let dirty_allowed = repo_dirty_only_allowed_paths(temp.path(), &["cache/"])?;
        assert!(!dirty_allowed, "expected other paths to be disallowed");
        assert!(
            require_clean_repo_ignoring_paths(temp.path(), false, &["cache/"]).is_err(),
            "expected clean-repo enforcement to fail"
        );
        Ok(())
    }

    #[test]
    fn require_no_unexpected_dirty_paths_since_baseline_allows_queue_changes() -> anyhow::Result<()>
    {
        let temp = TempDir::new()?;
        git_test::init_repo(temp.path())?;
        std::fs::create_dir_all(temp.path().join(".cueloop/cache"))?;
        std::fs::write(temp.path().join("README.md"), "baseline\n")?;
        std::fs::write(temp.path().join(".cueloop/queue.jsonc"), "{}")?;
        std::fs::write(
            temp.path().join(".cueloop/cache/execution_history.json"),
            "{}",
        )?;
        git_test::commit_all(temp.path(), "init")?;

        let baseline = capture_dirty_path_baseline_ignoring_paths(
            temp.path(),
            CUELOOP_QUEUE_ONLY_ALLOWED_PATHS,
        )?;
        std::fs::write(
            temp.path().join(".cueloop/queue.jsonc"),
            "{\n  \"version\": 1\n}",
        )?;
        std::fs::write(
            temp.path().join(".cueloop/cache/execution_history.json"),
            "{\n  \"ok\": true\n}",
        )?;

        require_no_unexpected_dirty_paths_since_baseline(
            temp.path(),
            CUELOOP_QUEUE_ONLY_ALLOWED_PATHS,
            &baseline,
        )?;
        Ok(())
    }

    #[test]
    fn require_no_unexpected_dirty_paths_since_baseline_rejects_new_source_and_config_changes()
    -> anyhow::Result<()> {
        let temp = TempDir::new()?;
        git_test::init_repo(temp.path())?;
        std::fs::create_dir_all(temp.path().join(".cueloop"))?;
        std::fs::write(temp.path().join("README.md"), "baseline\n")?;
        std::fs::write(temp.path().join(".cueloop/config.jsonc"), "{}")?;
        git_test::commit_all(temp.path(), "init")?;

        let baseline = capture_dirty_path_baseline_ignoring_paths(
            temp.path(),
            CUELOOP_QUEUE_ONLY_ALLOWED_PATHS,
        )?;
        std::fs::write(temp.path().join("README.md"), "changed\n")?;
        std::fs::write(
            temp.path().join(".cueloop/config.jsonc"),
            "{\n  \"v\": 2\n}",
        )?;

        let err = require_no_unexpected_dirty_paths_since_baseline(
            temp.path(),
            CUELOOP_QUEUE_ONLY_ALLOWED_PATHS,
            &baseline,
        )
        .expect_err("expected queue-only guard to reject README/config changes");
        let message = err.to_string();
        assert!(message.contains("Queue-only runner modified unexpected paths"));
        assert!(message.contains("README.md"));
        assert!(message.contains(".cueloop/config.jsonc"));
        Ok(())
    }

    #[test]
    fn require_no_unexpected_dirty_paths_since_baseline_rejects_changed_forced_baseline_path()
    -> anyhow::Result<()> {
        let temp = TempDir::new()?;
        git_test::init_repo(temp.path())?;
        std::fs::write(temp.path().join("README.md"), "baseline\n")?;
        git_test::commit_all(temp.path(), "init")?;

        std::fs::write(temp.path().join("README.md"), "pre-existing dirt\n")?;
        let baseline = capture_dirty_path_baseline_ignoring_paths(
            temp.path(),
            CUELOOP_QUEUE_ONLY_ALLOWED_PATHS,
        )?;
        std::fs::write(temp.path().join("README.md"), "runner changed dirt\n")?;

        let err = require_no_unexpected_dirty_paths_since_baseline(
            temp.path(),
            CUELOOP_QUEUE_ONLY_ALLOWED_PATHS,
            &baseline,
        )
        .expect_err("expected changed baseline path to be rejected");
        let message = err.to_string();
        assert!(message.contains("Previously dirty disallowed paths changed during the run"));
        assert!(message.contains("README.md"));
        Ok(())
    }

    #[test]
    fn execution_history_json_is_in_allowed_paths() -> anyhow::Result<()> {
        // Verify that execution_history.json is covered by CUELOOP_RUN_CLEAN_ALLOWED_PATHS
        // via the .cueloop/cache/ directory prefix
        let temp = TempDir::new()?;
        git_test::init_repo(temp.path())?;
        std::fs::create_dir_all(temp.path().join(".cueloop/cache"))?;
        std::fs::write(
            temp.path().join(".cueloop/cache/execution_history.json"),
            "{}",
        )?;

        let dirty_allowed =
            repo_dirty_only_allowed_paths(temp.path(), CUELOOP_RUN_CLEAN_ALLOWED_PATHS)?;
        assert!(
            dirty_allowed,
            "execution_history.json should be covered by CUELOOP_RUN_CLEAN_ALLOWED_PATHS"
        );
        Ok(())
    }
}
