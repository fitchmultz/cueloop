//! Resolved settings and RepoPrompt override policy for parallel runs.
//!
//! Purpose:
//! - Resolved settings and RepoPrompt override policy for parallel runs.
//!
//! Responsibilities:
//! - Build `ParallelSettings` from resolved config and CLI options.
//! - Apply agent override rules specific to parallel worker processes.
//!
//! Not handled here:
//! - Workspace-root gitignore validation (see `preflight.rs`).
//! - Orchestration or worker lifecycle.
//!
//!
//! Usage:
//! - Used through the crate module tree or integration test harness.
//!
//! Invariants/assumptions:
//! - `Resolved` reflects the active repo and merged config.
//! - Parallel workers must not inherit RepoPrompt plan/tooling modes that assume a single workspace.

use crate::agent::AgentOverrides;
use crate::config;
use crate::git;
use anyhow::Result;
use std::path::PathBuf;

/// Default push backoff intervals in milliseconds.
pub fn default_push_backoff_ms() -> Vec<u64> {
    vec![500, 2000, 5000, 10000]
}

pub(crate) struct ParallelRunOptions {
    pub max_tasks: u32,
    pub workers: u8,
    pub agent_overrides: AgentOverrides,
    pub force: bool,
}

pub(crate) struct ParallelSettings {
    pub(crate) workers: u8,
    pub(crate) workspace_root: PathBuf,
}

pub(crate) fn resolve_parallel_settings(
    resolved: &config::Resolved,
    opts: &ParallelRunOptions,
) -> Result<ParallelSettings> {
    Ok(ParallelSettings {
        workers: opts.workers,
        workspace_root: git::workspace_root(&resolved.repo_root, &resolved.config),
    })
}

pub(crate) fn overrides_for_parallel_workers(
    resolved: &config::Resolved,
    overrides: &AgentOverrides,
) -> AgentOverrides {
    let repoprompt_flags =
        crate::agent::resolve_repoprompt_flags_from_overrides(overrides, resolved);
    if repoprompt_flags.plan_required || repoprompt_flags.tool_injection {
        log::warn!(
            "Parallel workers disable RepoPrompt plan/tooling instructions to keep edits in workspace clones."
        );
    }

    let mut worker_overrides = overrides.clone();
    worker_overrides.repoprompt_plan_required = Some(false);
    worker_overrides.repoprompt_tool_injection = Some(false);
    worker_overrides
}
