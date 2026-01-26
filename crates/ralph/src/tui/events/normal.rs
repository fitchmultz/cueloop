use super::super::AppMode;
use super::types::TuiAction;
use super::App;
use crate::tui::PaletteCommand;
use anyhow::Result;
use crossterm::event::KeyCode;

/// Handle key events in Normal mode.
pub(super) fn handle_normal_mode_key(
    app: &mut App,
    key: KeyCode,
    now_rfc3339: &str,
) -> Result<TuiAction> {
    match key {
        KeyCode::Char('?') | KeyCode::Char('h') => {
            app.mode = AppMode::Help;
            Ok(TuiAction::Continue)
        }
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
            } else {
                app.set_status_message("No task selected");
            }
            Ok(TuiAction::Continue)
        }
        KeyCode::Char('e') => {
            if app.selected_task().is_some() {
                app.mode = AppMode::EditingTask {
                    selected: 0,
                    editing_value: None,
                };
            } else {
                app.set_status_message("No task selected");
            }
            Ok(TuiAction::Continue)
        }
        KeyCode::Char('c') => {
            app.mode = AppMode::EditingConfig {
                selected: 0,
                editing_value: None,
            };
            Ok(TuiAction::Continue)
        }
        KeyCode::Char('C') => {
            app.execute_palette_command(PaletteCommand::ToggleCaseSensitive, now_rfc3339)
        }
        KeyCode::Char('g') => {
            if app.runner_active {
                app.set_status_message("Runner already active");
            } else {
                app.mode = AppMode::Scanning(String::new());
            }
            Ok(TuiAction::Continue)
        }
        KeyCode::Char('n') => {
            app.mode = AppMode::CreatingTask(String::new());
            Ok(TuiAction::Continue)
        }
        KeyCode::Char('N') => {
            if app.runner_active {
                app.set_status_message("Runner already active");
            } else {
                app.mode = AppMode::CreatingTaskDescription(String::new());
            }
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
        KeyCode::Char('o') => {
            app.mode = AppMode::FilteringScopes(app.filters.search_options.scopes.join(","));
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
        KeyCode::Char('R') => app.execute_palette_command(PaletteCommand::ToggleRegex, now_rfc3339),
        _ => Ok(TuiAction::Continue),
    }
}
