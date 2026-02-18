//! History report implementation.
//!
//! Responsibilities:
//! - Generate timeline of task creation/completion events by day.
//!
//! Not handled here:
//! - Output formatting (see shared.rs).
//! - CLI argument parsing.
//!
//! Invariants/assumptions:
//! - Queue files are validated before reporting.
//! - Timestamps are RFC3339 format.

use anyhow::Result;
use serde::Serialize;
use time::{Duration, OffsetDateTime};

use crate::contracts::{QueueFile, Task};
use crate::timeutil;

use super::shared::{ReportFormat, format_date_key, print_json};

#[derive(Debug, Serialize)]
pub(crate) struct HistoryWindow {
    pub days: i64,
    pub start_date: String,
    pub end_date: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct HistoryDay {
    pub date: String,
    pub created: Vec<String>,
    pub completed: Vec<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct HistoryReport {
    pub window: HistoryWindow,
    pub days: Vec<HistoryDay>,
}

fn start_of_window(now: OffsetDateTime, days_to_show: i64) -> OffsetDateTime {
    // These time component replacements (0 hour, minute, second, nanosecond) are always valid
    // for any valid OffsetDateTime. The expect calls document this invariant.
    (now - Duration::days(days_to_show - 1))
        .replace_hour(0)
        .expect("hour 0 is always valid")
        .replace_minute(0)
        .expect("minute 0 is always valid")
        .replace_second(0)
        .expect("second 0 is always valid")
        .replace_nanosecond(0)
        .expect("nanosecond 0 is always valid")
}

fn collect_all_tasks<'a>(queue: &'a QueueFile, done: Option<&'a QueueFile>) -> Vec<&'a Task> {
    let mut all_tasks: Vec<&Task> = queue.tasks.iter().collect();
    if let Some(done_file) = done {
        all_tasks.extend(done_file.tasks.iter().collect::<Vec<&Task>>());
    }
    all_tasks
}

pub(crate) fn build_history_report(
    queue: &QueueFile,
    done: Option<&QueueFile>,
    days: u32,
) -> HistoryReport {
    let all_tasks = collect_all_tasks(queue, done);
    let days_to_show = days.max(1) as i64;
    let now = OffsetDateTime::now_utc();
    let start_of_day = start_of_window(now, days_to_show);
    let end_of_day = start_of_day + Duration::days(days_to_show - 1);

    let mut created_by_day: std::collections::BTreeMap<String, Vec<String>> =
        std::collections::BTreeMap::new();
    let mut completed_by_day: std::collections::BTreeMap<String, Vec<String>> =
        std::collections::BTreeMap::new();

    for task in all_tasks {
        if let Some(created_ts) = &task.created_at
            && let Ok(dt) = timeutil::parse_rfc3339(created_ts)
            && dt >= start_of_day
        {
            let day_key = format_date_key(dt);
            created_by_day
                .entry(day_key)
                .or_default()
                .push(task.id.clone());
        }

        if let Some(completed_ts) = &task.completed_at
            && let Ok(dt) = timeutil::parse_rfc3339(completed_ts)
            && dt >= start_of_day
        {
            let day_key = format_date_key(dt);
            completed_by_day
                .entry(day_key)
                .or_default()
                .push(task.id.clone());
        }
    }

    let mut days = Vec::new();
    for i in 0..days_to_show {
        let day_dt = start_of_day + Duration::days(i);
        let day_key = format_date_key(day_dt);
        let created = created_by_day.get(&day_key).cloned().unwrap_or_default();
        let completed = completed_by_day.get(&day_key).cloned().unwrap_or_default();
        days.push(HistoryDay {
            date: day_key,
            created,
            completed,
        });
    }

    HistoryReport {
        window: HistoryWindow {
            days: days_to_show,
            start_date: format_date_key(start_of_day),
            end_date: format_date_key(end_of_day),
        },
        days,
    }
}

/// Print history of task events by day.
///
/// # Arguments
/// * `queue` - Active queue tasks
/// * `done` - Completed tasks (optional)
/// * `days` - Number of days to show (default: 7)
pub(crate) fn print_history(
    queue: &QueueFile,
    done: Option<&QueueFile>,
    days: u32,
    format: ReportFormat,
) -> Result<()> {
    let report = build_history_report(queue, done, days);

    match format {
        ReportFormat::Json => {
            print_json(&report)?;
        }
        ReportFormat::Text => {
            println!(
                "Task History (last {} day{})",
                report.window.days,
                if report.window.days == 1 { "" } else { "s" }
            );
            println!(
                "================{}",
                "=".repeat(if report.window.days == 1 { 11 } else { 12 })
            );
            println!();

            let mut has_events = false;

            for day in &report.days {
                if day.created.is_empty() && day.completed.is_empty() {
                    continue;
                }

                has_events = true;

                println!("{}", day.date);
                if !day.created.is_empty() {
                    println!("  Created: {}", day.created.join(", "));
                }
                if !day.completed.is_empty() {
                    println!("  Completed: {}", day.completed.join(", "));
                }
                println!();
            }

            if !has_events {
                println!(
                    "No task creation or completion events in the last {} day{}.",
                    report.window.days,
                    if report.window.days == 1 { "" } else { "s" }
                );
            }
        }
    }

    Ok(())
}
