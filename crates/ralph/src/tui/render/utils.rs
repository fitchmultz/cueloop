//! Shared rendering helpers for the TUI.
//!
//! Responsibilities:
//! - Provide text wrapping and color helpers for panel rendering.
//! - Provide small formatting helpers used by multiple renderers.
//!
//! Not handled here:
//! - Layout logic or widget composition.
//! - Event handling or state mutation.
//!
//! Invariants/assumptions:
//! - Callers clamp input widths before rendering to avoid zero-width layouts.

use crate::contracts::{TaskPriority, TaskStatus};
use ratatui::style::Color;

/// Wrap text to fit within a given width.
pub(super) fn wrap_text(text: &str, width: usize) -> Vec<String> {
    // `textwrap` requires a non-zero width. Extremely small layouts can yield 0
    // (e.g., panel width smaller than padding). Clamp to keep rendering resilient.
    let width = width.max(1);

    textwrap::wrap(text, width)
        .into_iter()
        .map(|s| s.into_owned())
        .collect()
}

/// Get the color for a task status.
pub(super) fn status_color(status: TaskStatus) -> Color {
    match status {
        TaskStatus::Draft => Color::DarkGray,
        TaskStatus::Todo => Color::Blue,
        TaskStatus::Doing => Color::Yellow,
        TaskStatus::Done => Color::Green,
        TaskStatus::Rejected => Color::Red,
    }
}

/// Get the color for a task priority.
pub(super) fn priority_color(priority: TaskPriority) -> Color {
    match priority {
        TaskPriority::Critical => Color::Red,
        TaskPriority::High => Color::Yellow,
        TaskPriority::Medium => Color::Blue,
        TaskPriority::Low => Color::DarkGray,
    }
}

/// Format a scroll indicator string when content exceeds the viewport.
pub(super) fn scroll_indicator(
    scroll: usize,
    visible_lines: usize,
    total_lines: usize,
) -> Option<String> {
    if total_lines <= visible_lines {
        return None;
    }

    let start = scroll.saturating_add(1);
    let end = (scroll + visible_lines).min(total_lines);
    let percent = if total_lines == 0 {
        0
    } else {
        (end.saturating_mul(100)) / total_lines
    };
    Some(format!("({start}-{end}/{total_lines}, {percent}%)"))
}
