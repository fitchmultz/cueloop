//! Purpose: Define shared data types for resolved CLI agent overrides.
//!
//! Responsibilities:
//! - Represent the fully resolved override set used by run, scan, and task
//!   workflows.
//! - Keep the stable `AgentOverrides` shape re-exported through `crate::agent`.
//!
//! Scope:
//! - Data modeling only; parsing and resolution live in sibling modules.
//!
//! Usage:
//! - Constructed by `resolve_run_agent_overrides` and
//!   `resolve_agent_overrides`.
//!
//! Invariants/Assumptions:
//! - These overrides always take precedence over task-level and config
//!   defaults.
//! - Phase-specific overrides are only populated when at least one phase flag
//!   is set.

use crate::contracts::{
    GitPublishMode, GitRevertMode, Model, PhaseOverrides, ReasoningEffort, Runner,
    RunnerCliOptionsPatch,
};

/// Agent overrides from CLI arguments.
///
/// These overrides take precedence over task.agent and config defaults.
#[derive(Debug, Clone, Default)]
pub struct AgentOverrides {
    /// Named configuration profile to apply.
    pub profile: Option<String>,
    pub runner: Option<Runner>,
    pub model: Option<Model>,
    pub reasoning_effort: Option<ReasoningEffort>,
    pub runner_cli: RunnerCliOptionsPatch,
    /// Execution shape override:
    /// - 1 => single-pass execution
    /// - 2 => two-pass execution (plan then implement)
    /// - 3 => three-pass execution (plan, implement+CI, review+complete)
    pub phases: Option<u8>,
    pub repoprompt_plan_required: Option<bool>,
    pub repoprompt_tool_injection: Option<bool>,
    pub git_revert_mode: Option<GitRevertMode>,
    pub git_publish_mode: Option<GitPublishMode>,
    pub include_draft: Option<bool>,
    /// Enable/disable desktop notification on task completion.
    pub notify_on_complete: Option<bool>,
    /// Enable/disable desktop notification on task failure.
    pub notify_on_fail: Option<bool>,
    /// Enable/disable desktop notification when loop completes.
    pub notify_on_loop_complete: Option<bool>,
    /// Enable sound alert with notification.
    pub notify_sound: Option<bool>,
    /// Enable strict LFS validation before commit.
    pub lfs_check: Option<bool>,
    /// Disable progress indicators and celebrations.
    pub no_progress: Option<bool>,
    /// Per-phase overrides from CLI (phase1, phase2, phase3).
    pub phase_overrides: Option<PhaseOverrides>,
}
