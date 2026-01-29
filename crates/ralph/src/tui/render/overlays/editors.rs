//! Config and task editor overlay rendering.
//!
//! Responsibilities:
//! - Render project config editor overlay with field selection and editing.
//! - Render task field editor overlay with various field types (text, list, map, cycle).
//!
//! Not handled here:
//! - Config/task entry generation (handled by `App::config_entries` and `App::task_edit_entries`).
//! - Input handling and value editing (handled by app event loop).
//!
//! Invariants/assumptions:
//! - Callers provide a properly sized terminal area.
//! - Editing values are tracked via `TextInput` state.

use crate::outpututil::truncate_chars;
use crate::tui::config_edit::RiskLevel;
use crate::tui::{App, ConfigFieldKind, TaskEditKind, TextInput};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};

/// Draw config editor overlay.
pub fn draw_config_editor(
    f: &mut Frame<'_>,
    app: &App,
    area: Rect,
    selected: usize,
    editing_value: Option<&TextInput>,
) {
    let entries = app.config_entries();
    if entries.is_empty() {
        return;
    }

    let popup_width = 86.min(area.width.saturating_sub(4)).max(40);
    let popup_height = (entries.len() as u16 + 6)
        .min(area.height.saturating_sub(4))
        .max(8);

    let popup_area = Rect {
        x: area.x + (area.width.saturating_sub(popup_width)) / 2,
        y: area.y + (area.height.saturating_sub(popup_height)) / 2,
        width: popup_width,
        height: popup_height,
    };

    f.render_widget(Clear, popup_area);

    let title = Line::from(vec![
        Span::styled(
            "Project Config",
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        Span::styled("(.ralph/config.json)", Style::default().fg(Color::DarkGray)),
    ]);

    let block = Block::default().borders(Borders::ALL).title(title);
    f.render_widget(block.clone(), popup_area);

    let inner = block.inner(popup_area);
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)].as_ref())
        .split(inner);

    let list_area = layout[0];
    let hint_area = layout[1];

    let label_width = 24usize;

    let items: Vec<ListItem> = entries
        .iter()
        .enumerate()
        .take(list_area.height as usize)
        .map(|(idx, entry)| {
            let is_selected = idx == selected;
            let mut value = entry.value.clone();
            if is_selected && entry.kind == ConfigFieldKind::Text {
                if let Some(editing) = editing_value {
                    value = editing.with_cursor_marker('_');
                }
            }
            let label = format!("{:label_width$}", entry.label);

            // Build line with optional warning indicator
            let (line_text, warning_style) = if entry.risk_level == RiskLevel::Danger {
                let warning_icon = "⚠ ";
                let text = format!("{}{} {}", label, value, warning_icon);
                (text, Some(Color::Red))
            } else if entry.risk_level == RiskLevel::Warning {
                let info_icon = "ℹ ";
                let text = format!("{}{} {}", label, value, info_icon);
                (text, Some(Color::Yellow))
            } else {
                let text = format!("{} {}", label, value);
                (text, None)
            };

            let display = truncate_chars(&line_text, list_area.width as usize);

            let mut style = Style::default();
            if entry.value == "(global default)" {
                style = style.fg(Color::DarkGray);
            }
            if let Some(color) = warning_style {
                style = style.fg(color);
            }
            if is_selected {
                style = style.bg(Color::Blue).add_modifier(Modifier::BOLD);
            }

            ListItem::new(Line::from(Span::styled(display, style)))
        })
        .collect();

    let list = List::new(items).block(Block::default());
    f.render_widget(list, list_area);

    let hint = Line::from(vec![
        Span::styled("Enter/Space", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(":edit "),
        Span::styled("x", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(":clear "),
        Span::styled("Esc", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(":close"),
    ]);
    f.render_widget(
        Paragraph::new(hint)
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::DarkGray)),
        hint_area,
    );
}

/// Draw task editor overlay.
pub fn draw_task_editor(
    f: &mut Frame<'_>,
    app: &App,
    area: Rect,
    selected: usize,
    editing_value: Option<&TextInput>,
) {
    let entries = app.task_edit_entries();
    if entries.is_empty() {
        return;
    }

    let popup_width = 96.min(area.width.saturating_sub(4)).max(44);
    let popup_height = (entries.len() as u16 + 7)
        .min(area.height.saturating_sub(4))
        .max(9);

    let popup_area = Rect {
        x: area.x + (area.width.saturating_sub(popup_width)) / 2,
        y: area.y + (area.height.saturating_sub(popup_height)) / 2,
        width: popup_width,
        height: popup_height,
    };

    f.render_widget(Clear, popup_area);

    let title = Line::from(vec![Span::styled(
        "Task Editor",
        Style::default().add_modifier(Modifier::BOLD),
    )]);

    let block = Block::default().borders(Borders::ALL).title(title);
    f.render_widget(block.clone(), popup_area);

    let inner = block.inner(popup_area);
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(2)].as_ref())
        .split(inner);

    let list_area = layout[0];
    let hint_area = layout[1];

    let label_width = 18usize;

    let items: Vec<ListItem> = entries
        .iter()
        .enumerate()
        .take(list_area.height as usize)
        .map(|(idx, entry)| {
            let is_selected = idx == selected;
            let mut value = entry.value.clone();
            if is_selected {
                match entry.kind {
                    TaskEditKind::Cycle => {}
                    TaskEditKind::Text
                    | TaskEditKind::List
                    | TaskEditKind::Map
                    | TaskEditKind::OptionalText => {
                        if let Some(editing) = editing_value {
                            value = editing.with_cursor_marker('_');
                        }
                    }
                }
            }
            let label = format!("{:label_width$}", entry.label);
            let line_text = format!("{} {}", label, value);
            let display = truncate_chars(&line_text, list_area.width as usize);

            let mut style = Style::default();
            if entry.value == "(empty)" {
                style = style.fg(Color::DarkGray);
            }
            if is_selected {
                style = style.bg(Color::Blue).add_modifier(Modifier::BOLD);
            }

            ListItem::new(Line::from(Span::styled(display, style)))
        })
        .collect();

    let list = List::new(items).block(Block::default());
    f.render_widget(list, list_area);

    let hint = Line::from(vec![
        Span::styled("Enter/Space", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(":edit "),
        Span::styled("↑↓", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(":nav "),
        Span::styled("x", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(":clear "),
        Span::styled("Esc", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(":close"),
    ]);
    let format_hint = Line::from(vec![
        Span::styled("lists", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(": a, b, c  "),
        Span::styled("maps", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(": key=value"),
    ]);
    let hint_paragraph = Paragraph::new(ratatui::text::Text::from(vec![hint, format_hint]))
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::DarkGray));
    f.render_widget(hint_paragraph, hint_area);
}
