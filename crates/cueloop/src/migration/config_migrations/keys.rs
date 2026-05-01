//! Purpose: Apply config key rename and removal migrations.
//!
//! Responsibilities:
//! - Rename config keys in project/global config files.
//! - Remove deprecated config keys from parsed JSON values.
//! - Preserve JSONC comments while renaming keys.
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
//! - Leaf renames only support changing the final key within the same parent path.
//! - Simple [`MigrationType::ConfigKeyRemove`] removals reparse JSON to serde and may discard
//!   comments; `cursor_bin` removal uses JSONC AST spans to preserve formatting.

use anyhow::{Context, Result};
use jsonc_parser::ast::{Object, ObjectPropName, Value as JsoncAstValue};
use jsonc_parser::common::Ranged;
use serde_json::Value;
use std::{fs, path::Path};

use super::super::MigrationContext;
use super::detect::config_file_has_key;

/// Apply a key rename to both project and global configs.
/// Uses JSONC-aware text replacement to preserve comments.
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

/// Remove legacy Cursor binary override keys from agent defaults and profile patches.
pub fn apply_cursor_bin_remove(ctx: &MigrationContext) -> Result<()> {
    for path in super::detect::project_migration_config_disk_paths(&ctx.repo_root) {
        remove_cursor_bin_in_file(path.as_path())
            .with_context(|| format!("remove cursor_bin entries in {}", path.display()))?;
    }

    if let Some(global_path) = &ctx.global_config_path {
        remove_cursor_bin_in_file(global_path)
            .with_context(|| format!("remove cursor_bin entries in {}", global_path.display()))?;
    }

    Ok(())
}

/// Rename a key in a specific config file while preserving comments.
/// Uses JSONC AST-guided text replacement to only rename within the specified parent object.
/// For "parallel.worktree_root", only renames "worktree_root" inside "parallel" objects.
pub(crate) fn rename_key_in_file(path: &Path, old_key: &str, new_key: &str) -> Result<()> {
    let raw =
        fs::read_to_string(path).with_context(|| format!("read config file {}", path.display()))?;

    let old_parts: Vec<&str> = old_key.split('.').collect();
    let new_parts: Vec<&str> = new_key.split('.').collect();

    if old_parts.iter().any(|part| part.is_empty()) || new_parts.iter().any(|part| part.is_empty())
    {
        return Err(anyhow::anyhow!("Empty key segment"));
    }

    let empty_parent_path: &[&str] = &[];
    let old_parent_path: &[&str] = if old_parts.len() > 1 {
        &old_parts[..old_parts.len() - 1]
    } else {
        empty_parent_path
    };
    let new_parent_path: &[&str] = if new_parts.len() > 1 {
        &new_parts[..new_parts.len() - 1]
    } else {
        empty_parent_path
    };

    if old_parent_path != new_parent_path {
        return Err(anyhow::anyhow!(
            "rename key {} to {} must keep the same parent path",
            old_key,
            new_key
        ));
    }

    let old_leaf = old_parts[old_parts.len() - 1];
    let new_leaf = new_parts[new_parts.len() - 1];

    let modified = rename_key_in_text_scoped(&raw, old_parent_path, old_leaf, new_leaf)
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
pub(crate) fn remove_key_in_file(path: &Path, key: &str) -> Result<()> {
    let raw =
        fs::read_to_string(path).with_context(|| format!("read config file {}", path.display()))?;

    let mut value: Value = jsonc_parser::parse_to_serde_value::<Value>(&raw, &Default::default())?;

    if !value.is_object() {
        return Ok(());
    }

    remove_key_from_value(&mut value, key);

    let modified = serde_json::to_string_pretty(&value).context("serialize config")?;
    crate::fsutil::write_atomic(path, modified.as_bytes())
        .with_context(|| format!("write modified config to {}", path.display()))?;

    log::info!("Removed config key '{}' in {}", key, path.display());
    Ok(())
}

fn remove_cursor_bin_in_file(path: &Path) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }

    let raw =
        fs::read_to_string(path).with_context(|| format!("read config file {}", path.display()))?;

    let collect_options = jsonc_parser::CollectOptions::default();
    let parse_options = jsonc_parser::ParseOptions::default();
    let parse_result = jsonc_parser::parse_to_ast(&raw, &collect_options, &parse_options)
        .with_context(|| format!("parse JSONC for cursor_bin removal in {}", path.display()))?;

    let Some(root) = parse_result.value.as_ref() else {
        return Ok(());
    };

    let mut spans = Vec::new();
    collect_legacy_cursor_bin_spans(root, &raw, &mut spans);

    if spans.is_empty() {
        return Ok(());
    }

    let replacements: Vec<(usize, usize, String)> = spans
        .into_iter()
        .map(|(s, e)| (s, e, String::new()))
        .collect();
    let modified = apply_text_replacements(&raw, replacements);

    crate::fsutil::write_atomic(path, modified.as_bytes())
        .with_context(|| format!("write modified config to {}", path.display()))?;

    log::info!(
        "Removed legacy Cursor SDK binary override keys in {}",
        path.display()
    );
    Ok(())
}

/// Span covering a property `key: value` plus an adjacent separator comma so the object stays
/// valid JSONC after removal.
fn removal_span_for_object_prop(raw: &str, object: &Object<'_>, prop_idx: usize) -> (usize, usize) {
    let prop = &object.properties[prop_idx];
    let mut start = prop.range.start;
    let end = prop.range.end;
    let bytes = raw.as_bytes();

    let mut idx = end;
    while idx < bytes.len() && bytes[idx].is_ascii_whitespace() {
        idx += 1;
    }
    if idx < bytes.len() && bytes[idx] == b',' {
        return (start, idx + 1);
    }

    if prop_idx > 0 {
        let mut s = start;
        while s > 0 && bytes[s - 1].is_ascii_whitespace() {
            s -= 1;
        }
        if s > 0 && bytes[s - 1] == b',' {
            start = s - 1;
        }
    }

    (start, end)
}

fn collect_spans_for_cursor_bin_in_object(
    raw: &str,
    object: &Object<'_>,
    spans: &mut Vec<(usize, usize)>,
) {
    for (idx, prop) in object.properties.iter().enumerate() {
        if prop.name.as_str() == "cursor_bin" {
            spans.push(removal_span_for_object_prop(raw, object, idx));
        }
    }
}

fn collect_legacy_cursor_bin_spans(
    value: &JsoncAstValue<'_>,
    raw: &str,
    spans: &mut Vec<(usize, usize)>,
) {
    let JsoncAstValue::Object(root) = value else {
        return;
    };

    for prop in &root.properties {
        match prop.name.as_str() {
            "agent" => {
                if let JsoncAstValue::Object(agent_obj) = &prop.value {
                    collect_spans_for_cursor_bin_in_object(raw, agent_obj, spans);
                }
            }
            "profiles" => {
                if let JsoncAstValue::Object(profiles) = &prop.value {
                    for profile_prop in &profiles.properties {
                        if let JsoncAstValue::Object(profile_obj) = &profile_prop.value {
                            collect_spans_for_cursor_bin_in_object(raw, profile_obj, spans);
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

/// Rename a key in JSONC text while preserving comments and formatting.
/// Uses regex-like pattern matching to find and replace key names.
pub(super) fn rename_key_in_text(raw: &str, old_key: &str, new_key: &str) -> Result<String> {
    let mut result = raw.to_string();

    let double_quoted = format!(r#""{}""#, old_key);
    let single_quoted = format!("'{}'", old_key);

    result = replace_key_pattern(&result, &double_quoted, '"', new_key);
    result = replace_key_pattern(&result, &single_quoted, '\'', new_key);

    Ok(result)
}

/// Replace key patterns that appear to be JSON object keys.
pub(super) fn replace_key_pattern(text: &str, pattern: &str, quote: char, new_key: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut last_end = 0;

    for (start, _) in text.match_indices(pattern) {
        let after_pattern = start + pattern.len();
        let rest = &text[after_pattern..];

        let trimmed = rest.trim_start();

        if trimmed.starts_with(':') {
            result.push_str(&text[last_end..start]);
            result.push_str(&render_quoted_object_key(new_key, quote));
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
    let collect_options = jsonc_parser::CollectOptions::default();
    let parse_options = jsonc_parser::ParseOptions::default();
    let parse_result = match jsonc_parser::parse_to_ast(raw, &collect_options, &parse_options) {
        Ok(result) => result,
        Err(_) => return rename_key_in_text(raw, old_key, new_key),
    };

    let Some(root) = parse_result.value.as_ref() else {
        return Ok(raw.to_string());
    };

    let mut replacements = Vec::new();
    collect_scoped_key_renames(root, parent_path, old_key, new_key, raw, &mut replacements);

    if replacements.is_empty() {
        return Ok(raw.to_string());
    }

    Ok(apply_text_replacements(raw, replacements))
}

fn collect_scoped_key_renames<'a>(
    value: &'a JsoncAstValue<'a>,
    path: &[&str],
    old_key: &str,
    new_key: &str,
    raw: &str,
    replacements: &mut Vec<(usize, usize, String)>,
) {
    let JsoncAstValue::Object(object) = value else {
        return;
    };

    let Some((path_head, path_tail)) = path.split_first() else {
        for prop in &object.properties {
            if prop.name.as_str() == old_key {
                replacements.push((
                    prop.name.range().start,
                    prop.name.range().end,
                    render_object_prop_name_replacement(raw, &prop.name, new_key),
                ));
            }
        }
        return;
    };

    for prop in &object.properties {
        if prop.name.as_str() != *path_head {
            continue;
        }

        collect_scoped_key_renames(&prop.value, path_tail, old_key, new_key, raw, replacements);
    }
}

fn render_object_prop_name_replacement(
    raw: &str,
    name: &ObjectPropName<'_>,
    new_key: &str,
) -> String {
    match name {
        ObjectPropName::String(lit) => {
            let quote = raw[lit.range.start..lit.range.end]
                .chars()
                .next()
                .unwrap_or('"');
            render_quoted_object_key(new_key, quote)
        }
        ObjectPropName::Word(_) => {
            if is_bare_object_key(new_key) {
                new_key.to_string()
            } else {
                serde_json::to_string(new_key).unwrap_or_else(|_| format!(r#""{}""#, new_key))
            }
        }
    }
}

fn render_quoted_object_key(key: &str, quote: char) -> String {
    if quote == '\'' {
        let mut escaped = String::with_capacity(key.len() + 2);
        escaped.push(quote);
        for ch in key.chars() {
            match ch {
                '\\' => escaped.push_str("\\\\"),
                '\'' => escaped.push_str("\\'"),
                '\n' => escaped.push_str("\\n"),
                '\r' => escaped.push_str("\\r"),
                '\t' => escaped.push_str("\\t"),
                '\u{08}' => escaped.push_str("\\b"),
                '\u{0C}' => escaped.push_str("\\f"),
                c if c.is_control() => escaped.push_str(&format!("\\u{:04x}", c as u32)),
                _ => escaped.push(ch),
            }
        }
        escaped.push(quote);
        escaped
    } else {
        serde_json::to_string(key).unwrap_or_else(|_| format!(r#""{}""#, key))
    }
}

fn is_bare_object_key(key: &str) -> bool {
    let mut chars = key.chars();
    let Some(first) = chars.next() else {
        return false;
    };

    if !(first.is_ascii_alphabetic() || first == '_') {
        return false;
    }

    chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

fn apply_text_replacements(raw: &str, mut replacements: Vec<(usize, usize, String)>) -> String {
    replacements.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| b.1.cmp(&a.1)));

    let mut result = raw.to_string();
    for (start, end, replacement) in replacements {
        result.replace_range(start..end, &replacement);
    }

    result
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
