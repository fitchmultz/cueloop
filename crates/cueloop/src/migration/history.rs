//! Migration history persistence for tracking applied migrations.
//!
//! Purpose:
//! - Migration history persistence for tracking applied migrations.
//!
//! Responsibilities:
//! - Load and save migration history from the active runtime cache.
//! - Provide default history for new projects.
//!
//! Not handled here:
//! - Migration execution logic (see `super::mod.rs`).
//! - Config file modifications (see `config_migrations/`).
//!
//!
//! Usage:
//! - Used through the crate module tree or integration test harness.
//!
//! Invariants/assumptions:
//! - History file is stored in the active runtime directory's `cache/migrations.jsonc`.
//! - History format is versioned for future compatibility.

use crate::constants::versions::HISTORY_VERSION;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use time::OffsetDateTime;

/// Migration history tracking all applied migrations.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct MigrationHistory {
    /// Schema version for migration history.
    pub version: u32,
    /// List of applied migrations.
    pub applied_migrations: Vec<AppliedMigration>,
}

impl Default for MigrationHistory {
    fn default() -> Self {
        Self {
            version: HISTORY_VERSION,
            applied_migrations: Vec::new(),
        }
    }
}

/// A single applied migration entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppliedMigration {
    /// Unique identifier for the migration.
    pub id: String,
    /// Timestamp when the migration was applied.
    ///
    /// Serialized as RFC3339 to stay compatible with the historical chrono
    /// `DateTime<Utc>` wire format used in `cache/migrations.jsonc`.
    #[serde(with = "time::serde::rfc3339")]
    pub applied_at: OffsetDateTime,
    /// Type of migration (for informational purposes).
    pub migration_type: String,
}

/// Load migration history from the repo.
/// Returns default (empty) history if file doesn't exist.
pub fn load_migration_history(repo_root: &Path) -> Result<MigrationHistory> {
    let history_path = migration_history_path(repo_root);

    if !history_path.exists() {
        log::debug!(
            "Migration history not found at {}, using default",
            history_path.display()
        );
        return Ok(MigrationHistory::default());
    }

    let raw = fs::read_to_string(&history_path)
        .with_context(|| format!("read migration history from {}", history_path.display()))?;

    let history: MigrationHistory = serde_json::from_str(&raw)
        .with_context(|| format!("parse migration history from {}", history_path.display()))?;

    // Validate version
    if history.version != HISTORY_VERSION {
        log::warn!(
            "Migration history version mismatch: expected {}, got {}. Attempting to proceed.",
            HISTORY_VERSION,
            history.version
        );
    }

    log::debug!(
        "Loaded migration history with {} applied migrations",
        history.applied_migrations.len()
    );

    Ok(history)
}

/// Save migration history to the repo.
pub fn save_migration_history(repo_root: &Path, history: &MigrationHistory) -> Result<()> {
    let history_path = migration_history_path(repo_root);

    // Ensure parent directory exists
    if let Some(parent) = history_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("create migration history directory {}", parent.display()))?;
    }

    let raw =
        serde_json::to_string_pretty(history).context("serialize migration history to JSON")?;

    crate::fsutil::write_atomic(&history_path, raw.as_bytes())
        .with_context(|| format!("write migration history to {}", history_path.display()))?;

    log::debug!(
        "Saved migration history with {} applied migrations",
        history.applied_migrations.len()
    );

    Ok(())
}

/// Get the path to the migration history file.
pub fn migration_history_path(repo_root: &Path) -> PathBuf {
    crate::config::project_runtime_dir(repo_root)
        .join("cache")
        .join("migrations.jsonc")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn load_migration_history_returns_default_when_missing() {
        let dir = TempDir::new().unwrap();
        let history = load_migration_history(dir.path()).unwrap();

        assert_eq!(history.version, HISTORY_VERSION);
        assert!(history.applied_migrations.is_empty());
    }

    #[test]
    fn save_and_load_migration_history_round_trips() {
        let dir = TempDir::new().unwrap();

        // Create and save a history
        let mut history = MigrationHistory::default();
        history.applied_migrations.push(AppliedMigration {
            id: "test_migration_1".to_string(),
            applied_at: OffsetDateTime::now_utc(),
            migration_type: "config_key_rename".to_string(),
        });
        history.applied_migrations.push(AppliedMigration {
            id: "test_migration_2".to_string(),
            applied_at: OffsetDateTime::now_utc(),
            migration_type: "file_rename".to_string(),
        });

        save_migration_history(dir.path(), &history).unwrap();

        // Load it back
        let loaded = load_migration_history(dir.path()).unwrap();

        assert_eq!(loaded.version, HISTORY_VERSION);
        assert_eq!(loaded.applied_migrations.len(), 2);
        assert_eq!(loaded.applied_migrations[0].id, "test_migration_1");
        assert_eq!(loaded.applied_migrations[1].id, "test_migration_2");
    }

    #[test]
    fn migration_history_path_defaults_to_cueloop() {
        let dir = crate::testsupport::path::portable_abs_path("test_repo");
        let path = migration_history_path(&dir);

        assert_eq!(path, dir.join(".cueloop/cache/migrations.jsonc"));
    }

    #[test]
    fn migration_history_path_uses_legacy_runtime_when_marked() {
        let dir = TempDir::new().unwrap();
        let cueloop_dir = dir.path().join(".cueloop");
        fs::create_dir_all(&cueloop_dir).unwrap();
        fs::write(cueloop_dir.join("config.jsonc"), r#"{"version":2}"#).unwrap();

        assert_eq!(
            migration_history_path(dir.path()),
            dir.path().join(".cueloop/cache/migrations.jsonc")
        );
    }

    #[test]
    fn save_migration_history_creates_parent_directories() {
        let dir = TempDir::new().unwrap();
        let deep_path = dir.path().join(".cueloop/cache");

        // Ensure the directory doesn't exist yet
        assert!(!deep_path.exists());

        let history = MigrationHistory::default();
        save_migration_history(dir.path(), &history).unwrap();

        // Directory should now exist
        assert!(deep_path.exists());
    }

    /// Regression: migration history files previously written by `chrono`'s
    /// `DateTime<Utc>` serde (e.g. `2024-01-15T12:30:00.120Z`) must still
    /// deserialize into the `time`-backed `AppliedMigration`. time's rfc3339
    /// deserializer accepts both the `Z` and `+00:00` offset forms and any
    /// subsecond precision.
    #[test]
    fn applied_migration_parses_chrono_rfc3339_offset_form() {
        let json = r#"{"id":"x","applied_at":"2024-01-15T12:30:00.123456789+00:00","migration_type":"test"}"#;
        let parsed: AppliedMigration =
            serde_json::from_str(json).expect("+00:00 offset form must parse");
        assert_eq!(parsed.id, "x");
        assert_eq!(parsed.applied_at.offset(), time::UtcOffset::UTC);
    }

    #[test]
    fn applied_migration_parses_chrono_rfc3339_z_form() {
        let json =
            r#"{"id":"x","applied_at":"2024-01-15T12:30:00.123456789Z","migration_type":"test"}"#;
        let parsed: AppliedMigration =
            serde_json::from_str(json).expect("Z shorthand form must parse");
        assert_eq!(parsed.applied_at.offset(), time::UtcOffset::UTC);
    }

    /// Pin the actual on-disk wire format: the serializer emits RFC3339 with
    /// a `Z` suffix and trims trailing fractional zeros (time's behavior, e.g.
    /// `.12Z` rather than chrono's `.120Z`). The bytes are RFC3339 and
    /// re-parse cleanly; files written by this binary stay interoperable with
    /// the previous chrono-written form.
    #[test]
    fn applied_migration_serializes_rfc3339_z_and_reparses() {
        let original = AppliedMigration {
            id: "pin".to_string(),
            // 2024-01-15T12:30:00.120 UTC — chrono would emit `.120Z`.
            applied_at: OffsetDateTime::from_unix_timestamp_nanos(1_705_321_800_120_000_000)
                .expect("valid timestamp"),
            migration_type: "test".to_string(),
        };
        let serialized = serde_json::to_string(&original).expect("serialize");
        assert!(
            serialized.contains(r#""applied_at":"2024-01-15T12:30:00.12Z""#),
            "unexpected wire format: {serialized}"
        );
        let reparsed: AppliedMigration =
            serde_json::from_str(&serialized).expect("round-trip parse");
        assert_eq!(reparsed.applied_at, original.applied_at);
    }
}
