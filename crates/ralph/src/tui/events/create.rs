use super::super::AppMode;
use super::types::TuiAction;
use super::App;
use anyhow::Result;
use crossterm::event::KeyCode;

/// Handle key events in CreatingTask mode.
pub(super) fn handle_creating_mode_key(
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

/// Handle key events in CreatingTaskDescription mode.
pub(super) fn handle_creating_description_mode_key(
    app: &mut App,
    key: KeyCode,
    current: &str,
) -> Result<TuiAction> {
    match key {
        KeyCode::Enter => {
            let description = current.trim().to_string();
            if description.is_empty() {
                app.mode = AppMode::Normal;
                app.set_status_message("Description cannot be empty");
                return Ok(TuiAction::Continue);
            }
            app.mode = AppMode::Normal;
            Ok(TuiAction::BuildTask(description))
        }
        KeyCode::Esc => {
            app.mode = AppMode::Normal;
            Ok(TuiAction::Continue)
        }
        KeyCode::Char(c) => {
            let mut new_description = current.to_string();
            new_description.push(c);
            app.mode = AppMode::CreatingTaskDescription(new_description);
            Ok(TuiAction::Continue)
        }
        KeyCode::Backspace => {
            let mut new_description = current.to_string();
            new_description.pop();
            app.mode = AppMode::CreatingTaskDescription(new_description);
            Ok(TuiAction::Continue)
        }
        _ => Ok(TuiAction::Continue),
    }
}
