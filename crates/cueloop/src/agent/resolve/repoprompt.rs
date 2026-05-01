//! Purpose: Bridge resolved CLI overrides with RepoPrompt configuration
//! fallback behavior.
//!
//! Responsibilities:
//! - Merge explicit CLI RepoPrompt overrides with resolved config defaults.
//! - Return the effective `RepopromptFlags` used by run execution surfaces.
//!
//! Scope:
//! - RepoPrompt flag fallback only; CLI argument parsing lives in `run.rs` and
//!   agent-config resolution lives in `super::super::repoprompt`.
//!
//! Usage:
//! - Called by run orchestration code that needs the effective RepoPrompt mode
//!   after override resolution.
//!
//! Invariants/Assumptions:
//! - Explicit CLI overrides always win over config defaults.
//! - Unset CLI flags fall back to the resolved agent config.

use crate::config;

use super::super::repoprompt::{RepopromptFlags, resolve_repoprompt_flags_from_agent_config};
use super::types::AgentOverrides;

/// Resolve RepoPrompt flags from overrides, falling back to config.
pub fn resolve_repoprompt_flags_from_overrides(
    overrides: &AgentOverrides,
    resolved: &config::Resolved,
) -> RepopromptFlags {
    let config_flags = resolve_repoprompt_flags_from_agent_config(&resolved.config.agent);
    let plan_required = overrides
        .repoprompt_plan_required
        .unwrap_or(config_flags.plan_required);
    let tool_injection = overrides
        .repoprompt_tool_injection
        .unwrap_or(config_flags.tool_injection);
    RepopromptFlags {
        plan_required,
        tool_injection,
    }
}
