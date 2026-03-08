//! Timestamp repair helpers for explicit queue maintenance flows.
//!
//! Responsibilities:
//! - Normalize queue task timestamps into canonical UTC formatting.
//! - Backfill terminal `completed_at` values during explicit repair flows.
//! - Persist maintenance changes and emit a concise repair log.
//!
//! Not handled here:
//! - Queue parsing or JSON repair.
//! - Semantic validation of queue invariants.
//!
//! Invariants/assumptions:
//! - Callers only invoke these helpers for explicit repair flows.
//! - Read-only load paths must not call these helpers.

use crate::contracts::{QueueFile, Task};
use anyhow::{Context, Result};
use std::path::Path;
use time::UtcOffset;

#[derive(Debug, Default, Clone, Copy)]
pub(super) struct QueueMaintenanceReport {
    normalized_timestamps: usize,
    backfilled_completed_at: usize,
    queue_changed: bool,
    done_changed: bool,
}

impl QueueMaintenanceReport {
    fn has_changes(self) -> bool {
        self.normalized_timestamps > 0 || self.backfilled_completed_at > 0
    }
}

#[derive(Debug, Default, Clone, Copy)]
struct SingleQueueMaintenance {
    normalized_timestamps: usize,
    backfilled_completed_at: usize,
    changed: bool,
}

fn normalize_timestamp_field(field: &mut Option<String>) -> Result<bool> {
    let Some(raw) = field.as_ref() else {
        return Ok(false);
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(false);
    }

    let dt = match crate::timeutil::parse_rfc3339(trimmed) {
        Ok(dt) => dt,
        Err(_) => return Ok(false),
    };

    if dt.offset() == UtcOffset::UTC {
        return Ok(false);
    }

    let normalized = crate::timeutil::format_rfc3339(dt)?;
    if normalized == *raw {
        return Ok(false);
    }
    *field = Some(normalized);
    Ok(true)
}

fn normalize_task_timestamps(task: &mut Task) -> Result<usize> {
    let mut normalized = 0usize;

    if normalize_timestamp_field(&mut task.created_at)? {
        normalized += 1;
    }
    if normalize_timestamp_field(&mut task.updated_at)? {
        normalized += 1;
    }
    if normalize_timestamp_field(&mut task.completed_at)? {
        normalized += 1;
    }
    if normalize_timestamp_field(&mut task.started_at)? {
        normalized += 1;
    }
    if normalize_timestamp_field(&mut task.scheduled_start)? {
        normalized += 1;
    }

    Ok(normalized)
}

fn maintain_single_queue_timestamps(
    queue: &mut QueueFile,
    now_utc: &str,
) -> Result<SingleQueueMaintenance> {
    let mut normalized_timestamps = 0usize;
    for task in &mut queue.tasks {
        normalized_timestamps += normalize_task_timestamps(task)?;
    }

    let backfilled_completed_at = super::super::backfill_terminal_completed_at(queue, now_utc);
    let changed = normalized_timestamps > 0 || backfilled_completed_at > 0;

    Ok(SingleQueueMaintenance {
        normalized_timestamps,
        backfilled_completed_at,
        changed,
    })
}

fn log_maintenance_report(report: QueueMaintenanceReport, queue_path: &Path, done_path: &Path) {
    if !report.has_changes() {
        return;
    }

    log::warn!(
        "Queue repair applied: normalized {} non-UTC timestamp(s), backfilled {} terminal completed_at value(s). Saved queue={}, done={} (queue_path={}, done_path={}).",
        report.normalized_timestamps,
        report.backfilled_completed_at,
        report.queue_changed,
        report.done_changed,
        queue_path.display(),
        done_path.display()
    );
}

pub(super) fn maintain_and_save_loaded_queues(
    queue_path: &Path,
    queue_file: &mut QueueFile,
    done_path: &Path,
    done_path_exists: bool,
    done_file: &mut QueueFile,
) -> Result<QueueMaintenanceReport> {
    let now = crate::timeutil::now_utc_rfc3339()?;

    let queue_report = maintain_single_queue_timestamps(queue_file, &now)?;
    let done_report = maintain_single_queue_timestamps(done_file, &now)?;

    if queue_report.changed {
        super::super::save_queue(queue_path, queue_file)
            .with_context(|| format!("save auto-repaired queue {}", queue_path.display()))?;
    }
    if done_report.changed && (done_path_exists || !done_file.tasks.is_empty()) {
        super::super::save_queue(done_path, done_file)
            .with_context(|| format!("save auto-repaired done {}", done_path.display()))?;
    }

    let report = QueueMaintenanceReport {
        normalized_timestamps: queue_report.normalized_timestamps
            + done_report.normalized_timestamps,
        backfilled_completed_at: queue_report.backfilled_completed_at
            + done_report.backfilled_completed_at,
        queue_changed: queue_report.changed,
        done_changed: done_report.changed,
    };

    log_maintenance_report(report, queue_path, done_path);
    Ok(report)
}
