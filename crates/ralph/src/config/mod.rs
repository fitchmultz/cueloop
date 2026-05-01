//! Configuration resolution facade for CueLoop's transitional `ralph` CLI.
//!
//! Purpose:
//! - Expose the config layer, resolution, trust, and validation APIs from one module.
//!
//! Responsibilities:
//! - Resolve configuration from defaults, legacy/current global config, and active project config.
//! - Load and parse JSONC config files via `load_layer`.
//! - Merge configuration layers via `ConfigLayer` and `apply_layer`.
//! - Validate configuration values (version, paths, numeric ranges, runner binaries).
//! - Resolve queue/done file paths, ID generation settings, and active runtime layout.
//! - Discover repository roots via `.cueloop/`, legacy `.ralph/`, or `.git/` markers.
//!
//! Not handled here:
//! - CLI argument parsing (see `crate::cli`).
//! - Queue operations like task CRUD (see `crate::queue`).
//! - Runner execution or agent invocation (see `crate::runner`).
//! - Prompt rendering or template processing (see `crate::prompts_internal`).
//! - Lock management (see `crate::lock`).
//!
//! Usage:
//! - Used through the crate module tree or integration test harness.
//!
//! Invariants/assumptions:
//! - Config version must be 2; unsupported versions are rejected.
//! - Paths are resolved relative to repo root unless absolute.
//! - Global config resolves from `~/.config/cueloop/config.jsonc` with legacy `~/.config/ralph/config.jsonc` fallback.
//! - Project config resolves from `.cueloop/config.jsonc` or legacy `.ralph/config.jsonc` based on active runtime markers.
//! - Config layers are applied in this order: defaults, legacy global, current global, then project.
//! - `save_layer` creates parent directories automatically if needed.

use std::path::PathBuf;

mod layer;
mod resolution;
mod trust;
mod validation;

#[cfg(test)]
mod tests;

// Re-export main types and functions for backward compatibility
pub use layer::{ConfigLayer, apply_layer, load_layer, save_layer};
pub use resolution::{
    ProjectRuntimeLayout, find_repo_root, global_config_path, legacy_global_config_path,
    project_config_path, project_runtime_dir, project_runtime_layout, resolve_done_path,
    resolve_from_cwd, resolve_from_cwd_for_doctor,
    resolve_from_cwd_skipping_project_execution_trust, resolve_from_cwd_with_profile,
    resolve_id_prefix, resolve_id_width, resolve_queue_path,
};
pub use trust::{
    RepoTrust, TrustFileInitStatus, initialize_repo_trust_file, load_repo_trust, project_trust_path,
};
pub(crate) use validation::{
    CiGateArgvIssue, ERR_PROJECT_EXECUTION_TRUST, detect_ci_gate_argv_issue,
};
pub use validation::{
    git_ref_invalid_reason, validate_agent_binary_paths, validate_agent_patch, validate_config,
    validate_project_execution_trust, validate_queue_done_file_override,
    validate_queue_file_override, validate_queue_id_prefix_override,
    validate_queue_id_width_override, validate_queue_overrides,
};

/// Resolved configuration including computed paths.
#[derive(Debug, Clone)]
pub struct Resolved {
    pub config: crate::contracts::Config,
    pub repo_root: PathBuf,
    pub queue_path: PathBuf,
    pub done_path: PathBuf,
    pub id_prefix: String,
    pub id_width: usize,
    pub global_config_path: Option<PathBuf>,
    pub project_config_path: Option<PathBuf>,
}
