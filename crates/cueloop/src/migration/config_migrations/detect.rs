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
use std::{
    fs,
    path::{Path, PathBuf},
};

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

/// Resolved project-layer config files that currently exist (`config.jsonc` then `config.json`).
///
/// [`MigrationContext::project_config_path`](crate::migration::MigrationContext::project_config_path)
/// always prefers the JSONC path for stable layering, but migrations must inspect legacy `.json`
/// when only that file exists.
pub(crate) fn project_migration_config_disk_paths(repo_root: &Path) -> Vec<PathBuf> {
    let cueloop = repo_root.join(".cueloop");
    let jsonc = cueloop.join("config.jsonc");
    let json = cueloop.join("config.json");
    let mut paths = Vec::new();
    if jsonc.exists() {
        paths.push(jsonc);
    }
    if json.exists() {
        paths.push(json);
    }
    paths
}

/// Check if either project or global config contains the removed Cursor binary override.
///
/// This intentionally scans profile patches as well as `agent` because profiles are
/// deserialized as `AgentConfig` and reject unknown fields after the cutover.
pub fn config_has_legacy_cursor_bin(ctx: &MigrationContext) -> bool {
    for path in project_migration_config_disk_paths(&ctx.repo_root) {
        if let Ok(true) = config_file_has_legacy_cursor_bin(path.as_path()) {
            return true;
        }
    }

    if let Some(global_path) = &ctx.global_config_path
        && let Ok(true) = config_file_has_legacy_cursor_bin(global_path)
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

    let value =
        match jsonc_parser::parse_to_serde_value::<serde_json::Value>(&raw, &Default::default()) {
            Ok(v) => v,
            Err(_) => return Ok(false),
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

fn config_file_has_legacy_cursor_bin(path: &Path) -> Result<bool> {
    if !path.exists() {
        return Ok(false);
    }

    let raw =
        fs::read_to_string(path).with_context(|| format!("read config file {}", path.display()))?;
    let value =
        match jsonc_parser::parse_to_serde_value::<serde_json::Value>(&raw, &Default::default()) {
            Ok(v) => v,
            Err(_) => return Ok(false),
        };

    if value
        .get("agent")
        .and_then(serde_json::Value::as_object)
        .is_some_and(|agent| agent.contains_key("cursor_bin"))
    {
        return Ok(true);
    }

    let has_profile_cursor_bin = value
        .get("profiles")
        .and_then(serde_json::Value::as_object)
        .is_some_and(|profiles| {
            profiles
                .values()
                .filter_map(serde_json::Value::as_object)
                .any(|profile| profile.contains_key("cursor_bin"))
        });

    Ok(has_profile_cursor_bin)
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
