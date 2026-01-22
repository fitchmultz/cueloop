//! TUI event handling extracted from `crate::tui`.
//!
//! This module contains all key-event dispatch and per-mode handlers.
//! Public API is preserved via `crate::tui` re-exporting:
//! - `AppMode`
//! - `TuiAction`
//! - `handle_key_event`
//!
//! The interaction model is intentionally user-centric:
//! - `:` opens a command palette (discoverability)
//! - `l` toggles loop mode (auto-run tasks)
//! - `a` archives terminal tasks (done/rejected) with confirmation

use anyhow::Result;
use crossterm::event::KeyCode;

use super::App;

/// Actions that can result from handling a key event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TuiAction {
    /// Continue running the TUI
    Continue,
    /// Exit the TUI
    Quit,
    /// Reload the queue from disk
    ReloadQueue,
    /// Run a specific task (transitions to Executing mode)
    RunTask(String),
}

/// Interaction modes for the TUI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppMode {
    /// Normal navigation mode
    Normal,
    /// Editing task title
    EditingTitle(String),
    /// Creating a new task (title input)
    CreatingTask(String),
    /// Searching tasks (query input)
    Searching(String),
    /// Filtering tasks by tag list (comma-separated input)
    FilteringTags(String),
    /// Command palette (":" style)
    CommandPalette { query: String, selected: usize },
    /// Confirming task deletion
    ConfirmDelete,
    /// Confirming archive of done/rejected tasks
    ConfirmArchive,
    /// Confirming quit while a task is running
    ConfirmQuit,
    /// Executing a task (live output view)
    Executing { task_id: String },
}

/// High-level commands available in the command palette.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaletteCommand {
    RunSelected,
    RunNextRunnable,
    ToggleLoop,
    ArchiveTerminal,
    NewTask,
    EditTitle,
    Search,
    FilterTags,
    ClearFilters,
    CycleStatus,
    CyclePriority,
    ReloadQueue,
    Quit,
}

/// A single palette entry, already filtered and ready to render.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaletteEntry {
    pub cmd: PaletteCommand,
    pub title: String,
}

/// Handle a key event and return the resulting action.
///
/// This function is the core of TUI interaction handling and is public
/// to allow testing without a full terminal setup.
pub fn handle_key_event(app: &mut App, key: KeyCode, now_rfc3339: &str) -> Result<TuiAction> {
    match app.mode.clone() {
        AppMode::Normal => handle_normal_mode_key(app, key, now_rfc3339),
        AppMode::EditingTitle(ref current) => {
            handle_editing_mode_key(app, key, current, now_rfc3339)
        }
        AppMode::CreatingTask(ref current) => {
            handle_creating_mode_key(app, key, current, now_rfc3339)
        }
        AppMode::Searching(ref current) => handle_searching_mode_key(app, key, current),
        AppMode::FilteringTags(ref current) => handle_filtering_tags_key(app, key, current),
        AppMode::CommandPalette { query, selected } => {
            handle_command_palette_key(app, key, &query, selected, now_rfc3339)
        }
        AppMode::ConfirmDelete => handle_confirm_delete_key(app, key),
        AppMode::ConfirmArchive => handle_confirm_archive_key(app, key, now_rfc3339),
        AppMode::ConfirmQuit => handle_confirm_quit_key(app, key),
        AppMode::Executing { .. } => handle_executing_mode_key(app, key),
    }
}

/// Handle key events in Normal mode.
fn handle_normal_mode_key(app: &mut App, key: KeyCode, now_rfc3339: &str) -> Result<TuiAction> {
    match key {
        KeyCode::Char(':') => {
            app.mode = AppMode::CommandPalette {
                query: String::new(),
                selected: 0,
            };
            Ok(TuiAction::Continue)
        }
        KeyCode::Char('q') | KeyCode::Esc => {
            app.execute_palette_command(PaletteCommand::Quit, now_rfc3339)
        }
        KeyCode::Up | KeyCode::Char('k') => {
            app.move_up();
            Ok(TuiAction::Continue)
        }
        KeyCode::Down | KeyCode::Char('j') => {
            let list_height = app.list_height;
            app.move_down(list_height);
            Ok(TuiAction::Continue)
        }
        KeyCode::Enter => app.execute_palette_command(PaletteCommand::RunSelected, now_rfc3339),
        KeyCode::Char('l') => app.execute_palette_command(PaletteCommand::ToggleLoop, now_rfc3339),
        KeyCode::Char('a') => {
            app.execute_palette_command(PaletteCommand::ArchiveTerminal, now_rfc3339)
        }
        KeyCode::Char('d') => {
            if app.selected_task().is_some() {
                app.mode = AppMode::ConfirmDelete;
            }
            Ok(TuiAction::Continue)
        }
        KeyCode::Char('e') => {
            if let Some(task) = app.selected_task() {
                app.mode = AppMode::EditingTitle(task.title.clone());
            }
            Ok(TuiAction::Continue)
        }
        KeyCode::Char('n') => {
            app.mode = AppMode::CreatingTask(String::new());
            Ok(TuiAction::Continue)
        }
        KeyCode::Char('/') => {
            app.mode = AppMode::Searching(app.filters.query.clone());
            Ok(TuiAction::Continue)
        }
        KeyCode::Char('t') => {
            app.mode = AppMode::FilteringTags(app.filters.tags.join(","));
            Ok(TuiAction::Continue)
        }
        KeyCode::Char('f') => {
            app.cycle_status_filter();
            Ok(TuiAction::Continue)
        }
        KeyCode::Char('x') => {
            app.clear_filters();
            app.set_status_message("Filters cleared");
            Ok(TuiAction::Continue)
        }
        KeyCode::Char('s') => app.execute_palette_command(PaletteCommand::CycleStatus, now_rfc3339),
        KeyCode::Char('p') => {
            app.execute_palette_command(PaletteCommand::CyclePriority, now_rfc3339)
        }
        KeyCode::Char('r') => Ok(TuiAction::ReloadQueue),
        _ => Ok(TuiAction::Continue),
    }
}

/// Handle key events in EditingTitle mode.
fn handle_editing_mode_key(
    app: &mut App,
    key: KeyCode,
    current: &str,
    now_rfc3339: &str,
) -> Result<TuiAction> {
    match key {
        KeyCode::Enter => {
            let new_title = current.to_string();
            if let Err(e) = app.update_title(new_title, now_rfc3339) {
                app.set_status_message(format!("Error: {}", e));
            } else {
                app.mode = AppMode::Normal;
            }
            Ok(TuiAction::Continue)
        }
        KeyCode::Esc => {
            app.mode = AppMode::Normal;
            Ok(TuiAction::Continue)
        }
        KeyCode::Char(c) => {
            let mut new_title = current.to_string();
            new_title.push(c);
            app.mode = AppMode::EditingTitle(new_title);
            Ok(TuiAction::Continue)
        }
        KeyCode::Backspace => {
            let mut new_title = current.to_string();
            new_title.pop();
            app.mode = AppMode::EditingTitle(new_title);
            Ok(TuiAction::Continue)
        }
        _ => Ok(TuiAction::Continue),
    }
}

/// Handle key events in CreatingTask mode.
fn handle_creating_mode_key(
    app: &mut App,
    key: KeyCode,
    current: &str,
    now_rfc3339: &str,
) -> Result<TuiAction> {
    match key {
        KeyCode::Enter => {
            if let Err(e) = app.create_task_from_title(current, now_rfc3339) {
                app.set_status_message(format!("Error: {}", e));
            }
            Ok(TuiAction::Continue)
        }
        KeyCode::Esc => {
            app.mode = AppMode::Normal;
            Ok(TuiAction::Continue)
        }
        KeyCode::Char(c) => {
            let mut new_title = current.to_string();
            new_title.push(c);
            app.mode = AppMode::CreatingTask(new_title);
            Ok(TuiAction::Continue)
        }
        KeyCode::Backspace => {
            let mut new_title = current.to_string();
            new_title.pop();
            app.mode = AppMode::CreatingTask(new_title);
            Ok(TuiAction::Continue)
        }
        _ => Ok(TuiAction::Continue),
    }
}

/// Handle key events in Searching mode.
fn handle_searching_mode_key(app: &mut App, key: KeyCode, current: &str) -> Result<TuiAction> {
    match key {
        KeyCode::Enter => {
            app.set_search_query(current.to_string());
            app.mode = AppMode::Normal;
            Ok(TuiAction::Continue)
        }
        KeyCode::Esc => {
            app.mode = AppMode::Normal;
            Ok(TuiAction::Continue)
        }
        KeyCode::Char(c) => {
            let mut next = current.to_string();
            next.push(c);
            app.mode = AppMode::Searching(next);
            Ok(TuiAction::Continue)
        }
        KeyCode::Backspace => {
            let mut next = current.to_string();
            next.pop();
            app.mode = AppMode::Searching(next);
            Ok(TuiAction::Continue)
        }
        _ => Ok(TuiAction::Continue),
    }
}

/// Handle key events in FilteringTags mode.
fn handle_filtering_tags_key(app: &mut App, key: KeyCode, current: &str) -> Result<TuiAction> {
    match key {
        KeyCode::Enter => {
            let tags = App::parse_tags(current);
            app.set_tag_filters(tags);
            app.mode = AppMode::Normal;
            Ok(TuiAction::Continue)
        }
        KeyCode::Esc => {
            app.mode = AppMode::Normal;
            Ok(TuiAction::Continue)
        }
        KeyCode::Char(c) => {
            let mut next = current.to_string();
            next.push(c);
            app.mode = AppMode::FilteringTags(next);
            Ok(TuiAction::Continue)
        }
        KeyCode::Backspace => {
            let mut next = current.to_string();
            next.pop();
            app.mode = AppMode::FilteringTags(next);
            Ok(TuiAction::Continue)
        }
        _ => Ok(TuiAction::Continue),
    }
}

/// Handle key events in CommandPalette mode.
fn handle_command_palette_key(
    app: &mut App,
    key: KeyCode,
    query: &str,
    selected: usize,
    now_rfc3339: &str,
) -> Result<TuiAction> {
    let entries = app.palette_entries(query);
    let max_index = entries.len().saturating_sub(1);
    let selected = selected.min(max_index);

    match key {
        KeyCode::Esc => {
            app.mode = AppMode::Normal;
            Ok(TuiAction::Continue)
        }
        KeyCode::Enter => {
            if let Some(entry) = entries.get(selected) {
                app.mode = AppMode::Normal;
                app.execute_palette_command(entry.cmd, now_rfc3339)
            } else {
                app.mode = AppMode::Normal;
                app.set_status_message("No matching command");
                Ok(TuiAction::Continue)
            }
        }
        KeyCode::Up | KeyCode::Char('k') => {
            let next_selected = selected.saturating_sub(1);
            app.mode = AppMode::CommandPalette {
                query: query.to_string(),
                selected: next_selected,
            };
            Ok(TuiAction::Continue)
        }
        KeyCode::Down | KeyCode::Char('j') => {
            let next_selected = if entries.is_empty() {
                0
            } else {
                (selected + 1).min(max_index)
            };
            app.mode = AppMode::CommandPalette {
                query: query.to_string(),
                selected: next_selected,
            };
            Ok(TuiAction::Continue)
        }
        KeyCode::Char(c) => {
            let mut next = query.to_string();
            next.push(c);
            app.mode = AppMode::CommandPalette {
                query: next,
                selected: 0,
            };
            Ok(TuiAction::Continue)
        }
        KeyCode::Backspace => {
            let mut next = query.to_string();
            next.pop();
            app.mode = AppMode::CommandPalette {
                query: next,
                selected: 0,
            };
            Ok(TuiAction::Continue)
        }
        _ => Ok(TuiAction::Continue),
    }
}

/// Handle key events in ConfirmDelete mode.
fn handle_confirm_delete_key(app: &mut App, key: KeyCode) -> Result<TuiAction> {
    match key {
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            if let Err(e) = app.delete_selected_task() {
                app.set_status_message(format!("Error: {}", e));
            }
            app.mode = AppMode::Normal;
            Ok(TuiAction::Continue)
        }
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
            app.mode = AppMode::Normal;
            Ok(TuiAction::Continue)
        }
        _ => Ok(TuiAction::Continue),
    }
}

/// Handle key events in ConfirmArchive mode.
fn handle_confirm_archive_key(app: &mut App, key: KeyCode, now_rfc3339: &str) -> Result<TuiAction> {
    match key {
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            if let Err(e) = app.archive_terminal_tasks(now_rfc3339) {
                app.set_status_message(format!("Error: {}", e));
            }
            app.mode = AppMode::Normal;
            Ok(TuiAction::Continue)
        }
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
            app.mode = AppMode::Normal;
            Ok(TuiAction::Continue)
        }
        _ => Ok(TuiAction::Continue),
    }
}

/// Handle key events in ConfirmQuit mode.
fn handle_confirm_quit_key(app: &mut App, key: KeyCode) -> Result<TuiAction> {
    match key {
        KeyCode::Char('y') | KeyCode::Char('Y') => Ok(TuiAction::Quit),
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
            app.mode = AppMode::Normal;
            Ok(TuiAction::Continue)
        }
        _ => Ok(TuiAction::Continue),
    }
}

/// Handle key events in Executing mode.
fn handle_executing_mode_key(app: &mut App, key: KeyCode) -> Result<TuiAction> {
    let visible_lines = app.log_visible_lines();
    let page_lines = visible_lines.saturating_sub(1).max(1);
    match key {
        KeyCode::Esc => {
            app.mode = AppMode::Normal;
            Ok(TuiAction::Continue)
        }
        KeyCode::Up | KeyCode::Char('k') => {
            app.scroll_logs_up(1);
            Ok(TuiAction::Continue)
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app.scroll_logs_down(1, visible_lines);
            Ok(TuiAction::Continue)
        }
        KeyCode::PageUp => {
            app.scroll_logs_up(page_lines);
            Ok(TuiAction::Continue)
        }
        KeyCode::PageDown => {
            app.scroll_logs_down(page_lines, visible_lines);
            Ok(TuiAction::Continue)
        }
        KeyCode::Char('a') => {
            if app.autoscroll {
                app.autoscroll = false;
            } else {
                app.enable_autoscroll(visible_lines);
            }
            Ok(TuiAction::Continue)
        }
        KeyCode::Char('l') => {
            if app.loop_active {
                app.loop_active = false;
                app.loop_arm_after_current = false;
                app.set_status_message("Loop stopped");
            }
            Ok(TuiAction::Continue)
        }
        _ => Ok(TuiAction::Continue),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contracts::{QueueFile, Task, TaskPriority, TaskStatus};

    fn make_test_task(id: &str) -> Task {
        Task {
            id: id.to_string(),
            title: "Test task".to_string(),
            status: TaskStatus::Todo,
            priority: TaskPriority::Medium,
            tags: vec![],
            scope: vec![],
            evidence: vec![],
            plan: vec![],
            notes: vec![],
            request: None,
            agent: None,
            created_at: Some("2026-01-19T00:00:00Z".to_string()),
            updated_at: Some("2026-01-19T00:00:00Z".to_string()),
            completed_at: None,
            depends_on: vec![],
            custom_fields: std::collections::HashMap::new(),
        }
    }

    #[test]
    fn quit_when_not_running_exits_immediately() {
        let queue = QueueFile {
            version: 1,
            tasks: vec![make_test_task("RQ-0001")],
        };
        let mut app = App::new(queue);

        let action = handle_key_event(&mut app, KeyCode::Char('q'), "2026-01-19T00:00:00Z")
            .expect("handle key");

        assert_eq!(action, TuiAction::Quit);
        assert_eq!(app.mode, AppMode::Normal);
    }

    #[test]
    fn quit_when_running_requires_confirmation() {
        let queue = QueueFile {
            version: 1,
            tasks: vec![make_test_task("RQ-0001")],
        };
        let mut app = App::new(queue);
        app.runner_active = true;

        let action = handle_key_event(&mut app, KeyCode::Char('q'), "2026-01-19T00:00:00Z")
            .expect("handle key");

        assert_eq!(action, TuiAction::Continue);
        assert_eq!(app.mode, AppMode::ConfirmQuit);
    }

    #[test]
    fn confirm_quit_accepts_yes() {
        let queue = QueueFile {
            version: 1,
            tasks: vec![make_test_task("RQ-0001")],
        };
        let mut app = App::new(queue);
        app.mode = AppMode::ConfirmQuit;

        let action = handle_key_event(&mut app, KeyCode::Char('y'), "2026-01-19T00:00:00Z")
            .expect("handle key");

        assert_eq!(action, TuiAction::Quit);
    }

    #[test]
    fn loop_key_starts_loop_and_runs_next_runnable() {
        let queue = QueueFile {
            version: 1,
            tasks: vec![make_test_task("RQ-0001")],
        };
        let mut app = App::new(queue);

        let action = handle_key_event(&mut app, KeyCode::Char('l'), "2026-01-20T00:00:00Z")
            .expect("handle key");

        assert_eq!(action, TuiAction::RunTask("RQ-0001".to_string()));
        assert!(app.loop_active);
        assert!(app.runner_active);
    }

    #[test]
    fn archive_flow_enters_confirm_mode_then_moves_tasks() {
        let mut done_task = make_test_task("RQ-0001");
        done_task.status = TaskStatus::Done;
        done_task.completed_at = Some("2026-01-19T00:00:00Z".to_string());

        let queue = QueueFile {
            version: 1,
            tasks: vec![done_task, make_test_task("RQ-0002")],
        };
        let mut app = App::new(queue);

        // Enter confirm archive.
        let action = handle_key_event(&mut app, KeyCode::Char('a'), "2026-01-20T00:00:00Z")
            .expect("handle key");
        assert_eq!(action, TuiAction::Continue);
        assert_eq!(app.mode, AppMode::ConfirmArchive);

        // Confirm.
        let action = handle_key_event(&mut app, KeyCode::Char('y'), "2026-01-20T00:00:00Z")
            .expect("handle key");
        assert_eq!(action, TuiAction::Continue);
        assert_eq!(app.mode, AppMode::Normal);

        assert_eq!(app.queue.tasks.len(), 1);
        assert_eq!(app.queue.tasks[0].id, "RQ-0002");
        assert_eq!(app.done.tasks.len(), 1);
        assert_eq!(app.done.tasks[0].id, "RQ-0001");
        assert!(app.dirty);
        assert!(app.dirty_done);
    }

    #[test]
    fn colon_enters_command_palette() {
        let queue = QueueFile {
            version: 1,
            tasks: vec![make_test_task("RQ-0001")],
        };
        let mut app = App::new(queue);

        let action = handle_key_event(&mut app, KeyCode::Char(':'), "2026-01-20T00:00:00Z")
            .expect("handle key");

        assert_eq!(action, TuiAction::Continue);
        match app.mode {
            AppMode::CommandPalette { .. } => {}
            other => panic!("expected command palette, got {:?}", other),
        }
    }

    #[test]
    fn command_palette_runs_selected_command() {
        let queue = QueueFile {
            version: 1,
            tasks: vec![make_test_task("RQ-0001")],
        };
        let mut app = App::new(queue);
        app.mode = AppMode::CommandPalette {
            query: "run selected".to_string(),
            selected: 0,
        };

        let action =
            handle_key_event(&mut app, KeyCode::Enter, "2026-01-20T00:00:00Z").expect("handle key");

        assert_eq!(action, TuiAction::RunTask("RQ-0001".to_string()));
        assert!(app.runner_active);
    }

    #[test]
    fn command_palette_with_no_matches_sets_status_message() {
        let queue = QueueFile {
            version: 1,
            tasks: vec![make_test_task("RQ-0001")],
        };
        let mut app = App::new(queue);
        app.mode = AppMode::CommandPalette {
            query: "nope".to_string(),
            selected: 0,
        };

        let action =
            handle_key_event(&mut app, KeyCode::Enter, "2026-01-20T00:00:00Z").expect("handle key");

        assert_eq!(action, TuiAction::Continue);
        assert_eq!(app.mode, AppMode::Normal);
        assert_eq!(app.status_message.as_deref(), Some("No matching command"));
    }
}
