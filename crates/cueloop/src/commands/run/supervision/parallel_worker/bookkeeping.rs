//! Parallel-worker bookkeeping restore helpers.
//!
//! Purpose:
//! - Restore workspace-local bookkeeping files and generated artifacts after a worker run.
//!
//! Responsibilities:
//! - Restore tracked queue/done/productivity files to HEAD.
//! - Remove generated runtime artifacts under `.cueloop/cache` and `.cueloop/logs`.
//! - Detect lingering bookkeeping dirtiness after restore attempts.
//!
//! Scope:
//! - Bookkeeping cleanup only; CI marker writing and supervision orchestration live elsewhere.
//!
//! Usage:
//! - Called by `parallel_worker::mod` and companion tests.
//!
//! Invariants/assumptions:
//! - Bookkeeping restore is retried once before surfacing failure.
//! - Generated plan cache cleanup must preserve tracked plan snapshots by restoring them from HEAD.

use anyhow::{Context, Result};

use crate::{git, promptflow};

const PRODUCTIVITY_BOOKKEEPING_FILES: [&str; 2] = [
    ".cueloop/cache/productivity.json",
    ".cueloop/cache/productivity.jsonc",
];

const GENERATED_BOOKKEEPING_STATUS_FRAGMENTS: [&str; 8] = [
    ".cueloop/cache/productivity.json",
    ".cueloop/cache/productivity.jsonc",
    ".cueloop/cache/plans/",
    ".cueloop/cache/phase2_final/",
    ".cueloop/cache/session.jsonc",
    ".cueloop/cache/migrations.jsonc",
    ".cueloop/cache/parallel/",
    ".cueloop/logs/",
];

const GENERATED_PARALLEL_PATHS: [&str; 5] = [
    ".cueloop/cache/phase2_final",
    ".cueloop/cache/session.jsonc",
    ".cueloop/cache/migrations.jsonc",
    ".cueloop/cache/parallel",
    ".cueloop/logs",
];

pub(super) fn restore_parallel_worker_bookkeeping_and_check_clean(
    resolved: &crate::config::Resolved,
    task_id: &str,
) -> Result<String> {
    for attempt in 0..2 {
        restore_parallel_worker_bookkeeping(resolved, task_id)?;
        let status = git::status_porcelain(&resolved.repo_root)?;
        let bookkeeping_lines = collect_bookkeeping_status_lines(resolved, &status);
        if bookkeeping_lines.is_empty() {
            return Ok(status);
        }
        if attempt == 1 {
            anyhow::bail!(
                "parallel bookkeeping files remained dirty after restore: {}",
                bookkeeping_lines.join(", ")
            );
        }
    }
    unreachable!("loop returns on success and errors on final failure")
}

pub(super) fn restore_parallel_worker_bookkeeping(
    resolved: &crate::config::Resolved,
    task_id: &str,
) -> Result<()> {
    let mut paths = vec![resolved.queue_path.clone(), resolved.done_path.clone()];
    paths.extend(repo_paths(
        &resolved.repo_root,
        &PRODUCTIVITY_BOOKKEEPING_FILES,
    ));
    git::restore_tracked_paths_to_head(&resolved.repo_root, &paths)
        .context("restore queue/done/productivity to HEAD")?;
    remove_parallel_worker_generated_artifacts(&resolved.repo_root, task_id)?;
    Ok(())
}

pub(super) fn collect_bookkeeping_status_lines(
    resolved: &crate::config::Resolved,
    status: &str,
) -> Vec<String> {
    let path_fragments = parallel_bookkeeping_status_fragments(resolved);
    status
        .split_terminator(['\0', '\n'])
        .filter(|line| {
            path_fragments
                .iter()
                .any(|path| !path.is_empty() && line.contains(path))
        })
        .map(std::string::ToString::to_string)
        .collect()
}

fn parallel_bookkeeping_status_fragments(resolved: &crate::config::Resolved) -> Vec<String> {
    let mut fragments = Vec::new();
    push_repo_relative_status_fragment(&mut fragments, &resolved.repo_root, &resolved.queue_path);
    push_repo_relative_status_fragment(&mut fragments, &resolved.repo_root, &resolved.done_path);
    fragments.extend(
        GENERATED_BOOKKEEPING_STATUS_FRAGMENTS
            .iter()
            .map(std::string::ToString::to_string),
    );
    fragments
}

fn push_repo_relative_status_fragment(
    fragments: &mut Vec<String>,
    repo_root: &std::path::Path,
    path: &std::path::Path,
) {
    let Ok(relative) = path.strip_prefix(repo_root) else {
        return;
    };
    if relative.as_os_str().is_empty() {
        return;
    }
    fragments.push(relative.to_string_lossy().replace('\\', "/"));
}

fn remove_parallel_worker_generated_artifacts(
    repo_root: &std::path::Path,
    task_id: &str,
) -> Result<()> {
    cleanup_plan_cache(repo_root, task_id)?;

    for path in repo_paths(repo_root, &GENERATED_PARALLEL_PATHS) {
        remove_path_if_exists(&path)?;
    }

    Ok(())
}

fn cleanup_plan_cache(repo_root: &std::path::Path, task_id: &str) -> Result<()> {
    let trimmed = task_id.trim();
    if trimmed.is_empty() {
        return Ok(());
    }
    let plan_path = promptflow::plan_cache_path(repo_root, trimmed);

    if plan_path.exists() {
        if plan_path.is_dir() {
            std::fs::remove_dir_all(&plan_path).with_context(|| {
                format!("remove generated plan directory {}", plan_path.display())
            })?;
        } else {
            std::fs::remove_file(&plan_path)
                .with_context(|| format!("remove generated plan cache {}", plan_path.display()))?;
        }
    }

    git::restore_tracked_paths_to_head(repo_root, &[plan_path])
        .context("restore tracked plan cache to HEAD")?;

    Ok(())
}

fn repo_paths(repo_root: &std::path::Path, relative_paths: &[&str]) -> Vec<std::path::PathBuf> {
    relative_paths
        .iter()
        .map(|relative_path| repo_root.join(relative_path))
        .collect()
}

fn remove_path_if_exists(path: &std::path::Path) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }

    if path.is_dir() {
        std::fs::remove_dir_all(path)
            .with_context(|| format!("remove generated directory {}", path.display()))?;
    } else {
        std::fs::remove_file(path)
            .with_context(|| format!("remove generated file {}", path.display()))?;
    }

    Ok(())
}
