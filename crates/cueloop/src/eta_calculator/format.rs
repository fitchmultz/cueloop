//! Purpose: Render ETA durations into compact human-readable strings.
//!
//! Responsibilities:
//! - Convert `Duration` values into stable CLI/report-friendly labels.
//!
//! Scope:
//! - Formatting only; ETA modeling and calculation live in sibling modules.
//!
//! Usage:
//! - Used anywhere Ralph needs to present ETA values through
//!   `crate::eta_calculator::format_eta`.
//!
//! Invariants/Assumptions:
//! - Output stays compact (`30s`, `1m 30s`, `1h 1m`).
//! - Sub-minute, minute, and hour formatting boundaries remain stable.

use std::time::Duration;

/// Format a duration as a human-readable ETA string.
pub fn format_eta(duration: Duration) -> String {
    let total_secs = duration.as_secs();

    if total_secs < 60 {
        format!("{}s", total_secs)
    } else if total_secs < 3600 {
        let mins = total_secs / 60;
        let secs = total_secs % 60;
        if secs > 0 {
            format!("{}m {}s", mins, secs)
        } else {
            format!("{}m", mins)
        }
    } else {
        let hours = total_secs / 3600;
        let mins = (total_secs % 3600) / 60;
        if mins > 0 {
            format!("{}h {}m", hours, mins)
        } else {
            format!("{}h", hours)
        }
    }
}
