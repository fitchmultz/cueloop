//! Purpose: Safeguard text-dump helpers for troubleshooting output.
//!
//! Responsibilities:
//! - Write redacted safeguard dumps by default.
//! - Gate raw safeguard dumps behind explicit opt-in.
//! - Persist dump output under CueLoop-managed temp directories.
//!
//! Scope:
//! - Safeguard dump orchestration only; redaction pattern implementation and temp cleanup policy live elsewhere.
//!
//! Usage:
//! - Used by scan and runtime orchestration paths when output needs to be preserved for debugging.
//!
//! Invariants/Assumptions:
//! - Redacted dumps are the safe default.
//! - Raw dumps require either debug mode or an explicit env-var opt-in.
//! - Successful dumps persist their temp directory so callers can inspect the output later.

use crate::constants::paths::ENV_RAW_DUMP;
use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

use super::temp::create_cueloop_temp_dir;

/// Writes a safeguard dump with redaction applied to sensitive content.
///
/// This is the recommended default for error dumps. Secrets like API keys,
/// bearer tokens, AWS keys, and SSH keys are masked before writing.
///
/// Returns the path to the written file.
pub fn safeguard_text_dump_redacted(label: &str, content: &str) -> Result<PathBuf> {
    use crate::redaction::redact_text;
    let redacted_content = redact_text(content);
    safeguard_text_dump_internal(label, &redacted_content)
}

/// Writes a safeguard dump with raw (non-redacted) content.
///
/// SECURITY WARNING: This function writes raw content that may contain secrets.
/// It requires explicit opt-in via either:
/// - Setting the `CUELOOP_RAW_DUMP=1` environment variable
/// - Passing `is_debug_mode=true` (e.g., when `--debug` flag is used)
///
/// If opt-in is not provided, this function returns an error.
/// For safe dumping, use `safeguard_text_dump_redacted` instead.
pub fn safeguard_text_dump(label: &str, content: &str, is_debug_mode: bool) -> Result<PathBuf> {
    let raw_dump_enabled = raw_dump_env_enabled();

    if !raw_dump_enabled && !is_debug_mode {
        anyhow::bail!(
            "Raw safeguard dumps require explicit opt-in. \
             Set {}=1 or use --debug mode. \
             Consider using safeguard_text_dump_redacted() for safe dumping.",
            ENV_RAW_DUMP
        );
    }

    if raw_dump_enabled {
        log::warn!(
            "SECURITY: Writing raw safeguard dump ({}=1). Secrets may be written to disk.",
            ENV_RAW_DUMP
        );
    }

    safeguard_text_dump_internal(label, content)
}

fn raw_dump_env_enabled() -> bool {
    std::env::var(ENV_RAW_DUMP)
        .is_ok_and(|value| value == "1" || value.eq_ignore_ascii_case("true"))
}

fn safeguard_text_dump_internal(label: &str, content: &str) -> Result<PathBuf> {
    let temp_dir = create_cueloop_temp_dir(label)?;
    let output_path = temp_dir.path().join("output.txt");
    fs::write(&output_path, content)
        .with_context(|| format!("write safeguard dump to {}", output_path.display()))?;

    // Persist the temp dir so it's not deleted when the TempDir object is dropped.
    let dir_path = temp_dir.keep();
    Ok(dir_path.join("output.txt"))
}
