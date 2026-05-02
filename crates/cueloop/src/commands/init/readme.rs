//! README file version management for CueLoop initialization.
//!
//! Purpose:
//! - README file version management for CueLoop initialization.
//!
//! Responsibilities:
//! - Track README template versions via embedded version markers.
//! - Detect outdated README files and support updates.
//! - Create new README files from embedded template.
//!
//! Not handled here:
//! - Queue/config file creation (see `super::writers`).
//! - Prompt content validation (handled by `crate::prompts`).
//!
//!
//! Usage:
//! - Used through the crate module tree or integration test harness.
//!
//! Invariants/assumptions:
//! - README_VERSION is incremented when template changes.
//! - Version marker format: `<!-- CUELOOP_README_VERSION: X -->`.
//! - Legacy `RALPH_README_VERSION` markers are still parsed.
//! - Legacy files without markers are treated as version 1.

use crate::config;
use crate::constants::identity::{LEGACY_README_MARKER, README_MARKER};
use crate::constants::versions::README_VERSION;
use crate::fsutil;
use crate::prompts;
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;
use thiserror::Error;

/// Errors that can occur when extracting README version.
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum ReadmeVersionError {
    /// No version marker found in the file (legacy file).
    #[error("no version marker found")]
    NoMarker,

    /// Version marker is malformed (e.g., missing closing `-->`).
    #[error("malformed version marker: missing closing '-->'")]
    InvalidFormat,

    /// Version value could not be parsed as a non-negative integer.
    #[error("invalid version value: '{value}' is not a valid non-negative integer")]
    ParseError { value: String },
}

const DEFAULT_RALPH_README: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/cueloop_readme.md"
));

const RUNTIME_DIR_PLACEHOLDER: &str = "{{RUNTIME_DIR}}";

/// Result of checking if README is current.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadmeCheckResult {
    /// README is current with the specified version.
    Current(u32),
    /// README is outdated (has older version).
    Outdated {
        current_version: u32,
        embedded_version: u32,
    },
    /// README is missing.
    Missing,
    /// README not applicable (prompts don't reference it).
    NotApplicable,
}

/// Extract version from README content.
///
/// Current generated files use `CUELOOP_README_VERSION`; legacy generated files with
/// `RALPH_README_VERSION` are accepted so old runtime READMEs can be refreshed.
pub fn extract_readme_version(content: &str) -> Result<u32, ReadmeVersionError> {
    for marker in [README_MARKER, LEGACY_README_MARKER] {
        if let Some(version) = extract_readme_version_for_marker(content, marker)? {
            return Ok(version);
        }
    }
    Err(ReadmeVersionError::NoMarker)
}

fn extract_readme_version_for_marker(
    content: &str,
    marker: &str,
) -> Result<Option<u32>, ReadmeVersionError> {
    let marker_start = format!("<!-- {marker}:");

    let Some(start_idx) = content.find(&marker_start) else {
        return Ok(None);
    };

    let after_marker = &content[start_idx + marker_start.len()..];

    let Some(end_idx) = after_marker.find("-->") else {
        return Err(ReadmeVersionError::InvalidFormat);
    };

    let trimmed = after_marker[..end_idx].trim();
    match trimmed.parse::<u32>() {
        Ok(version) => Ok(Some(version)),
        Err(_) => Err(ReadmeVersionError::ParseError {
            value: trimmed.to_string(),
        }),
    }
}

/// Check if README is current without modifying it.
/// Returns the check result with version information.
pub fn check_readme_current(resolved: &config::Resolved) -> Result<ReadmeCheckResult> {
    check_readme_current_from_root(&resolved.repo_root)
}

/// Check if README is current from a repo root path.
/// This is a helper for migration context that doesn't need full Resolved config.
pub fn check_readme_current_from_root(repo_root: &std::path::Path) -> Result<ReadmeCheckResult> {
    // First check if README is applicable
    if !prompts::prompts_reference_readme(repo_root)? {
        return Ok(ReadmeCheckResult::NotApplicable);
    }

    let readme_path = config::project_runtime_dir(repo_root).join("README.md");

    if !readme_path.exists() {
        return Ok(ReadmeCheckResult::Missing);
    }

    let content = fs::read_to_string(&readme_path)
        .with_context(|| format!("read {}", readme_path.display()))?;

    let current_version = match extract_readme_version(&content) {
        Ok(version) => version,
        Err(ReadmeVersionError::NoMarker) => 1, // Legacy file, treat as v1
        Err(e) => {
            return Err(anyhow::anyhow!(e).context(format!(
                "README version marker in {} is malformed",
                readme_path.display()
            )));
        }
    };

    if current_version >= README_VERSION {
        Ok(ReadmeCheckResult::Current(current_version))
    } else {
        Ok(ReadmeCheckResult::Outdated {
            current_version,
            embedded_version: README_VERSION,
        })
    }
}

/// Write README file, creating it when missing and refreshing it when outdated.
/// Returns (status, version) tuple - version is Some if README was read/created.
pub fn write_readme(path: &Path, force: bool) -> Result<(super::FileInitStatus, Option<u32>)> {
    let should_update = if path.exists() && !force {
        let content =
            fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
        let current_version = match extract_readme_version(&content) {
            Ok(version) => version,
            Err(ReadmeVersionError::NoMarker) => 1,
            Err(e) => {
                return Err(anyhow::anyhow!(e).context(format!(
                    "README version marker in {} is malformed",
                    path.display()
                )));
            }
        };
        current_version < README_VERSION
    } else {
        true // Create new or force overwrite.
    };

    if !should_update {
        let content =
            fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
        let version = match extract_readme_version(&content) {
            Ok(v) => Some(v),
            Err(ReadmeVersionError::NoMarker) => None,
            Err(e) => {
                return Err(anyhow::anyhow!(e).context(format!(
                    "README version marker in {} is malformed",
                    path.display()
                )));
            }
        };
        return Ok((super::FileInitStatus::Valid, version));
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }

    let is_update = path.exists();
    let rendered = render_default_readme(path);
    fsutil::write_atomic(path, rendered.as_bytes())
        .with_context(|| format!("write readme {}", path.display()))?;

    if is_update {
        Ok((super::FileInitStatus::Updated, Some(README_VERSION)))
    } else {
        Ok((super::FileInitStatus::Created, Some(README_VERSION)))
    }
}

fn render_default_readme(path: &Path) -> String {
    let runtime_name = path
        .parent()
        .and_then(|parent| parent.file_name())
        .and_then(|name| name.to_str())
        .unwrap_or(crate::constants::identity::PROJECT_RUNTIME_DIR);
    DEFAULT_RALPH_README.replace(RUNTIME_DIR_PLACEHOLDER, runtime_name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contracts::Config;
    use tempfile::TempDir;

    fn resolved_for(dir: &TempDir) -> config::Resolved {
        let repo_root = dir.path().to_path_buf();
        let queue_path = repo_root.join(".cueloop/queue.jsonc");
        let done_path = repo_root.join(".cueloop/done.jsonc");
        let project_config_path = Some(repo_root.join(".cueloop/config.jsonc"));
        config::Resolved {
            config: Config::default(),
            repo_root,
            queue_path,
            done_path,
            id_prefix: "RQ".to_string(),
            id_width: 4,
            global_config_path: None,
            project_config_path,
        }
    }

    #[test]
    fn extract_readme_version_finds_version_marker() {
        let content = "<!-- CUELOOP_README_VERSION: 6 -->\n# Heading";
        assert_eq!(extract_readme_version(content), Ok(6));

        let legacy_content = "<!-- RALPH_README_VERSION: 2 -->\n# Ralph";
        assert_eq!(extract_readme_version(legacy_content), Ok(2));
    }

    #[test]
    fn extract_readme_version_returns_error_for_no_marker() {
        let content = "# CueLoop runtime files\nSome content";
        // Legacy content without marker returns NoMarker error
        assert!(matches!(
            extract_readme_version(content),
            Err(ReadmeVersionError::NoMarker)
        ));
    }

    #[test]
    fn extract_readme_version_returns_error_for_invalid_version() {
        let content = "<!-- CUELOOP_README_VERSION: invalid -->\n# Heading";
        let result = extract_readme_version(content);
        assert!(
            matches!(result, Err(ReadmeVersionError::ParseError { value }) if value == "invalid")
        );
    }

    #[test]
    fn extract_readme_version_returns_error_for_malformed_marker() {
        let content = "<!-- CUELOOP_README_VERSION: 6 \n# Heading"; // Missing -->
        let result = extract_readme_version(content);
        assert!(matches!(result, Err(ReadmeVersionError::InvalidFormat)));
    }

    #[test]
    fn extract_readme_version_handles_whitespace() {
        let content = "<!-- CUELOOP_README_VERSION:   3   -->\n# Heading";
        assert_eq!(extract_readme_version(content), Ok(3));
    }

    #[test]
    fn extract_readme_version_rejects_negative_numbers() {
        let content = "<!-- CUELOOP_README_VERSION: -1 -->\n# Heading";
        let result = extract_readme_version(content);
        assert!(matches!(result, Err(ReadmeVersionError::ParseError { value }) if value == "-1"));
    }

    #[test]
    fn extract_readme_version_rejects_floats() {
        let content = "<!-- CUELOOP_README_VERSION: 1.5 -->\n# Heading";
        let result = extract_readme_version(content);
        assert!(matches!(result, Err(ReadmeVersionError::ParseError { value }) if value == "1.5"));
    }

    #[test]
    fn write_readme_creates_new_file_with_version() -> Result<()> {
        let dir = TempDir::new()?;
        let readme_path = dir.path().join(".cueloop/README.md");

        let (status, version) = write_readme(&readme_path, false)?;

        assert_eq!(status, super::super::FileInitStatus::Created);
        assert_eq!(version, Some(README_VERSION));
        assert!(readme_path.exists());

        let content = std::fs::read_to_string(&readme_path)?;
        assert!(content.contains("CUELOOP_README_VERSION"));
        assert!(content.contains(".cueloop/queue.jsonc"));
        assert!(!content.contains(RUNTIME_DIR_PLACEHOLDER));
        Ok(())
    }

    #[test]
    fn write_readme_renders_legacy_runtime_paths() -> Result<()> {
        let dir = TempDir::new()?;
        let readme_path = dir.path().join(".ralph/README.md");

        write_readme(&readme_path, false)?;

        let content = std::fs::read_to_string(&readme_path)?;
        assert!(content.contains(".ralph/queue.jsonc"));
        assert!(content.contains(".ralph/config.jsonc"));
        assert!(!content.contains(".cueloop/queue.jsonc"));
        assert!(!content.contains(RUNTIME_DIR_PLACEHOLDER));
        Ok(())
    }

    #[test]
    fn write_readme_updates_when_version_mismatch() -> Result<()> {
        let dir = TempDir::new()?;
        let readme_path = dir.path().join(".cueloop/README.md");

        // Create an existing README with old version
        fs::create_dir_all(readme_path.parent().unwrap())?;
        let old_content = "<!-- RALPH_README_VERSION: 1 -->\n# Old content";
        std::fs::write(&readme_path, old_content)?;

        let (status, version) = write_readme(&readme_path, false)?;

        assert_eq!(status, super::super::FileInitStatus::Updated);
        assert_eq!(version, Some(README_VERSION));

        // Content should be updated
        let content = std::fs::read_to_string(&readme_path)?;
        assert!(!content.contains("Old content"));
        assert!(content.contains("CueLoop runtime files"));
        Ok(())
    }

    #[test]
    fn write_readme_skips_update_when_current() -> Result<()> {
        let dir = TempDir::new()?;
        let readme_path = dir.path().join(".cueloop/README.md");

        // Create an existing README with current version
        fs::create_dir_all(readme_path.parent().unwrap())?;
        let current_content = format!(
            "<!-- CUELOOP_README_VERSION: {} -->\n# Current",
            README_VERSION
        );
        std::fs::write(&readme_path, &current_content)?;

        let (status, version) = write_readme(&readme_path, false)?;

        // Should be Valid since version is current
        assert_eq!(status, super::super::FileInitStatus::Valid);
        assert_eq!(version, Some(README_VERSION));

        // Content should be preserved
        let content = std::fs::read_to_string(&readme_path)?;
        assert!(content.contains("Current"));
        Ok(())
    }

    #[test]
    fn write_readme_force_overwrites_regardless() -> Result<()> {
        let dir = TempDir::new()?;
        let readme_path = dir.path().join(".cueloop/README.md");

        // Create an existing README
        fs::create_dir_all(readme_path.parent().unwrap())?;
        std::fs::write(
            &readme_path,
            "<!-- RALPH_README_VERSION: 99 -->\n# OLD_FORCE_OVERWRITE_SENTINEL",
        )?;

        let (status, version) = write_readme(&readme_path, true)?;

        // When force-overwriting an existing file, status is Updated
        assert_eq!(status, super::super::FileInitStatus::Updated);
        assert_eq!(version, Some(README_VERSION));

        // Content should be overwritten
        let content = std::fs::read_to_string(&readme_path)?;
        assert!(!content.contains("OLD_FORCE_OVERWRITE_SENTINEL"));
        Ok(())
    }

    #[test]
    fn check_readme_current_detects_missing() -> Result<()> {
        let dir = TempDir::new()?;
        let resolved = resolved_for(&dir);

        let result = check_readme_current(&resolved)?;

        // README is applicable but missing
        assert!(matches!(result, ReadmeCheckResult::Missing));
        Ok(())
    }

    #[test]
    fn check_readme_current_detects_outdated() -> Result<()> {
        let dir = TempDir::new()?;
        let resolved = resolved_for(&dir);

        // Create README with old version
        fs::create_dir_all(resolved.repo_root.join(".cueloop"))?;
        let old_readme = "<!-- RALPH_README_VERSION: 1 -->\n# Old";
        fs::write(resolved.repo_root.join(".cueloop/README.md"), old_readme)?;

        let result = check_readme_current(&resolved)?;

        assert!(
            matches!(result, ReadmeCheckResult::Outdated { current_version: 1, embedded_version } if embedded_version == README_VERSION)
        );
        Ok(())
    }

    #[test]
    fn check_readme_current_detects_current() -> Result<()> {
        let dir = TempDir::new()?;
        let resolved = resolved_for(&dir);

        // Create README with current version
        fs::create_dir_all(resolved.repo_root.join(".cueloop"))?;
        let current_readme = format!(
            "<!-- CUELOOP_README_VERSION: {} -->\n# Current",
            README_VERSION
        );
        fs::write(
            resolved.repo_root.join(".cueloop/README.md"),
            current_readme,
        )?;

        let result = check_readme_current(&resolved)?;

        assert!(matches!(result, ReadmeCheckResult::Current(v) if v == README_VERSION));
        Ok(())
    }
}
