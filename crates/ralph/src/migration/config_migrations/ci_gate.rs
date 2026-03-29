//! Purpose: Rewrite legacy CI gate config into the structured `agent.ci_gate` shape.
//!
//! Responsibilities:
//! - Detect and rewrite legacy `ci_gate_command` / `ci_gate_enabled` fields.
//! - Materialize the new argv-based `agent.ci_gate` payload.
//! - Persist migrated config files atomically.
//!
//! Scope:
//! - CI gate migration only; generic key rename/remove and legacy contract upgrade
//!   live in sibling modules.
//!
//! Usage:
//! - Used by `MigrationType::ConfigCiGateRewrite`.
//!
//! Invariants/Assumptions:
//! - Disabled legacy CI gate maps to `{ "enabled": false }`.
//! - Enabled legacy shell strings are migrated to argv-only execution via `shlex::split`.
//! - Missing/empty legacy commands default to `make ci`.

use anyhow::{Context, Result};
use serde_json::Value;
use std::{fs, path::Path};

use super::super::MigrationContext;

/// Rewrite legacy CI gate keys into structured `agent.ci_gate` config.
pub fn apply_ci_gate_rewrite(ctx: &MigrationContext) -> Result<()> {
    rewrite_ci_gate_in_file(&ctx.project_config_path)?;

    if let Some(global_path) = &ctx.global_config_path {
        rewrite_ci_gate_in_file(global_path)?;
    }

    Ok(())
}

pub(super) fn rewrite_ci_gate_in_file(path: &Path) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }

    let raw =
        fs::read_to_string(path).with_context(|| format!("read config file {}", path.display()))?;
    let mut value: Value = jsonc_parser::parse_to_serde_value::<Value>(&raw, &Default::default())?;

    let Some(root) = value.as_object_mut() else {
        return Ok(());
    };
    let Some(agent) = root.get_mut("agent").and_then(Value::as_object_mut) else {
        return Ok(());
    };

    let legacy_command = agent.remove("ci_gate_command");
    let legacy_enabled = agent.remove("ci_gate_enabled");
    if legacy_command.is_none() && legacy_enabled.is_none() {
        return Ok(());
    }

    let enabled = legacy_enabled
        .and_then(|value| value.as_bool())
        .unwrap_or(true);
    let ci_gate = build_ci_gate_value(legacy_command.as_ref(), enabled)?;
    agent.insert("ci_gate".to_string(), ci_gate);

    let rendered = serde_json::to_string_pretty(&value).context("serialize migrated config")?;
    crate::fsutil::write_atomic(path, rendered.as_bytes())
        .with_context(|| format!("write migrated config {}", path.display()))?;
    Ok(())
}

pub(super) fn build_ci_gate_value(legacy_command: Option<&Value>, enabled: bool) -> Result<Value> {
    if !enabled {
        return Ok(serde_json::json!({ "enabled": false }));
    }

    let command = legacy_command
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("make ci");

    let argv = shlex::split(command).ok_or_else(|| {
        anyhow::anyhow!(
            "could not migrate legacy CI gate command to argv-only execution: {}",
            command
        )
    })?;
    if argv.is_empty() {
        return Ok(serde_json::json!({ "enabled": false }));
    }

    Ok(serde_json::json!({
        "enabled": true,
        "argv": argv,
    }))
}
