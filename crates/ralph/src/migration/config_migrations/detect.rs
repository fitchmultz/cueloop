//! Purpose: Detect config migration applicability and expose resolved config lookups.
//!
//! Responsibilities:
//! - Check whether project/global config files contain a specific dot-path key.
//! - Read JSONC config files safely for migration applicability checks.
//! - Expose resolved-config value lookup for migration logic.
//!
//! Scope:
//! - Detection and lookup only; mutation behavior lives in sibling modules.
//!
//! Usage:
//! - Used by migration applicability checks and config migration execution.
//!
//! Invariants/Assumptions:
//! - Missing or unparsable config files are treated as "key not present" for applicability.
//! - Dot-path navigation only traverses JSON objects.

use anyhow::{Context, Result};
use std::{fs, path::Path};

use super::super::MigrationContext;

/// Check if a config key exists in either project or global config.
/// Supports dot notation for nested keys (e.g., "agent.runner_cli").
pub fn config_has_key(ctx: &MigrationContext, key: &str) -> bool {
    if let Ok(true) = config_file_has_key(&ctx.project_config_path, key) {
        return true;
    }

    if let Some(global_path) = &ctx.global_config_path
        && let Ok(true) = config_file_has_key(global_path, key)
    {
        return true;
    }

    false
}

/// Check if a specific config file contains a key.
pub(super) fn config_file_has_key(path: &Path, key: &str) -> Result<bool> {
    if !path.exists() {
        return Ok(false);
    }

    let raw =
        fs::read_to_string(path).with_context(|| format!("read config file {}", path.display()))?;

    let value = match jsonc_parser::parse_to_serde_value(&raw, &Default::default()) {
        Ok(Some(v)) => v,
        _ => return Ok(false),
    };

    let parts: Vec<&str> = key.split('.').collect();
    let mut current = &value;

    for part in &parts {
        match current {
            serde_json::Value::Object(map) => match map.get(*part) {
                Some(v) => current = v,
                None => return Ok(false),
            },
            _ => return Ok(false),
        }
    }

    Ok(true)
}

/// Get the value of a config key from the context's resolved config.
/// Returns None if the key doesn't exist.
pub fn get_config_value(ctx: &MigrationContext, key: &str) -> Option<serde_json::Value> {
    let parts: Vec<&str> = key.split('.').collect();

    let config_json = match serde_json::to_value(&ctx.resolved_config) {
        Ok(v) => v,
        Err(_) => return None,
    };

    let mut current = &config_json;
    for part in &parts {
        match current {
            serde_json::Value::Object(map) => {
                current = map.get(*part)?;
            }
            _ => return None,
        }
    }

    Some(current.clone())
}
