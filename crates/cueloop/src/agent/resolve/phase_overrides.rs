//! Purpose: Parse and assemble per-phase CLI override values for run commands.
//!
//! Responsibilities:
//! - Resolve a single phase override from optional runner/model/effort flags.
//! - Assemble the full `PhaseOverrides` structure across phases 1-3.
//!
//! Scope:
//! - Phase-specific override parsing only; top-level override resolution lives
//!   in `run.rs`.
//!
//! Usage:
//! - Called from `resolve_run_agent_overrides` when run-command flags are
//!   parsed.
//!
//! Invariants/Assumptions:
//! - Returns `None` when a phase has no override flags at all.
//! - Returns `None` for the overall phase override set when all phases are
//!   empty.

use anyhow::Result;

use crate::contracts::{PhaseOverrideConfig, PhaseOverrides};
use crate::runner;

use super::super::args::RunAgentArgs;
use super::super::parse::parse_runner;

/// Helper to resolve phase overrides for a single phase.
///
/// Takes optional runner, model, and effort strings and returns a
/// `PhaseOverrideConfig` if any are provided.
pub(super) fn resolve_single_phase_override(
    runner: Option<&str>,
    model: Option<&str>,
    effort: Option<&str>,
) -> Result<Option<PhaseOverrideConfig>> {
    if runner.is_none() && model.is_none() && effort.is_none() {
        return Ok(None);
    }

    Ok(Some(PhaseOverrideConfig {
        runner: runner.map(parse_runner).transpose()?,
        model: model.map(runner::parse_model).transpose()?,
        reasoning_effort: effort.map(runner::parse_reasoning_effort).transpose()?,
        cursor: None,
    }))
}

/// Resolve phase-specific overrides from CLI arguments.
pub(super) fn resolve_phase_overrides(args: &RunAgentArgs) -> Result<Option<PhaseOverrides>> {
    let phase1 = resolve_single_phase_override(
        args.runner_phase1.as_deref(),
        args.model_phase1.as_deref(),
        args.effort_phase1.as_deref(),
    )?;
    let phase2 = resolve_single_phase_override(
        args.runner_phase2.as_deref(),
        args.model_phase2.as_deref(),
        args.effort_phase2.as_deref(),
    )?;
    let phase3 = resolve_single_phase_override(
        args.runner_phase3.as_deref(),
        args.model_phase3.as_deref(),
        args.effort_phase3.as_deref(),
    )?;

    if phase1.is_none() && phase2.is_none() && phase3.is_none() {
        Ok(None)
    } else {
        Ok(Some(PhaseOverrides {
            phase1,
            phase2,
            phase3,
        }))
    }
}
