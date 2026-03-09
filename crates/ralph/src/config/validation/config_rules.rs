//! Full-config validation orchestration.
//!
//! Responsibilities:
//! - Validate top-level config version and cross-domain settings.
//! - Delegate queue, agent, CI gate, and profile checks to focused validators.
//!
//! Not handled here:
//! - Config loading/merging.
//! - Queue file contents or lock state.
//!
//! Invariants/assumptions:
//! - Parallel workspace roots must be normalized paths.
//! - Profile agent patches reuse the same agent validator used elsewhere.

use super::{
    agent::validate_agent_patch,
    ci_gate::validate_ci_gate_config,
    queue::{validate_queue_aging_thresholds, validate_queue_overrides},
    validate_agent_binary_paths,
};
use crate::constants::runner::{MAX_PHASES, MIN_ITERATIONS, MIN_PARALLEL_WORKERS, MIN_PHASES};
use crate::contracts::Config;
use anyhow::{Result, bail};
use std::path::Component;

pub fn validate_config(cfg: &Config) -> Result<()> {
    if cfg.version != 1 {
        bail!(
            "Unsupported config version: {}. Ralph requires version 1. Update the 'version' field in your config file.",
            cfg.version
        );
    }

    validate_queue_overrides(&cfg.queue)?;
    validate_queue_aging_thresholds(&cfg.queue.aging_thresholds)?;

    if let Some(phases) = cfg.agent.phases
        && !(MIN_PHASES..=MAX_PHASES).contains(&phases)
    {
        bail!(
            "Invalid agent.phases: {}. Supported values are {}, {}, or {}. Update .ralph/config.jsonc or CLI flags.",
            phases,
            MIN_PHASES,
            MIN_PHASES + 1,
            MAX_PHASES
        );
    }

    if let Some(iterations) = cfg.agent.iterations
        && iterations < MIN_ITERATIONS
    {
        bail!(
            "Invalid agent.iterations: {}. Iterations must be at least {}. Update .ralph/config.jsonc.",
            iterations,
            MIN_ITERATIONS
        );
    }

    if let Some(workers) = cfg.parallel.workers
        && workers < MIN_PARALLEL_WORKERS
    {
        bail!(
            "Invalid parallel.workers: {}. Parallel workers must be >= {}. Update .ralph/config.jsonc or CLI flags.",
            workers,
            MIN_PARALLEL_WORKERS
        );
    }

    if let Some(root) = &cfg.parallel.workspace_root {
        if root.as_os_str().is_empty() {
            bail!(
                "Empty parallel.workspace_root: path is required if specified. Set a valid path or remove the field."
            );
        }
        if root
            .components()
            .any(|component| matches!(component, Component::ParentDir))
        {
            bail!(
                "Invalid parallel.workspace_root: path must not contain '..' components (got {}). Use a normalized path.",
                root.display()
            );
        }
    }

    if let Some(timeout) = cfg.agent.session_timeout_hours
        && timeout == 0
    {
        bail!(
            "Invalid agent.session_timeout_hours: {}. Session timeout must be greater than 0. Update .ralph/config.jsonc.",
            timeout
        );
    }

    validate_agent_binary_paths(&cfg.agent, "agent")?;
    validate_ci_gate_config(cfg.agent.ci_gate.as_ref(), "agent")?;

    if let Some(profiles) = cfg.profiles.as_ref() {
        for (name, patch) in profiles {
            validate_agent_patch(patch, &format!("profiles.{name}"))?;
        }
    }

    Ok(())
}
