//! Purpose: Apply config key rename and removal migrations.
//!
//! Responsibilities:
//! - Rename config keys in project/global config files.
//! - Remove deprecated config keys from parsed JSON values.
//! - Preserve JSONC comments for text-based rename flows.
//! - Scope leaf-key renames to the intended parent object.
//!
//! Scope:
//! - Key rename/remove behavior only; detection, CI gate rewrite, and legacy
//!   contract upgrade live in sibling modules.
//!
//! Usage:
//! - Used by `MigrationType::ConfigKeyRename` and `MigrationType::ConfigKeyRemove`.
//!
//! Invariants/Assumptions:
//! - Dot-path keys use object navigation only.
//! - Scoped rename behavior must remain exactly compatible with the prior module.
//! - Removal rewrites parsed JSON and may normalize formatting/comments.

use anyhow::{Context, Result};
use serde_json::Value;
use std::{fs, path::Path};

use super::super::MigrationContext;
use super::detect::config_file_has_key;

/// Apply a key rename to both project and global configs.
/// Uses text-based replacement to preserve comments.
pub fn apply_key_rename(ctx: &MigrationContext, old_key: &str, new_key: &str) -> Result<()> {
    if config_file_has_key(&ctx.project_config_path, old_key)? {
        rename_key_in_file(&ctx.project_config_path, old_key, new_key)
            .with_context(|| "rename key in project config".to_string())?;
    }

    if let Some(global_path) = &ctx.global_config_path
        && config_file_has_key(global_path, old_key)?
    {
        rename_key_in_file(global_path, old_key, new_key)
            .with_context(|| "rename key in global config".to_string())?;
    }

    Ok(())
}

/// Apply a key removal to both project and global configs.
pub fn apply_key_remove(ctx: &MigrationContext, key: &str) -> Result<()> {
    if config_file_has_key(&ctx.project_config_path, key)? {
        remove_key_in_file(&ctx.project_config_path, key)
            .with_context(|| "remove key in project config".to_string())?;
    }

    if let Some(global_path) = &ctx.global_config_path
        && config_file_has_key(global_path, key)?
    {
        remove_key_in_file(global_path, key)
            .with_context(|| "remove key in global config".to_string())?;
    }

    Ok(())
}

/// Rename a key in a specific config file while preserving comments.
/// Uses scoped text-based replacement to only rename within the specified parent object.
/// For "parallel.worktree_root", only renames "worktree_root" inside "parallel" objects.
pub(super) fn rename_key_in_file(path: &Path, old_key: &str, new_key: &str) -> Result<()> {
    let raw =
        fs::read_to_string(path).with_context(|| format!("read config file {}", path.display()))?;

    let old_parts: Vec<&str> = old_key.split('.').collect();
    let new_parts: Vec<&str> = new_key.split('.').collect();

    if old_parts.is_empty() || new_parts.is_empty() {
        return Err(anyhow::anyhow!("Empty key"));
    }

    let old_leaf = old_parts[old_parts.len() - 1];
    let new_leaf = new_parts[new_parts.len() - 1];

    let parent_path = if old_parts.len() > 1 {
        old_parts[..old_parts.len() - 1].to_vec()
    } else {
        Vec::new()
    };

    let modified = if parent_path.is_empty() {
        rename_key_in_text(&raw, old_leaf, new_leaf)
    } else {
        rename_key_in_text_scoped(&raw, &parent_path, old_leaf, new_leaf)
    }
    .with_context(|| format!("rename key {} to {} in text", old_key, new_key))?;

    crate::fsutil::write_atomic(path, modified.as_bytes())
        .with_context(|| format!("write modified config to {}", path.display()))?;

    log::info!(
        "Renamed config key '{}' to '{}' in {}",
        old_key,
        new_key,
        path.display()
    );

    Ok(())
}

/// Remove a key from a specific config file.
pub(super) fn remove_key_in_file(path: &Path, key: &str) -> Result<()> {
    let raw =
        fs::read_to_string(path).with_context(|| format!("read config file {}", path.display()))?;

    let mut value = jsonc_parser::parse_to_serde_value(&raw, &Default::default())?
        .ok_or_else(|| anyhow::anyhow!("parse config file {}", path.display()))?;

    remove_key_from_value(&mut value, key);

    let modified = serde_json::to_string_pretty(&value).context("serialize config")?;
    crate::fsutil::write_atomic(path, modified.as_bytes())
        .with_context(|| format!("write modified config to {}", path.display()))?;

    log::info!("Removed config key '{}' in {}", key, path.display());
    Ok(())
}

/// Rename a key in JSONC text while preserving comments and formatting.
/// Uses regex-like pattern matching to find and replace key names.
pub(super) fn rename_key_in_text(raw: &str, old_key: &str, new_key: &str) -> Result<String> {
    let mut result = raw.to_string();

    let double_quoted = format!(r#""{}""#, old_key);
    let single_quoted = format!("'{}'", old_key);

    result = replace_key_pattern(&result, &double_quoted, old_key, new_key);
    result = replace_key_pattern(&result, &single_quoted, old_key, new_key);

    Ok(result)
}

/// Replace key patterns that appear to be JSON object keys.
pub(super) fn replace_key_pattern(
    text: &str,
    pattern: &str,
    old_key: &str,
    new_key: &str,
) -> String {
    let mut result = String::with_capacity(text.len());
    let mut last_end = 0;

    for (start, _) in text.match_indices(pattern) {
        let after_pattern = start + pattern.len();
        let rest = &text[after_pattern..];

        let trimmed = rest.trim_start();
        let _whitespace_len = rest.len() - trimmed.len();

        if trimmed.starts_with(':') {
            result.push_str(&text[last_end..start + 1]);
            result.push_str(new_key);
            result.push_str(&text[start + 1 + old_key.len()..after_pattern]);
            last_end = after_pattern;
        }
    }

    result.push_str(&text[last_end..]);

    result
}

/// Rename a key within a scoped parent object path.
/// For example, with parent_path=["parallel"], old_key="worktree_root",
/// only renames "worktree_root" keys that appear inside "parallel" objects.
pub(super) fn rename_key_in_text_scoped(
    raw: &str,
    parent_path: &[&str],
    old_key: &str,
    new_key: &str,
) -> Result<String> {
    let value = match jsonc_parser::parse_to_serde_value(raw, &Default::default()) {
        Ok(Some(v)) => v,
        _ => {
            return rename_key_in_text(raw, old_key, new_key);
        }
    };

    if !key_exists_at_path(&value, parent_path, old_key) {
        return Ok(raw.to_string());
    }

    let parent_key = parent_path[0];
    rename_key_in_object_scope(raw, parent_key, old_key, new_key)
}

/// Check if a key exists at a specific nested path in the JSON value.
pub(super) fn key_exists_at_path(value: &Value, path: &[&str], key: &str) -> bool {
    let mut current = value;

    for part in path {
        match current {
            Value::Object(map) => {
                if let Some(v) = map.get(*part) {
                    current = v;
                } else {
                    return false;
                }
            }
            _ => return false,
        }
    }

    match current {
        Value::Object(map) => map.contains_key(key),
        _ => false,
    }
}

/// Rename a key within a specific object scope in the raw text.
/// Finds the object by its key and renames the target key only within that object's scope.
pub(super) fn rename_key_in_object_scope(
    raw: &str,
    object_key: &str,
    old_key: &str,
    new_key: &str,
) -> Result<String> {
    let object_pattern = format!(r#""{}""#, object_key);

    let mut result = String::with_capacity(raw.len());
    let mut last_end = 0;

    for (start, _) in raw.match_indices(&object_pattern) {
        let after_pattern = start + object_pattern.len();
        let rest = &raw[after_pattern..];

        let rest_trimmed = rest.trim_start();
        let whitespace_before_colon = rest.len() - rest_trimmed.len();

        if !rest_trimmed.starts_with(':') {
            continue;
        }

        let after_colon = &rest_trimmed[1..];
        let after_colon_trimmed = after_colon.trim_start();
        let whitespace_after_colon = after_colon.len() - after_colon_trimmed.len();

        if !after_colon_trimmed.starts_with('{') {
            continue;
        }

        let object_content_start =
            after_pattern + whitespace_before_colon + 1 + whitespace_after_colon;

        let after_brace = object_content_start + 1;
        let mut pos = after_brace;
        let mut depth = 1;

        while pos < raw.len() && depth > 0 {
            match raw.as_bytes().get(pos) {
                Some(b'{') => depth += 1,
                Some(b'}') => depth -= 1,
                Some(b'"') => {
                    pos += 1;
                    while pos < raw.len() {
                        match raw.as_bytes().get(pos) {
                            Some(b'\\') => pos += 2,
                            Some(b'"') => {
                                pos += 1;
                                break;
                            }
                            _ => pos += 1,
                        }
                    }
                    continue;
                }
                _ => {}
            }
            pos += 1;
        }

        let object_content_end = pos;

        result.push_str(&raw[last_end..object_content_start]);

        let inner_content = &raw[object_content_start..object_content_end];
        let modified_inner = rename_key_in_text(inner_content, old_key, new_key)?;
        result.push_str(&modified_inner);

        last_end = object_content_end;
    }

    result.push_str(&raw[last_end..]);

    Ok(result)
}

/// Remove a key from a serde_json value using dot notation (e.g., "agent.runner").
pub(super) fn remove_key_from_value(value: &mut Value, key: &str) {
    let parts: Vec<&str> = key.split('.').collect();
    if parts.is_empty() {
        return;
    }

    let mut current = value;
    for part in &parts[..parts.len() - 1] {
        match current {
            Value::Object(map) => {
                if let Some(next) = map.get_mut(*part) {
                    current = next;
                } else {
                    return;
                }
            }
            _ => return,
        }
    }

    if let Value::Object(map) = current {
        map.remove(parts[parts.len() - 1]);
    }
}
