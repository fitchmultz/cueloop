//! Parallel run-loop configuration.
//!
//! Purpose:
//! - Parallel run-loop configuration.
//!
//! Responsibilities:
//! - Define parallel config struct, merge behavior, and related enums.
//!
//! Not handled here:
//! - Parallel execution logic (see `crate::parallel` module).
//!
//! Usage:
//! - Used through the crate module tree or integration test harness.
//!
//! Invariants/Assumptions:
//! - Keep behavior aligned with Ralph's canonical CLI, machine-contract, and queue semantics.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Parallel run-loop configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct ParallelConfig {
    /// Number of workers to run concurrently when parallel mode is enabled.
    #[schemars(range(min = 2))]
    pub workers: Option<u8>,

    /// Root directory for parallel workspaces (relative to repo root if not absolute).
    pub workspace_root: Option<PathBuf>,

    /// Maximum number of push attempts before giving up.
    #[schemars(range(min = 1))]
    pub max_push_attempts: Option<u8>,

    /// Backoff intervals in milliseconds for push retries.
    pub push_backoff_ms: Option<Vec<u64>>,

    /// Hours to retain blocked workspaces before cleanup.
    #[schemars(range(min = 1))]
    pub workspace_retention_hours: Option<u32>,

    /// Additional gitignored non-.ralph files/globs to sync into parallel worker workspaces.
    /// Entries must be repo-relative file paths or glob patterns. Directories,
    /// absolute paths, `..`, and known heavy/runtime paths are rejected.
    pub ignored_file_allowlist: Option<Vec<String>>,
}

impl ParallelConfig {
    pub fn merge_from(&mut self, other: Self) {
        if other.workers.is_some() {
            self.workers = other.workers;
        }
        if other.workspace_root.is_some() {
            self.workspace_root = other.workspace_root;
        }
        if other.max_push_attempts.is_some() {
            self.max_push_attempts = other.max_push_attempts;
        }
        if other.push_backoff_ms.is_some() {
            self.push_backoff_ms = other.push_backoff_ms;
        }
        if other.workspace_retention_hours.is_some() {
            self.workspace_retention_hours = other.workspace_retention_hours;
        }
        if other.ignored_file_allowlist.is_some() {
            self.ignored_file_allowlist = other.ignored_file_allowlist;
        }
    }
}

/// Default push backoff intervals in milliseconds.
pub fn default_push_backoff_ms() -> Vec<u64> {
    vec![500, 2000, 5000, 10000]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parallel_config_merge_prefers_other_when_some() {
        let workspace_root = crate::testsupport::path::portable_abs_path("ws");
        let mut base = ParallelConfig {
            workers: Some(2),
            workspace_root: None,
            max_push_attempts: Some(3),
            push_backoff_ms: None,
            workspace_retention_hours: Some(12),
            ignored_file_allowlist: Some(vec!["local/base.json".to_string()]),
        };

        let other = ParallelConfig {
            workers: Some(4),
            workspace_root: Some(workspace_root.clone()),
            max_push_attempts: None,
            push_backoff_ms: Some(vec![1000, 2000]),
            workspace_retention_hours: None,
            ignored_file_allowlist: Some(vec!["local/worker.json".to_string()]),
        };

        base.merge_from(other);

        assert_eq!(base.workers, Some(4));
        assert_eq!(base.workspace_root, Some(workspace_root));
        assert_eq!(base.max_push_attempts, Some(3)); // unchanged
        assert_eq!(base.push_backoff_ms, Some(vec![1000, 2000]));
        assert_eq!(base.workspace_retention_hours, Some(12)); // unchanged
        assert_eq!(
            base.ignored_file_allowlist,
            Some(vec!["local/worker.json".to_string()])
        );
    }

    #[test]
    fn default_push_backoff_ms_has_expected_values() {
        let backoff = default_push_backoff_ms();
        assert_eq!(backoff, vec![500, 2000, 5000, 10000]);
    }
}
