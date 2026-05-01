//! Plan-file source loading for task decomposition.
//!
//! Purpose:
//! - Load file-backed task decomposition sources for planner-backed queue tree creation.
//!
//! Responsibilities:
//! - Normalize user-supplied plan-file paths, including leading-tilde expansion.
//! - Enforce file-only, UTF-8, non-empty, bounded-size input before planner invocation.
//! - Record stable provenance paths for CLI, machine, and app preview output.
//!
//! Not handled here:
//! - Source kind resolution for inline task IDs or freeform requests.
//! - Planner prompt rendering or queue mutation.
//!
//! Usage:
//! - CLI and machine handlers call `read_plan_file_source` when `--from-file` is supplied.
//!
//! Invariants/assumptions:
//! - Relative paths are resolved against the process current working directory.
//! - Files inside the repo are recorded repo-relative; files outside are recorded as absolute paths.

use super::types::TaskDecomposeSourceInput;
use crate::{config, fsutil};
use anyhow::{Context, Result, bail};
use std::path::Path;

pub const MAX_PLAN_FILE_BYTES: u64 = 1024 * 1024;

pub fn read_plan_file_source(
    resolved: &config::Resolved,
    path: &Path,
) -> Result<TaskDecomposeSourceInput> {
    let expanded = fsutil::expand_tilde(path);
    let absolute = if expanded.is_absolute() {
        expanded
    } else {
        std::env::current_dir()
            .context("resolve current directory for plan file path")?
            .join(expanded)
    };
    let display_input = path.display().to_string();

    let canonical = absolute
        .canonicalize()
        .with_context(|| format!("Plan file not found: {display_input}"))?;
    let metadata = canonical
        .metadata()
        .with_context(|| format!("Inspect plan file metadata: {}", canonical.display()))?;
    if !metadata.is_file() {
        bail!("Plan file path is not a file: {display_input}");
    }
    if metadata.len() > MAX_PLAN_FILE_BYTES {
        bail!(
            "Plan file {} is too large for task decomposition ({}; limit {}).",
            display_path_for_error(&canonical, &display_input),
            format_bytes(metadata.len()),
            format_bytes(MAX_PLAN_FILE_BYTES)
        );
    }

    let content = std::fs::read_to_string(&canonical)
        .with_context(|| format!("Read plan file {} as UTF-8 text", canonical.display()))?;
    if content.trim().is_empty() {
        bail!(
            "Plan file {} is empty; provide a plan with enough content to decompose.",
            display_path_for_error(&canonical, &display_input)
        );
    }

    Ok(TaskDecomposeSourceInput::PlanFile {
        path: display_path(resolved, &canonical)?,
        content,
    })
}

fn display_path(resolved: &config::Resolved, canonical: &Path) -> Result<String> {
    let repo_root = resolved
        .repo_root
        .canonicalize()
        .with_context(|| format!("canonicalize repo root {}", resolved.repo_root.display()))?;
    let display = canonical
        .strip_prefix(&repo_root)
        .map(path_to_string)
        .unwrap_or_else(|_| path_to_string(canonical));
    Ok(display)
}

fn display_path_for_error(canonical: &Path, fallback: &str) -> String {
    if fallback.trim().is_empty() {
        path_to_string(canonical)
    } else {
        fallback.to_string()
    }
}

fn path_to_string(path: &Path) -> String {
    let value = path.display().to_string();
    if std::path::MAIN_SEPARATOR == '\\' {
        value.replace('\\', "/")
    } else {
        value
    }
}

fn format_bytes(bytes: u64) -> String {
    const MIB: f64 = 1024.0 * 1024.0;
    if bytes >= 1024 * 1024 {
        format!("{:.1} MiB", bytes as f64 / MIB)
    } else if bytes >= 1024 {
        format!("{:.1} KiB", bytes as f64 / 1024.0)
    } else {
        format!("{bytes} B")
    }
}
