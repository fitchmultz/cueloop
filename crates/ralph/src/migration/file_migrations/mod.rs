//! Purpose: Directory-backed facade for file migration helpers.
//!
//! Responsibilities:
//! - Declare focused file migration companion modules.
//! - Re-export the stable file migration API used by migration dispatch code.
//! - Keep regression coverage colocated with the facade.
//!
//! Scope:
//! - Thin facade only; rename logic, config reference updates, and JSON-to-JSONC
//!   migration behavior live in companion modules.
//!
//! Usage:
//! - Imported through `crate::migration::file_migrations::*` by `migration::mod`.
//!
//! Invariants/Assumptions:
//! - Public signatures remain stable across this split.
//! - File migration semantics, backup defaults, and rollback behavior stay unchanged.
//! - Companion modules stay private to the migration boundary.

mod config_refs;
mod json_to_jsonc;
mod rename;

#[cfg(test)]
mod tests;

pub use json_to_jsonc::{
    is_config_json_to_jsonc_applicable, is_done_json_to_jsonc_applicable,
    is_queue_json_to_jsonc_applicable, migrate_config_json_to_jsonc, migrate_done_json_to_jsonc,
    migrate_queue_json_to_jsonc,
};
pub use rename::{
    FileMigrationOptions, apply_file_rename, apply_file_rename_with_options,
    rollback_file_migration,
};
