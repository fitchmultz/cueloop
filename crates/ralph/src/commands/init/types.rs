//! Initialization workflow types.
//!
//! Purpose:
//! - Centralize public option and report types for `crate::commands::init`.
//!
//! Responsibilities:
//! - Define initialization options accepted by the workflow.
//! - Describe per-file initialization outcomes.
//! - Capture the final report returned by `run_init`.
//!
//! Scope:
//! - Shared data types only; orchestration lives in `workflow.rs`.
//!
//! Usage:
//! - Imported by CLI handlers, tutorials, tests, and the workflow module.
//!
//! Invariants/assumptions:
//! - Reported file paths reflect the actual paths used for initialization.
//! - File status values stay aligned with writer behavior.

/// Options for initializing Ralph files.
pub struct InitOptions {
    /// Overwrite existing files if they already exist.
    pub force: bool,
    /// Force remove stale locks.
    pub force_lock: bool,
    /// Run interactive onboarding wizard.
    pub interactive: bool,
    /// Update README if it exists (force overwrite with latest template).
    pub update_readme: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileInitStatus {
    Created,
    Valid,
    Updated,
}

#[derive(Debug)]
pub struct InitReport {
    pub queue_status: FileInitStatus,
    pub done_status: FileInitStatus,
    pub config_status: FileInitStatus,
    /// (status, version) tuple - version is Some if README was read/created
    pub readme_status: Option<(FileInitStatus, Option<u32>)>,
    /// Paths that were actually used for file creation (may differ from resolved paths)
    pub queue_path: std::path::PathBuf,
    pub done_path: std::path::PathBuf,
    pub config_path: std::path::PathBuf,
}
