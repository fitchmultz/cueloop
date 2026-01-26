use super::super::AppMode;
use super::types::TuiAction;
use super::App;
use anyhow::Result;
use crossterm::event::KeyCode;

/// Handle key events in Searching mode.
pub(super) fn handle_searching_mode_key(
    app: &mut App,
    key: KeyCode,
    current: &str,
) -> Result<TuiAction> {
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
