//! Repo-local execution trust loading.
//!
//! Purpose:
//! - Repo-local execution trust loading.
//!
//! Responsibilities:
//! - Define the local trust file contract for execution-sensitive project settings.
//! - Load active `.cueloop/trust.jsonc` files with JSONC support.
//! - Provide helpers for source-aware trust checks during config resolution.
//!
//! Not handled here:
//! - Main config layering or schema generation (see `crate::contracts::config`).
//! - CI command validation or execution (see `crate::config::validation` and `crate::runutil`).
//!
//!
//! Usage:
//! - Used through the crate module tree or integration test harness.
//!
//! Invariants/assumptions:
//! - Trust is local-only and must not be committed to version control.
//! - Missing trust files mean the repo is untrusted.
//! - Trust file writes use the same JSONC parse path on read and standard JSON on write.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use time::OffsetDateTime;

use super::resolution::project_runtime_dir;
use crate::fsutil;

/// Local trust file for execution-sensitive project configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct RepoTrust {
    /// Allow repo-local executable configuration such as `agent.ci_gate`.
    pub allow_project_commands: bool,

    /// Timestamp for the explicit trust decision.
    ///
    /// Serialized as RFC3339 to stay compatible with the historical chrono
    /// `DateTime<Utc>` wire format. `None` serializes as `null`, matching the
    /// prior chrono behavior, and is also tolerated when the field is absent.
    #[serde(with = "time::serde::rfc3339::option", default)]
    pub trusted_at: Option<OffsetDateTime>,
}

impl RepoTrust {
    pub fn is_trusted(&self) -> bool {
        self.allow_project_commands
    }
}

/// Preferred local trust path for a repository root.
pub fn project_trust_path(repo_root: &Path) -> PathBuf {
    project_runtime_dir(repo_root).join("trust.jsonc")
}

/// Load repo trust if present, otherwise return the default untrusted state.
pub fn load_repo_trust(repo_root: &Path) -> Result<RepoTrust> {
    let path = project_trust_path(repo_root);
    if !path.exists() {
        return Ok(RepoTrust::default());
    }

    let raw = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    crate::jsonc::parse_jsonc::<RepoTrust>(&raw, &format!("trust {}", path.display()))
}

/// Outcome of [`initialize_repo_trust_file`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrustFileInitStatus {
    /// Wrote a new repo-local `trust.jsonc` in the active runtime directory.
    Created,
    /// Updated an existing file (enabled trust or backfilled `trusted_at`).
    Updated,
    /// File already marked the repo trusted; left unchanged on disk.
    Unchanged,
}

fn write_trust_file(path: &Path, trust: &RepoTrust) -> Result<()> {
    let rendered = crate::jsonc::to_string_pretty(trust)
        .with_context(|| format!("serialize {}", path.display()))?;
    fsutil::write_atomic(path, rendered.as_bytes())
        .with_context(|| format!("write {}", path.display()))
}

fn print_trust_warning() {
    eprintln!(
        "Warning: Trusting this repo allows project-local execution settings under the active runtime config\n\
         (runner binary overrides, Cursor/plugin runner selection, agent.ci_gate, plugins.*, parallel.ignored_file_allowlist) to take effect. Review that file first.\n\
         Keep the repo-local trust.jsonc untracked; do not commit it."
    );
}

/// Create or update repo-local `trust.jsonc` so [`RepoTrust::is_trusted`] becomes true.
///
/// - Creates the active runtime directory when missing.
/// - If the file is absent, writes `allow_project_commands: true` and `trusted_at` set to the
///   current UTC instant.
/// - If the file exists: when already trusted with a `trusted_at` timestamp, leaves the file
///   unchanged; otherwise merges in trust (backfills `trusted_at` or enables `allow_project_commands`).
pub fn initialize_repo_trust_file(repo_root: &Path) -> Result<TrustFileInitStatus> {
    let runtime_dir = project_runtime_dir(repo_root);
    fs::create_dir_all(&runtime_dir)
        .with_context(|| format!("create {}", runtime_dir.display()))?;

    let path = project_trust_path(repo_root);
    if !path.exists() {
        print_trust_warning();
        let trust = RepoTrust {
            allow_project_commands: true,
            trusted_at: Some(OffsetDateTime::now_utc()),
        };
        write_trust_file(&path, &trust)?;
        eprintln!(
            "trust: created {} (do not commit this file)",
            path.display()
        );
        return Ok(TrustFileInitStatus::Created);
    }

    let existing = load_repo_trust(repo_root)?;
    if existing.allow_project_commands && existing.trusted_at.is_some() {
        eprintln!(
            "trust: unchanged ({} already allows project commands)",
            path.display()
        );
        return Ok(TrustFileInitStatus::Unchanged);
    }

    print_trust_warning();
    let trust = RepoTrust {
        allow_project_commands: true,
        trusted_at: Some(OffsetDateTime::now_utc()),
    };
    write_trust_file(&path, &trust)?;
    eprintln!(
        "trust: updated {} (do not commit this file)",
        path.display()
    );
    Ok(TrustFileInitStatus::Updated)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn project_trust_path_defaults_to_cueloop_jsonc() {
        let repo_root = TempDir::new().expect("temp dir");
        assert_eq!(
            project_trust_path(repo_root.path()),
            repo_root.path().join(".cueloop/trust.jsonc")
        );
    }

    #[test]
    fn load_repo_trust_ignores_old_json_file() {
        let repo_root = TempDir::new().expect("temp dir");
        let cueloop_dir = repo_root.path().join(".cueloop");
        fs::create_dir_all(&cueloop_dir).expect("create .cueloop");
        fs::write(
            cueloop_dir.join("trust.json"),
            r#"{"allow_project_commands":true}"#,
        )
        .expect("write old trust file");

        assert_eq!(
            load_repo_trust(repo_root.path()).expect("load trust"),
            RepoTrust::default()
        );
    }

    #[test]
    fn initialize_repo_trust_file_creates_valid_trust_roundtrip() {
        let repo_root = TempDir::new().expect("temp dir");
        let status = initialize_repo_trust_file(repo_root.path()).expect("init trust");
        assert_eq!(status, TrustFileInitStatus::Created);
        let loaded = load_repo_trust(repo_root.path()).expect("reload");
        assert!(loaded.is_trusted());
        assert!(loaded.trusted_at.is_some());
    }

    #[test]
    fn initialize_repo_trust_file_idempotent_when_fully_trusted() {
        let repo_root = TempDir::new().expect("temp dir");
        assert_eq!(
            initialize_repo_trust_file(repo_root.path()).expect("first"),
            TrustFileInitStatus::Created
        );
        let first = fs::read_to_string(project_trust_path(repo_root.path())).expect("read");
        assert_eq!(
            initialize_repo_trust_file(repo_root.path()).expect("second"),
            TrustFileInitStatus::Unchanged
        );
        let second = fs::read_to_string(project_trust_path(repo_root.path())).expect("read");
        assert_eq!(first, second, "second run must not rewrite bytes");
    }

    #[test]
    fn initialize_repo_trust_file_backfills_trusted_at() {
        let repo_root = TempDir::new().expect("temp dir");
        let runtime_dir = repo_root.path().join(".cueloop");
        fs::create_dir_all(&runtime_dir).expect("create .cueloop");
        fs::write(
            project_trust_path(repo_root.path()),
            r#"{"allow_project_commands":true}"#,
        )
        .expect("write trust");

        assert_eq!(
            initialize_repo_trust_file(repo_root.path()).expect("merge"),
            TrustFileInitStatus::Updated
        );
        let loaded = load_repo_trust(repo_root.path()).expect("reload");
        assert!(loaded.trusted_at.is_some());
    }

    /// Regression: trust files previously written by `chrono`'s
    /// `DateTime<Utc>` serde (e.g. `2024-01-15T12:30:00.120Z`) must still
    /// deserialize into the `time`-backed `RepoTrust`. time's rfc3339
    /// deserializer accepts both the `Z` and `+00:00` offset forms and any
    /// subsecond precision.
    #[test]
    fn repo_trust_parses_chrono_rfc3339_offset_form() {
        let json =
            r#"{"allow_project_commands":true,"trusted_at":"2024-01-15T12:30:00.123456789+00:00"}"#;
        let parsed: RepoTrust = serde_json::from_str(json).expect("+00:00 offset form must parse");
        assert!(parsed.is_trusted());
        let trusted_at = parsed.trusted_at.expect("trusted_at present");
        assert_eq!(trusted_at.offset(), time::UtcOffset::UTC);
    }

    #[test]
    fn repo_trust_parses_chrono_rfc3339_z_form() {
        let json =
            r#"{"allow_project_commands":true,"trusted_at":"2024-01-15T12:30:00.123456789Z"}"#;
        let parsed: RepoTrust = serde_json::from_str(json).expect("Z shorthand form must parse");
        assert_eq!(
            parsed.trusted_at.expect("trusted_at present").offset(),
            time::UtcOffset::UTC
        );
    }

    /// chrono serializes a `None` timestamp as `null`; that must still load
    /// (and also tolerate the field being absent entirely).
    #[test]
    fn repo_trust_parses_null_and_missing_trusted_at() {
        let with_null: RepoTrust =
            serde_json::from_str(r#"{"allow_project_commands":true,"trusted_at":null}"#)
                .expect("null trusted_at must parse as None");
        assert!(with_null.is_trusted());
        assert!(with_null.trusted_at.is_none());

        let without_field: RepoTrust = serde_json::from_str(r#"{"allow_project_commands":true}"#)
            .expect("missing trusted_at must default to None");
        assert!(without_field.trusted_at.is_none());
    }
}
