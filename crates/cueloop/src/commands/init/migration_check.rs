//! Pending-migration warning helpers.
//!
//! Purpose:
//! - Detect and report unapplied migrations during `cueloop init`.
//!
//! Responsibilities:
//! - Build a migration context from resolved config.
//! - Print a warning when migrations are pending.
//! - Treat unavailable migration context as a non-fatal condition.
//!
//! Scope:
//! - Warning/reporting logic only; migration execution lives in `crate::migration`.
//!
//! Usage:
//! - Called by the initialization workflow after files are created or validated.
//!
//! Invariants/assumptions:
//! - Missing migration context should not fail initialization.
//! - Warning text remains human-readable for direct CLI invocation.

use anyhow::Result;
use colored::Colorize;

use crate::config;

/// Check for pending migrations and display a warning if any exist.
pub(super) fn check_pending_migrations(resolved: &config::Resolved) -> Result<()> {
    use crate::migration::{self, MigrationCheckResult};

    let ctx = match migration::MigrationContext::from_resolved(resolved) {
        Ok(ctx) => ctx,
        Err(e) => {
            log::debug!("Could not create migration context: {}", e);
            return Ok(());
        }
    };

    match migration::check_migrations(&ctx)? {
        MigrationCheckResult::Current => {}
        MigrationCheckResult::Pending(migrations) => {
            eprintln!();
            eprintln!(
                "{}",
                format!("⚠ Warning: {} migration(s) pending", migrations.len()).yellow()
            );
            eprintln!("Run {} to apply them.", "cueloop migrate --apply".cyan());
        }
    }

    Ok(())
}
