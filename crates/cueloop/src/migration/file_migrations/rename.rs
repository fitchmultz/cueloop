//! Purpose: Apply generic file rename and rollback migrations.
//!
//! Responsibilities:
//! - Copy files from old path to new path safely.
//! - Create destination parents as needed.
//! - Optionally keep backups and update config references.
//! - Roll back rename migrations when the original backup still exists.
//!
//! Scope:
//! - Generic file move/rename behavior only; JSON-to-JSONC wrappers and config
//!   reference update internals live in sibling modules.
//!
//! Usage:
//! - Used by `MigrationType::FileRename` and by JSON-to-JSONC wrapper helpers.
//!
//! Invariants/Assumptions:
//! - Source must exist before migration.
//! - Destination must not already exist unless source and destination are identical.
//! - Keeping backups defaults to true.

use anyhow::{Context, Result};
use std::fs;

use super::super::MigrationContext;
use super::config_refs::update_config_file_references;

/// Options for file migration.
#[derive(Debug, Clone)]
pub struct FileMigrationOptions {
    /// Whether to keep the original file as a backup.
    pub keep_backup: bool,
    /// Whether to update config file references.
    pub update_config: bool,
}

impl Default for FileMigrationOptions {
    fn default() -> Self {
        Self {
            keep_backup: true,
            update_config: true,
        }
    }
}

/// Apply a file rename migration.
/// Copies content from old_path to new_path and optionally updates config.
pub fn apply_file_rename(ctx: &MigrationContext, old_path: &str, new_path: &str) -> Result<()> {
    let opts = FileMigrationOptions::default();
    apply_file_rename_with_options(ctx, old_path, new_path, &opts)
}

/// Apply a file rename migration with custom options.
pub fn apply_file_rename_with_options(
    ctx: &MigrationContext,
    old_path: &str,
    new_path: &str,
    opts: &FileMigrationOptions,
) -> Result<()> {
    let old_full_path = ctx.resolve_path(old_path);
    let new_full_path = ctx.resolve_path(new_path);

    if !old_full_path.exists() {
        anyhow::bail!("Source file does not exist: {}", old_full_path.display());
    }

    if new_full_path.exists() && old_full_path != new_full_path {
        anyhow::bail!(
            "Destination file already exists: {}",
            new_full_path.display()
        );
    }

    if let Some(parent) = new_full_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("create parent directory {}", parent.display()))?;
    }

    fs::copy(&old_full_path, &new_full_path).with_context(|| {
        format!(
            "copy {} to {}",
            old_full_path.display(),
            new_full_path.display()
        )
    })?;

    log::info!(
        "Migrated file from {} to {}",
        old_full_path.display(),
        new_full_path.display()
    );

    if opts.update_config {
        update_config_file_references(ctx, old_path, new_path)
            .context("update config file references")?;
    }

    if !opts.keep_backup {
        fs::remove_file(&old_full_path)
            .with_context(|| format!("remove original file {}", old_full_path.display()))?;
        log::debug!("Removed original file {}", old_full_path.display());
    } else {
        log::debug!("Kept original file {} as backup", old_full_path.display());
    }

    Ok(())
}

/// Rollback a file migration by restoring from backup.
/// This removes the new file and restores the original.
pub fn rollback_file_migration(
    ctx: &MigrationContext,
    old_path: &str,
    new_path: &str,
) -> Result<()> {
    let old_full_path = ctx.resolve_path(old_path);
    let new_full_path = ctx.resolve_path(new_path);

    if !old_full_path.exists() {
        anyhow::bail!(
            "Cannot rollback: original file {} does not exist",
            old_full_path.display()
        );
    }

    if new_full_path.exists() {
        fs::remove_file(&new_full_path)
            .with_context(|| format!("remove migrated file {}", new_full_path.display()))?;
    }

    log::info!(
        "Rolled back file migration: restored {}, removed {}",
        old_full_path.display(),
        new_full_path.display()
    );

    Ok(())
}
