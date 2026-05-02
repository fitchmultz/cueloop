//! Prompt version-tracking storage.
//!
//! Purpose:
//! - Prompt version-tracking storage.
//!
//! Responsibilities:
//! - Load and save prompt version metadata on disk.
//! - Define the persisted schema for exported prompt digests.
//!
//! Not handled here:
//! - Prompt export/sync policy.
//! - Template discovery.
//!
//!
//! Usage:
//! - Used through the crate module tree or integration test harness.
//!
//! Invariants/assumptions:
//! - Schema version `2` is the only accepted persisted format.
//! - Unknown or legacy files are ignored and replaced on the next export/sync write.

use crate::constants::identity::{LEGACY_PROJECT_RUNTIME_DIR, PROJECT_RUNTIME_DIR};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

pub(crate) const PROMPT_VERSION_SCHEMA: u32 = 2;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct PromptVersionInfo {
    pub schema_version: u32,
    pub cueloop_version: String,
    pub exported_at: String,
    pub templates: HashMap<String, TemplateVersion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct TemplateVersion {
    pub digest: String,
    pub exported_at: String,
}

fn current_version_file_path(repo_root: &Path) -> PathBuf {
    repo_root
        .join(PROJECT_RUNTIME_DIR)
        .join("cache/prompt_versions.json")
}

fn legacy_version_file_path(repo_root: &Path) -> PathBuf {
    repo_root
        .join(LEGACY_PROJECT_RUNTIME_DIR)
        .join("cache/prompt_versions.json")
}

pub(crate) fn load_version_info(repo_root: &Path) -> Result<Option<PromptVersionInfo>> {
    let current_path = current_version_file_path(repo_root);
    let legacy_path = legacy_version_file_path(repo_root);
    let path = if current_path.exists() {
        current_path
    } else if legacy_path.exists() {
        legacy_path
    } else {
        return Ok(None);
    };

    let content = fs::read_to_string(&path)
        .with_context(|| format!("read version file {}", path.display()))?;
    let info: PromptVersionInfo = match serde_json::from_str(&content) {
        Ok(info) => info,
        Err(error) => {
            log::debug!(
                "Ignoring prompt version file {} during schema cutover: {}",
                path.display(),
                error
            );
            return Ok(None);
        }
    };

    if info.schema_version != PROMPT_VERSION_SCHEMA {
        log::debug!(
            "Ignoring prompt version file {} with unsupported schema_version {}",
            path.display(),
            info.schema_version
        );
        return Ok(None);
    }

    Ok(Some(info))
}

pub(crate) fn save_version_info(repo_root: &Path, info: &PromptVersionInfo) -> Result<()> {
    let path = current_version_file_path(repo_root);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("create directory {}", parent.display()))?;
    }
    let content = serde_json::to_string_pretty(info).context("serialize version info")?;
    fs::write(&path, content).with_context(|| format!("write version file {}", path.display()))?;
    Ok(())
}
