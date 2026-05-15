//! State synchronization and git helpers for parallel workers.
//!
//! Purpose:
//! - State synchronization and git helpers for parallel workers.
//!
//! Responsibilities:
//! - Sync repo-local runtime state into worker workspaces.
//! - Commit changes on worker failure when diagnostics are needed.
//! - Provide push helpers for workspace branch synchronization.
//!
//! Not handled here:
//! - Worker lifecycle (see `super::worker`).
//! - Coordinator orchestration (see `super::orchestration`).
//!
//!
//! Usage:
//! - Used through the crate module tree or integration test harness.
//!
//! Invariants/assumptions:
//! - Worker queue/done paths are seeded from coordinator resolved paths.
//! - Workspace paths are valid and writable.

pub(crate) mod bookkeeping;
mod common;
mod gitignored;
mod runtime;

use crate::config;
use anyhow::{Context, Result};
use std::path::Path;

use gitignored::sync_gitignored;
pub(crate) use gitignored::{
    is_denied_parallel_ignored_sync_path, preflight_parallel_ignored_file_allowlist,
    validate_parallel_ignored_file_allowlist_config,
};
use runtime::sync_cueloop_runtime_tree;

/// Sync cueloop state files from repo root to workspace.
///
/// Syncs `.cueloop/` runtime files plus gitignored allowlisted files.
/// Ephemeral `.cueloop` runtime paths are intentionally NOT synchronized.
/// Queue/done files are seeded explicitly using resolved queue/done paths so
/// parallel workers work with `.jsonc` migrations and gitignored `.cueloop` setups.
pub(crate) fn sync_cueloop_state(resolved: &config::Resolved, workspace_path: &Path) -> Result<()> {
    let target = workspace_path.join(".cueloop");
    std::fs::create_dir_all(&target)
        .with_context(|| format!("create workspace cueloop dir {}", target.display()))?;

    let source = resolved.repo_root.join(".cueloop");
    sync_cueloop_runtime_tree(resolved, &source, &target)?;
    sync_worker_bookkeeping_files(resolved, workspace_path)?;
    sync_gitignored(resolved, workspace_path)?;

    Ok(())
}

fn sync_worker_bookkeeping_files(resolved: &config::Resolved, workspace_path: &Path) -> Result<()> {
    sync_worker_bookkeeping_file(resolved, workspace_path, &resolved.queue_path, "queue")?;
    sync_worker_bookkeeping_file(resolved, workspace_path, &resolved.done_path, "done")?;
    Ok(())
}

fn sync_worker_bookkeeping_file(
    resolved: &config::Resolved,
    workspace_path: &Path,
    source_path: &Path,
    label: &str,
) -> Result<()> {
    let target_path = super::path_map::map_resolved_path_into_workspace(
        &resolved.repo_root,
        workspace_path,
        source_path,
        label,
    )
    .with_context(|| format!("map {} bookkeeping path into workspace", label))?;

    common::sync_file_if_exists(source_path, &target_path)
        .with_context(|| format!("sync {} bookkeeping file to workspace", label))
}

#[cfg(test)]
mod tests;
