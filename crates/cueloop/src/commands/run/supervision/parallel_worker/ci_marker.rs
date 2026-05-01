//! Parallel-worker CI marker helpers.
//!
//! Purpose:
//! - Persist CI failure diagnostics for parallel-worker supervision.
//!
//! Responsibilities:
//! - Write the primary worker CI-failure marker file.
//! - Fall back to the alternate marker path when the primary path is unavailable.
//! - Log diagnostic details without aborting the caller on marker-write failures.
//!
//! Scope:
//! - Marker persistence only; CI execution and bookkeeping restore live elsewhere.
//!
//! Usage:
//! - Called by the parallel-worker supervision facade and companion tests.
//!
//! Invariants/assumptions:
//! - Marker payloads are JSON objects with task id, timestamp, and error text.
//! - Marker writes are best-effort and should never panic.

use std::io::Write as _;

use crate::timeutil;

/// Write a marker file indicating CI gate failure.
/// The coordinator can inspect this marker for CI failure diagnostics.
pub(super) fn write_ci_failure_marker(
    workspace_path: &std::path::Path,
    task_id: &str,
    error_message: &str,
) {
    let content = serde_json::json!({
        "task_id": task_id,
        "timestamp": timeutil::now_utc_rfc3339_or_fallback(),
        "error": error_message
    });

    let primary_marker =
        workspace_path.join(crate::commands::run::parallel::CI_FAILURE_MARKER_FILE);
    if write_marker_file(&primary_marker, &content) {
        log::debug!(
            "Wrote CI failure marker for task {} at {}",
            task_id,
            primary_marker.display()
        );
        return;
    }

    let fallback_marker =
        workspace_path.join(crate::commands::run::parallel::CI_FAILURE_MARKER_FALLBACK_FILE);
    if write_marker_file(&fallback_marker, &content) {
        log::warn!(
            "Primary CI failure marker unavailable; wrote fallback marker for task {} at {}",
            task_id,
            fallback_marker.display()
        );
        return;
    }

    log::error!(
        "Failed to write both primary and fallback CI failure markers for task {}",
        task_id
    );
}

fn write_marker_file(path: &std::path::Path, content: &serde_json::Value) -> bool {
    if let Some(parent) = path.parent()
        && let Err(e) = std::fs::create_dir_all(parent)
    {
        log::warn!("Failed to create marker parent directory: {}", e);
        return false;
    }
    match std::fs::File::create(path) {
        Ok(mut file) => {
            if let Err(e) = file.write_all(content.to_string().as_bytes()) {
                log::warn!("Failed to write marker file {}: {}", path.display(), e);
                false
            } else {
                true
            }
        }
        Err(e) => {
            log::warn!("Failed to create marker file {}: {}", path.display(), e);
            false
        }
    }
}
