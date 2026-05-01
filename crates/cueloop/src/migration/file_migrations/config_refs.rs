//! Purpose: Update config file references after file migrations.
//!
//! Responsibilities:
//! - Detect queue/done file references that match migrated paths.
//! - Rewrite project/global config references while preserving JSONC comments
//!   through text replacement.
//!
//! Scope:
//! - Config reference updates only; generic rename behavior and JSON-to-JSONC
//!   orchestration live in sibling modules.
//!
//! Usage:
//! - Invoked after file rename operations when `update_config` is enabled.
//!
//! Invariants/Assumptions:
//! - Only exact queue.file and queue.done_file path matches are rewritten.
//! - Raw config replacement remains string-based to preserve comments.

use crate::config;
use anyhow::{Context, Result};
use std::{
    fs,
    path::{Path, PathBuf},
};

use super::super::MigrationContext;

/// Update config file references after a file move.
/// Updates queue.file and queue.done_file if they match the old path.
pub(super) fn update_config_file_references(
    ctx: &MigrationContext,
    old_path: &str,
    new_path: &str,
) -> Result<()> {
    if ctx.project_config_path.exists() {
        update_config_file_if_needed(&ctx.project_config_path, old_path, new_path)
            .context("update project config file references")?;
    }

    if let Some(global_path) = &ctx.global_config_path
        && global_path.exists()
    {
        update_config_file_if_needed(global_path, old_path, new_path)
            .context("update global config file references")?;
    }

    Ok(())
}

/// Update a specific config file's file references.
pub(super) fn update_config_file_if_needed(
    config_path: &Path,
    old_path: &str,
    new_path: &str,
) -> Result<()> {
    let layer = config::load_layer(config_path)
        .with_context(|| format!("load config from {}", config_path.display()))?;

    let old_path_buf = PathBuf::from(old_path);
    let _new_path_buf = PathBuf::from(new_path);

    let mut needs_update = false;

    if let Some(ref file) = layer.queue.file
        && file == &old_path_buf
    {
        needs_update = true;
    }

    if let Some(ref done_file) = layer.queue.done_file
        && done_file == &old_path_buf
    {
        needs_update = true;
    }

    if !needs_update {
        return Ok(());
    }

    let raw = fs::read_to_string(config_path)
        .with_context(|| format!("read config {}", config_path.display()))?;

    let updated = raw.replace(&format!("\"{}\"", old_path), &format!("\"{}\"", new_path));

    crate::fsutil::write_atomic(config_path, updated.as_bytes())
        .with_context(|| format!("write updated config to {}", config_path.display()))?;

    log::info!(
        "Updated file reference in {}: {} -> {}",
        config_path.display(),
        old_path,
        new_path
    );

    Ok(())
}
