//! Queue-set validation helpers for loader entrypoints.
//!
//! Purpose:
//! - Queue-set validation helpers for loader entrypoints.
//!
//! Responsibilities:
//! - Validate loaded queue/done state using resolved config settings.
//! - Preserve the distinction between queue-only and queue+done validation.
//!
//! Not handled here:
//! - Queue parsing or timestamp repair.
//! - Filesystem persistence.
//!
//!
//! Usage:
//! - Used through the crate module tree or integration test harness.
//!
//! Invariants/assumptions:
//! - Callers supply already-loaded queue data.
//! - Callers decide whether non-blocking validation warnings should be logged.

use crate::config::Resolved;
use crate::contracts::QueueFile;
use crate::queue::validation::{self, ValidationWarning};
use anyhow::Result;

pub(super) fn validate_loaded_queues(
    resolved: &Resolved,
    queue_file: &QueueFile,
    done_file: &QueueFile,
) -> Result<Vec<ValidationWarning>> {
    validate_loaded_queues_with_warning_logging(resolved, queue_file, done_file, true)
}

pub(super) fn validate_loaded_queues_without_warning_logs(
    resolved: &Resolved,
    queue_file: &QueueFile,
    done_file: &QueueFile,
) -> Result<Vec<ValidationWarning>> {
    validate_loaded_queues_with_warning_logging(resolved, queue_file, done_file, false)
}

fn validate_loaded_queues_with_warning_logging(
    resolved: &Resolved,
    queue_file: &QueueFile,
    done_file: &QueueFile,
    log_warnings: bool,
) -> Result<Vec<ValidationWarning>> {
    let done_ref = if !done_file.tasks.is_empty() || resolved.done_path.exists() {
        Some(done_file)
    } else {
        None
    };

    let max_depth = resolved.queue_max_dependency_depth();
    let warnings = validation::validate_queue_set(
        queue_file,
        done_ref,
        &resolved.id_prefix,
        resolved.id_width,
        max_depth,
    )?;
    if log_warnings {
        validation::log_warnings(&warnings);
    }
    Ok(warnings)
}
