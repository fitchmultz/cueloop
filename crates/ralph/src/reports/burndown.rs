//! Burndown report implementation.
//!
//! Responsibilities:
//! - Generate burndown chart of remaining tasks over time.
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

use crate::contracts::QueueFile;
use crate::timeutil;

use super::shared::{ReportFormat, format_date_key, print_json};

#[derive(Debug, Serialize)]
pub(crate) struct BurndownWindow {
    pub days: i64,
    pub start_date: String,
    pub end_date: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct BurndownDay {
    pub date: String,
    pub remaining: usize,
}

#[derive(Debug, Serialize)]
pub(crate) struct BurndownLegend {
    pub scale_per_block: usize,
}

#[derive(Debug, Serialize)]
pub(crate) struct BurndownReport {
    pub window: BurndownWindow,
    pub daily_counts: Vec<BurndownDay>,
    pub max_count: usize,
    pub legend: Option<BurndownLegend>,
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

pub(crate) fn build_burndown_report(
    queue: &QueueFile,
    done: Option<&QueueFile>,
    days: u32,
) -> BurndownReport {
    let days_to_show = days.max(1) as i64;
    let now = OffsetDateTime::now_utc();
    let start_of_day = start_of_window(now, days_to_show);
    let end_of_day = start_of_day + Duration::days(days_to_show - 1);

    let mut all_tasks: Vec<&crate::contracts::Task> = queue.tasks.iter().collect();
    if let Some(done_file) = done {
        all_tasks.extend(
            done_file
                .tasks
                .iter()
                .collect::<Vec<&crate::contracts::Task>>(),
        );
    }

    let mut daily_counts: std::collections::BTreeMap<String, usize> =
        std::collections::BTreeMap::new();

    for i in 0..days_to_show {
        let day_dt = start_of_day + Duration::days(i);
        let day_end = day_dt + Duration::days(1) - Duration::seconds(1);

        let mut remaining = 0;
        for task in &all_tasks {
            let created = task
                .created_at
                .as_ref()
                .and_then(|ts| timeutil::parse_rfc3339(ts).ok());
            let completed = task
                .completed_at
                .as_ref()
                .and_then(|ts| timeutil::parse_rfc3339(ts).ok());

            let is_open = match created {
                Some(created_dt) => {
                    let created_before_or_on = created_dt <= day_end;
                    let not_completed_yet = match completed {
                        Some(completed_dt) => completed_dt > day_end,
                        None => true,
                    };
                    created_before_or_on && not_completed_yet
                }
                None => false,
            };

            if is_open {
                remaining += 1;
            }
        }

        let day_key = format_date_key(day_dt);
        daily_counts.insert(day_key, remaining);
    }

    let max_count = *daily_counts.values().max().unwrap_or(&0);
    let legend = if max_count == 0 {
        None
    } else {
        Some(BurndownLegend {
            scale_per_block: (max_count / 20).max(1),
        })
    };

    let daily_counts = daily_counts
        .into_iter()
        .map(|(date, remaining)| BurndownDay { date, remaining })
        .collect();

    BurndownReport {
        window: BurndownWindow {
            days: days_to_show,
            start_date: format_date_key(start_of_day),
            end_date: format_date_key(end_of_day),
        },
        daily_counts,
        max_count,
        legend,
    }
}

/// Print burndown chart of remaining tasks over time.
///
/// # Arguments
/// * `queue` - Active queue tasks
/// * `done` - Completed tasks (optional)
/// * `days` - Number of days to show (default: 7)
pub(crate) fn print_burndown(
    queue: &QueueFile,
    done: Option<&QueueFile>,
    days: u32,
    format: ReportFormat,
) -> Result<()> {
    let report = build_burndown_report(queue, done, days);

    match format {
        ReportFormat::Json => {
            print_json(&report)?;
        }
        ReportFormat::Text => {
            println!(
                "Task Burndown (last {} day{})",
                report.window.days,
                if report.window.days == 1 { "" } else { "s" }
            );
            println!(
                "================{}",
                "=".repeat(if report.window.days == 1 { 11 } else { 12 })
            );
            println!();

            if report.daily_counts.is_empty() {
                println!("No data to display.");
                return Ok(());
            }

            if report.max_count == 0 {
                println!(
                    "No remaining tasks in the last {} day{}.",
                    report.window.days,
                    if report.window.days == 1 { "" } else { "s" }
                );
                return Ok(());
            }

            println!("Remaining Tasks");
            println!();

            for day in &report.daily_counts {
                let bar_len =
                    (day.remaining as f64 / report.max_count as f64 * 20.0).round() as usize;
                let bar = "█".repeat(bar_len);

                println!("  {} | {} {}", day.date, bar, day.remaining);
            }

            println!();
            println!(
                "█ = ~{} task{}",
                (report.max_count / 20).max(1),
                if report.max_count / 20 == 1 { "" } else { "s" }
            );
        }
    }

    Ok(())
}
