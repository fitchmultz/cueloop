//! Purpose: Upgrade legacy config contract markers to the current schema.
//!
//! Responsibilities:
//! - Detect pre-0.3 config files that still require contract upgrade.
//! - Rewrite version markers from 1 to 2.
//! - Map `agent.git_commit_push_enabled` to `agent.git_publish_mode`.
//! - Preserve an existing `git_publish_mode` if already present.
//!
//! Scope:
//! - Legacy contract upgrade only; generic key rename/remove and CI gate rewrite
//!   live in sibling modules.
//!
//! Usage:
//! - Used by `MigrationType::ConfigLegacyContractUpgrade`.
//!
//! Invariants/Assumptions:
//! - Version 1 or `git_commit_push_enabled` means upgrade is needed.
//! - `true` maps to `commit_and_push`; `false` maps to `off`.
//! - Existing `git_publish_mode` wins if both keys are present.

use anyhow::{Context, Result};
use serde_json::Value;
use std::{fs, path::Path};

use super::super::MigrationContext;

/// Check whether either config file still uses the pre-0.3 contract.
pub fn config_needs_legacy_contract_upgrade(ctx: &MigrationContext) -> bool {
    config_file_needs_legacy_contract_upgrade(&ctx.project_config_path).unwrap_or(false)
        || ctx
            .global_config_path
            .as_ref()
            .and_then(|path| config_file_needs_legacy_contract_upgrade(path).ok())
            .unwrap_or(false)
}

/// Upgrade legacy config contract markers in project/global config files.
pub fn apply_legacy_contract_upgrade(ctx: &MigrationContext) -> Result<()> {
    upgrade_legacy_contract_in_file(&ctx.project_config_path)?;

    if let Some(global_path) = &ctx.global_config_path {
        upgrade_legacy_contract_in_file(global_path)?;
    }

    Ok(())
}

pub(super) fn config_file_needs_legacy_contract_upgrade(path: &Path) -> Result<bool> {
    if !path.exists() {
        return Ok(false);
    }

    let raw =
        fs::read_to_string(path).with_context(|| format!("read config file {}", path.display()))?;
    let value = match jsonc_parser::parse_to_serde_value(&raw, &Default::default()) {
        Ok(Some(v)) => v,
        _ => return Ok(false),
    };

    let has_legacy_version = value.get("version").and_then(Value::as_u64) == Some(1);
    let has_legacy_publish_flag = value
        .get("agent")
        .and_then(Value::as_object)
        .is_some_and(|agent| agent.contains_key("git_commit_push_enabled"));

    Ok(has_legacy_version || has_legacy_publish_flag)
}

pub(super) fn upgrade_legacy_contract_in_file(path: &Path) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }

    let raw =
        fs::read_to_string(path).with_context(|| format!("read config file {}", path.display()))?;
    let mut value = match jsonc_parser::parse_to_serde_value(&raw, &Default::default())? {
        Some(value) => value,
        None => return Ok(()),
    };

    let Some(root) = value.as_object_mut() else {
        return Ok(());
    };

    let agent_has_legacy_flag = root
        .get("agent")
        .and_then(Value::as_object)
        .is_some_and(|agent| agent.contains_key("git_commit_push_enabled"));
    let version_needs_upgrade = root.get("version").and_then(Value::as_u64) == Some(1);

    if !agent_has_legacy_flag && !version_needs_upgrade {
        return Ok(());
    }

    if version_needs_upgrade || agent_has_legacy_flag {
        root.insert("version".to_string(), Value::from(2));
    }

    if let Some(agent) = root.get_mut("agent").and_then(Value::as_object_mut) {
        let git_publish_mode_exists = agent.contains_key("git_publish_mode");
        let legacy_publish_value = agent
            .remove("git_commit_push_enabled")
            .and_then(|value| value.as_bool());

        if let Some(legacy_publish_value) = legacy_publish_value
            && !git_publish_mode_exists
        {
            let mode = if legacy_publish_value {
                "commit_and_push"
            } else {
                "off"
            };
            agent.insert(
                "git_publish_mode".to_string(),
                Value::String(mode.to_string()),
            );
        }
    }

    let rendered = serde_json::to_string_pretty(&value).context("serialize migrated config")?;
    crate::fsutil::write_atomic(path, rendered.as_bytes())
        .with_context(|| format!("write migrated config {}", path.display()))?;

    log::info!("Upgraded legacy config contract in {}", path.display());
    Ok(())
}
