//! Unknown config key detection and handling.
//!
//! Responsibilities:
//! - Detect unknown keys in config files (project and global)
//! - Prompt user for action (remove/keep/rename) or auto-remove
//! - Manipulate JSON config files to remove or rename keys
//! - Extract known keys from schema for comparison
//!
//! Not handled here:
//! - README updates (see readme.rs)
//! - Config migrations (see migrations.rs)
//! - Schema definition (see contracts/)
//!
//! Invariants:
//! - Unknown keys are detected by comparing against schemars-generated schema
//! - Auto-fix removes unknown keys without prompting
//! - Non-interactive mode without auto-fix keeps keys with warning

use crate::config::Resolved;
use anyhow::{Context, Result};
use std::collections::HashSet;
use std::io::{self, Write};

/// Action to take for an unknown key.
#[derive(Debug, Clone)]
enum UnknownKeyAction {
    /// Remove the unknown key.
    Remove,
    /// Keep the unknown key as-is.
    Keep,
    /// Rename the key to a new name.
    Rename(String),
}

/// Check for unknown config keys in config files.
///
/// Returns a list of actions taken.
pub(crate) fn check_unknown_keys(
    resolved: &Resolved,
    auto_fix: bool,
    non_interactive: bool,
    can_prompt: impl Fn() -> bool,
) -> Result<Vec<String>> {
    let mut actions = Vec::new();
    let known_keys = get_known_config_keys();

    // Check project config
    if let Some(ref project_path) = resolved.project_config_path
        && project_path.exists()
    {
        match check_config_file_unknown_keys(project_path, &known_keys) {
            Ok(unknown_keys) => {
                for key in unknown_keys {
                    let action = determine_key_action(
                        &key,
                        "project config",
                        auto_fix,
                        non_interactive,
                        &can_prompt,
                        project_path,
                    )?;
                    actions.extend(apply_key_action(
                        project_path,
                        &key,
                        action,
                        "project config",
                    )?);
                }
            }
            Err(e) => {
                log::warn!("Failed to check project config for unknown keys: {}", e);
            }
        }
    }

    // Check global config
    if let Some(ref global_path) = resolved.global_config_path
        && global_path.exists()
    {
        match check_config_file_unknown_keys(global_path, &known_keys) {
            Ok(unknown_keys) => {
                for key in unknown_keys {
                    let action = determine_key_action(
                        &key,
                        "global config",
                        auto_fix,
                        non_interactive,
                        &can_prompt,
                        global_path,
                    )?;
                    actions.extend(apply_key_action(
                        global_path,
                        &key,
                        action,
                        "global config",
                    )?);
                }
            }
            Err(e) => {
                log::warn!("Failed to check global config for unknown keys: {}", e);
            }
        }
    }

    Ok(actions)
}

fn determine_key_action(
    key: &str,
    config_file: &str,
    auto_fix: bool,
    non_interactive: bool,
    can_prompt: &impl Fn() -> bool,
    path: &std::path::Path,
) -> Result<UnknownKeyAction> {
    if auto_fix {
        match remove_key_from_config_file(path, key) {
            Ok(()) => Ok(UnknownKeyAction::Remove),
            Err(e) => {
                log::warn!("Failed to remove key '{}': {}", key, e);
                Ok(UnknownKeyAction::Keep)
            }
        }
    } else if !non_interactive && can_prompt() {
        prompt_unknown_key(key, config_file)
    } else {
        log::warn!(
            "Unknown config key '{}' in {} (use --auto-fix to remove)",
            key,
            config_file
        );
        Ok(UnknownKeyAction::Keep)
    }
}

fn apply_key_action(
    path: &std::path::Path,
    key: &str,
    action: UnknownKeyAction,
    config_file: &str,
) -> Result<Vec<String>> {
    let mut actions = Vec::new();
    match action {
        UnknownKeyAction::Remove => match remove_key_from_config_file(path, key) {
            Ok(()) => {
                actions.push(format!(
                    "Removed unknown key '{}' from {}",
                    key, config_file
                ));
            }
            Err(e) => {
                log::warn!("Failed to remove key '{}': {}", key, e);
            }
        },
        UnknownKeyAction::Keep => {
            log::info!("Kept unknown key '{}' in {}", key, config_file);
        }
        UnknownKeyAction::Rename(new_key) => match rename_key_in_config_file(path, key, &new_key) {
            Ok(()) => {
                actions.push(format!(
                    "Renamed key '{}' to '{}' in {}",
                    key, new_key, config_file
                ));
            }
            Err(e) => {
                log::warn!("Failed to rename key '{}': {}", key, e);
            }
        },
    }
    Ok(actions)
}

/// Prompt user for action on an unknown key.
fn prompt_unknown_key(key: &str, config_file: &str) -> Result<UnknownKeyAction> {
    print!(
        "Unknown config key '{}' in {}. [r]emove, [k]eep, or rename to: ",
        key, config_file
    );
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let trimmed = input.trim().to_lowercase();

    if trimmed.is_empty() || trimmed == "k" || trimmed == "keep" {
        Ok(UnknownKeyAction::Keep)
    } else if trimmed == "r" || trimmed == "remove" {
        Ok(UnknownKeyAction::Remove)
    } else if !trimmed.is_empty() {
        Ok(UnknownKeyAction::Rename(trimmed))
    } else {
        Ok(UnknownKeyAction::Keep)
    }
}

/// Get the set of known config keys from the Config schema.
fn get_known_config_keys() -> HashSet<String> {
    use serde_json::Value;

    let schema = schemars::schema_for!(crate::contracts::Config);
    let mut keys = HashSet::new();
    let Some(root) = schema.as_object() else {
        return keys;
    };

    if let Some(properties) = root.get("properties").and_then(Value::as_object) {
        let definitions = root
            .get("$defs")
            .and_then(Value::as_object)
            .or_else(|| root.get("definitions").and_then(Value::as_object));
        for (key, subschema) in properties {
            keys.insert(key.clone());
            extract_keys_from_schema(subschema, key, &mut keys, definitions);
        }
    }

    keys
}

/// Recursively extract dot-notation keys from a schema.
fn extract_keys_from_schema(
    schema: &serde_json::Value,
    prefix: &str,
    keys: &mut HashSet<String>,
    definitions: Option<&serde_json::Map<String, serde_json::Value>>,
) {
    use serde_json::Value;

    let Some(obj) = schema.as_object() else {
        return;
    };

    if let Some(ref_path) = obj.get("$ref").and_then(Value::as_str) {
        if let Some(definitions) = definitions
            && let Some(def_name) = ref_path
                .strip_prefix("#/$defs/")
                .or_else(|| ref_path.strip_prefix("#/definitions/"))
            && let Some(def_schema) = definitions.get(def_name)
        {
            extract_keys_from_schema(def_schema, prefix, keys, Some(definitions));
        }
        return;
    }

    if let Some(properties) = obj.get("properties").and_then(Value::as_object) {
        for (key, subschema) in properties {
            let full_key = format!("{}.{}", prefix, key);
            keys.insert(full_key.clone());
            extract_keys_from_schema(subschema, &full_key, keys, definitions);
        }
    }

    for keyword in ["allOf", "anyOf", "oneOf"] {
        if let Some(subschemas) = obj.get(keyword).and_then(Value::as_array) {
            for sub in subschemas {
                extract_keys_from_schema(sub, prefix, keys, definitions);
            }
        }
    }
}

/// Check a config file for unknown keys.
fn check_config_file_unknown_keys(
    path: &std::path::Path,
    known_keys: &HashSet<String>,
) -> Result<Vec<String>> {
    use std::fs;

    let raw = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;

    let value = match jsonc_parser::parse_to_serde_value(&raw, &Default::default()) {
        Ok(Some(v)) => v,
        _ => return Ok(Vec::new()),
    };

    let mut unknown_keys = Vec::new();
    collect_unknown_keys(&value, known_keys, "", &mut unknown_keys);

    Ok(unknown_keys)
}

/// Recursively collect unknown keys from a JSON value.
fn collect_unknown_keys(
    value: &serde_json::Value,
    known_keys: &HashSet<String>,
    prefix: &str,
    unknown: &mut Vec<String>,
) {
    if let serde_json::Value::Object(map) = value {
        for (key, child) in map {
            let full_key = if prefix.is_empty() {
                key.clone()
            } else {
                format!("{}.{}", prefix, key)
            };

            if !known_keys.contains(&full_key)
                && !is_known_parent_key(&full_key, known_keys)
                && (!child.is_object() || child.as_object().map(|m| m.is_empty()).unwrap_or(false))
            {
                unknown.push(full_key.clone());
            }

            collect_unknown_keys(child, known_keys, &full_key, unknown);
        }
    }
}

/// Check if a key is a known parent key.
fn is_known_parent_key(key: &str, known_keys: &HashSet<String>) -> bool {
    for known in known_keys {
        if known.starts_with(&format!("{}.", key)) {
            return true;
        }
    }
    false
}

/// Remove a key from a config file.
fn remove_key_from_config_file(path: &std::path::Path, key: &str) -> Result<()> {
    use std::fs;

    let raw = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;

    let mut value: serde_json::Value =
        jsonc_parser::parse_to_serde_value(&raw, &Default::default())
            .context("parse config")?
            .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));

    remove_key_from_value(&mut value, key);

    let modified = serde_json::to_string_pretty(&value).context("serialize config")?;
    crate::fsutil::write_atomic(path, modified.as_bytes())
        .with_context(|| format!("write {}", path.display()))?;

    log::info!("Removed key '{}' from {}", key, path.display());
    Ok(())
}

/// Remove a key from a JSON value using dot notation.
fn remove_key_from_value(value: &mut serde_json::Value, key: &str) {
    let parts: Vec<&str> = key.split('.').collect();
    if parts.is_empty() {
        return;
    }

    if parts.len() == 1 {
        if let serde_json::Value::Object(map) = value {
            map.remove(parts[0]);
        }
    } else {
        let parent_key = parts[..parts.len() - 1].join(".");
        let child_key = parts[parts.len() - 1];

        if let Some(serde_json::Value::Object(map)) = get_nested_value_mut(value, &parent_key) {
            map.remove(child_key);
        }
    }
}

/// Get a mutable reference to a nested value.
fn get_nested_value_mut<'a>(
    value: &'a mut serde_json::Value,
    key: &str,
) -> Option<&'a mut serde_json::Value> {
    let parts: Vec<&str> = key.split('.').collect();
    let mut current = value;

    for part in parts {
        match current {
            serde_json::Value::Object(map) => {
                current = map.get_mut(part)?;
            }
            _ => return None,
        }
    }

    Some(current)
}

/// Rename a key in a config file.
fn rename_key_in_config_file(path: &std::path::Path, old_key: &str, new_key: &str) -> Result<()> {
    use std::fs;

    let raw = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;

    let mut value: serde_json::Value =
        jsonc_parser::parse_to_serde_value(&raw, &Default::default())
            .context("parse config")?
            .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));

    rename_key_in_value(&mut value, old_key, new_key);

    let modified = serde_json::to_string_pretty(&value).context("serialize config")?;
    crate::fsutil::write_atomic(path, modified.as_bytes())
        .with_context(|| format!("write {}", path.display()))?;

    log::info!(
        "Renamed key '{}' to '{}' in {}",
        old_key,
        new_key,
        path.display()
    );
    Ok(())
}

/// Rename a key in a JSON value using dot notation.
fn rename_key_in_value(value: &mut serde_json::Value, old_key: &str, new_key: &str) {
    let parts: Vec<&str> = old_key.split('.').collect();
    if parts.is_empty() {
        return;
    }

    if parts.len() == 1 {
        if let serde_json::Value::Object(map) = value
            && let Some(v) = map.remove(parts[0])
        {
            map.insert(new_key.to_string(), v);
        }
    } else {
        let parent_key = parts[..parts.len() - 1].join(".");
        let child_key = parts[parts.len() - 1];
        let new_key_name = new_key.split('.').next_back().unwrap_or(new_key);

        if let Some(serde_json::Value::Object(map)) = get_nested_value_mut(value, &parent_key)
            && let Some(v) = map.remove(child_key)
        {
            map.insert(new_key_name.to_string(), v);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn get_known_config_keys_includes_top_level() {
        let keys = get_known_config_keys();
        assert!(keys.contains("version"));
        assert!(keys.contains("project_type"));
        assert!(keys.contains("agent"));
        assert!(keys.contains("queue"));
        assert!(keys.contains("parallel"));
        assert!(keys.contains("plugins"));
        assert!(keys.contains("profiles"));
    }

    #[test]
    fn get_known_config_keys_includes_agent_keys() {
        let keys = get_known_config_keys();
        assert!(keys.contains("agent.runner"));
        assert!(keys.contains("agent.model"));
        assert!(keys.contains("agent.phases"));
        assert!(keys.contains("agent.codex_bin"));
        assert!(keys.contains("agent.followup_reasoning_effort"));
        assert!(keys.contains("agent.claude_permission_mode"));
        assert!(keys.contains("agent.runner_cli"));
        assert!(keys.contains("agent.notification"));
        assert!(keys.contains("agent.notification.enabled"));
        assert!(keys.contains("agent.notification.notify_on_complete"));
    }

    #[test]
    fn get_known_config_keys_extracts_runner_cli_keys() {
        let keys = get_known_config_keys();
        assert!(keys.contains("agent.runner_cli"));
        assert!(keys.contains("agent.runner_cli.defaults"));
        assert!(keys.contains("agent.runner_cli.runners"));
    }

    #[test]
    fn check_config_file_unknown_keys_detects_unknown() {
        let dir = TempDir::new().unwrap();
        let config_path = dir.path().join("config.json");

        std::fs::write(
            &config_path,
            r#"{
                "version": 1,
                "unknown_key": "value",
                "agent": {
                    "runner": "claude",
                    "unknown_agent_key": 123
                }
            }"#,
        )
        .unwrap();

        let known_keys = get_known_config_keys();
        let unknown = check_config_file_unknown_keys(&config_path, &known_keys).unwrap();

        assert!(unknown.contains(&"unknown_key".to_string()));
        assert!(unknown.contains(&"agent.unknown_agent_key".to_string()));
        assert!(!unknown.contains(&"version".to_string()));
        assert!(!unknown.contains(&"agent.runner".to_string()));
    }

    #[test]
    fn remove_key_from_value_works() {
        let mut value: serde_json::Value = serde_json::json!({
            "version": 1,
            "agent": {
                "runner": "claude",
                "model": "sonnet"
            }
        });

        remove_key_from_value(&mut value, "version");
        assert!(value.get("version").is_none());
        assert!(value.get("agent").is_some());

        remove_key_from_value(&mut value, "agent.runner");
        let agent = value.get("agent").unwrap();
        assert!(agent.get("runner").is_none());
        assert!(agent.get("model").is_some());
    }

    #[test]
    fn rename_key_in_value_works() {
        let mut value: serde_json::Value = serde_json::json!({
            "version": 1,
            "agent": {
                "runner": "claude"
            }
        });

        rename_key_in_value(&mut value, "agent.runner", "agent.runner_cli");
        let agent = value.get("agent").unwrap();
        assert!(agent.get("runner").is_none());
        assert_eq!(agent.get("runner_cli").unwrap(), "claude");
    }

    #[test]
    fn is_known_parent_key_detects_parents() {
        let keys = get_known_config_keys();
        assert!(is_known_parent_key("agent", &keys));
        assert!(is_known_parent_key("queue", &keys));
        assert!(!is_known_parent_key("unknown", &keys));
    }
}
