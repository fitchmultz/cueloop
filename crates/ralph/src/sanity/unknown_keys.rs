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
use crate::migration::config_migrations::{remove_key_in_file, rename_key_in_file};
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
) -> Result<UnknownKeyAction> {
    if auto_fix {
        return Ok(UnknownKeyAction::Remove);
    }

    if !non_interactive && can_prompt() {
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
        UnknownKeyAction::Remove => match remove_key_in_file(path, key) {
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
        UnknownKeyAction::Rename(new_key) => match rename_key_in_file(path, key, &new_key) {
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
    Ok(parse_unknown_key_action(&input))
}

fn parse_unknown_key_action(input: &str) -> UnknownKeyAction {
    let trimmed = input.trim();
    let command = trimmed.to_ascii_lowercase();

    if trimmed.is_empty() || command == "k" || command == "keep" {
        UnknownKeyAction::Keep
    } else if command == "r" || command == "remove" {
        UnknownKeyAction::Remove
    } else {
        UnknownKeyAction::Rename(trimmed.to_string())
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

    let value =
        match jsonc_parser::parse_to_serde_value::<serde_json::Value>(&raw, &Default::default()) {
            Ok(v) => v,
            Err(_) => return Ok(Vec::new()),
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
    fn parse_unknown_key_action_preserves_rename_case() {
        assert!(matches!(
            parse_unknown_key_action(" Agent.Runner_CLI  "),
            UnknownKeyAction::Rename(rename) if rename == "Agent.Runner_CLI"
        ));
        assert!(matches!(
            parse_unknown_key_action("KEEP"),
            UnknownKeyAction::Keep
        ));
        assert!(matches!(
            parse_unknown_key_action("Remove"),
            UnknownKeyAction::Remove
        ));
    }

    #[test]
    fn is_known_parent_key_detects_parents() {
        let keys = get_known_config_keys();
        assert!(is_known_parent_key("agent", &keys));
        assert!(is_known_parent_key("queue", &keys));
        assert!(!is_known_parent_key("unknown", &keys));
    }

    #[test]
    fn remove_key_from_config_file_leaves_empty_file_unchanged() {
        let dir = tempfile::TempDir::new().unwrap();
        let config_path = dir.path().join("config.json");
        std::fs::write(&config_path, "").unwrap();

        remove_key_in_file(&config_path, "agent.runner").unwrap();

        assert_eq!(std::fs::read_to_string(&config_path).unwrap(), "");
    }

    #[test]
    fn rename_key_in_config_file_leaves_empty_file_unchanged() {
        let dir = tempfile::TempDir::new().unwrap();
        let config_path = dir.path().join("config.json");
        std::fs::write(&config_path, "").unwrap();

        rename_key_in_file(&config_path, "agent.runner", "agent.runner_cli").unwrap();

        assert_eq!(std::fs::read_to_string(&config_path).unwrap(), "");
    }

    #[test]
    fn rename_key_in_config_file_rejects_parent_path_changes() {
        let dir = tempfile::TempDir::new().unwrap();
        let config_path = dir.path().join("config.json");
        std::fs::write(&config_path, r#"{ "parallel": { "worktree_root": "x" } }"#).unwrap();

        let err = rename_key_in_file(
            &config_path,
            "parallel.worktree_root",
            "agent.workspace_root",
        )
        .unwrap_err();

        assert!(err.to_string().contains("must keep the same parent path"));
        let content = std::fs::read_to_string(&config_path).unwrap();
        assert!(content.contains("worktree_root"));
        assert!(!content.contains("workspace_root"));
    }

    #[test]
    fn check_unknown_keys_auto_fix_removes_unknown_key() {
        let dir = tempfile::TempDir::new().unwrap();
        let config_path = dir.path().join("config.json");
        std::fs::write(
            &config_path,
            r#"{
                "version": 2,
                "unknown_key": "value"
            }"#,
        )
        .unwrap();

        let resolved = Resolved {
            config: crate::contracts::Config::default(),
            repo_root: dir.path().to_path_buf(),
            queue_path: dir.path().join("queue.json"),
            done_path: dir.path().join("done.json"),
            id_prefix: "RQ".to_string(),
            id_width: 4,
            global_config_path: None,
            project_config_path: Some(config_path.clone()),
        };

        let actions = check_unknown_keys(&resolved, true, true, || false).unwrap();
        assert!(
            actions
                .iter()
                .any(|action| action.contains("Removed unknown key 'unknown_key'"))
        );

        let content = std::fs::read_to_string(&config_path).unwrap();
        assert!(!content.contains("unknown_key"));
        assert!(content.contains("\"version\": 2"));
    }
}
