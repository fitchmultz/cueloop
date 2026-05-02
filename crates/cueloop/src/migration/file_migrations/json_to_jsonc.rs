//! Purpose: Orchestrate JSON-to-JSONC migration wrappers for CueLoop state files.
//!
//! Responsibilities:
//! - Provide queue/done/config JSON-to-JSONC convenience entrypoints.
//! - Detect whether each legacy JSON migration is applicable.
//! - Normalize config references and clean up legacy files when JSONC already exists.
//!
//! Scope:
//! - JSON-to-JSONC orchestration only; generic rename behavior and config reference
//!   update internals live in sibling modules.
//!
//! Usage:
//! - Used by file migration dispatch for `.cueloop/*.json -> .cueloop/*.jsonc` cutovers.
//!
//! Invariants/Assumptions:
//! - If the `.jsonc` file already exists, the legacy `.json` file should be removed.
//! - Config references must be normalized before deleting an already-legacy file.

use anyhow::{Context, Result};
use std::fs;

use super::super::MigrationContext;
use super::config_refs::update_config_file_references;
use super::rename::{FileMigrationOptions, apply_file_rename_with_options};

/// Migrate queue.json to queue.jsonc.
/// This is a convenience function for the common case.
pub fn migrate_queue_json_to_jsonc(ctx: &MigrationContext) -> Result<()> {
    migrate_json_to_jsonc(ctx, ".cueloop/queue.json", ".cueloop/queue.jsonc")
        .context("migrate queue.json to queue.jsonc")
}

/// Migrate done.json to done.jsonc.
pub fn migrate_done_json_to_jsonc(ctx: &MigrationContext) -> Result<()> {
    migrate_json_to_jsonc(ctx, ".cueloop/done.json", ".cueloop/done.jsonc")
        .context("migrate done.json to done.jsonc")
}

/// Check if a migration from queue.json to queue.jsonc is applicable.
pub fn is_queue_json_to_jsonc_applicable(ctx: &MigrationContext) -> bool {
    ctx.file_exists(".cueloop/queue.json")
}

/// Check if a migration from done.json to done.jsonc is applicable.
pub fn is_done_json_to_jsonc_applicable(ctx: &MigrationContext) -> bool {
    ctx.file_exists(".cueloop/done.json")
}

/// Migrate config.json to config.jsonc.
pub fn migrate_config_json_to_jsonc(ctx: &MigrationContext) -> Result<()> {
    migrate_json_to_jsonc(ctx, ".cueloop/config.json", ".cueloop/config.jsonc")
        .context("migrate config.json to config.jsonc")
}

/// Check if a migration from config.json to config.jsonc is applicable.
pub fn is_config_json_to_jsonc_applicable(ctx: &MigrationContext) -> bool {
    ctx.file_exists(".cueloop/config.json")
}

pub(super) fn migrate_json_to_jsonc(
    ctx: &MigrationContext,
    old_path: &str,
    new_path: &str,
) -> Result<()> {
    let old_full_path = ctx.resolve_path(old_path);
    let new_full_path = ctx.resolve_path(new_path);

    if !old_full_path.exists() {
        return Ok(());
    }

    if new_full_path.exists() {
        update_config_file_references(ctx, old_path, new_path)
            .context("update config references for established jsonc migration")?;
        fs::remove_file(&old_full_path)
            .with_context(|| format!("remove legacy file {}", old_full_path.display()))?;
        log::info!(
            "Removed legacy file {} because {} already exists",
            old_full_path.display(),
            new_full_path.display()
        );
        return Ok(());
    }

    let opts = FileMigrationOptions {
        keep_backup: false,
        update_config: true,
    };
    apply_file_rename_with_options(ctx, old_path, new_path, &opts)
}
