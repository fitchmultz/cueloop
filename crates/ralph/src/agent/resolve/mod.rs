//! Purpose: Provide the public CLI agent-override resolution surface.
//!
//! Responsibilities:
//! - Declare the `agent::resolve` child modules.
//! - Re-export the stable public API used through `crate::agent::*`.
//!
//! Scope:
//! - Thin facade only; implementation lives in sibling files under
//!   `agent/resolve/`.
//!
//! Usage:
//! - Import `AgentOverrides` and resolution helpers through `crate::agent` or
//!   `crate::agent::resolve`.
//!
//! Invariants/Assumptions:
//! - The public API surface remains stable across this split.
//! - Override parsing, validation, and RepoPrompt fallback behavior remain
//!   unchanged.

mod phase_overrides;
mod repoprompt;
mod run;

#[cfg(test)]
mod tests;

mod types;

pub use repoprompt::resolve_repoprompt_flags_from_overrides;
pub use run::{resolve_agent_overrides, resolve_run_agent_overrides};
pub use types::AgentOverrides;
