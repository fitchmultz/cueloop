use super::super::AppMode;
use super::types::TuiAction;
use super::App;
use anyhow::Result;
use crossterm::event::KeyCode;

/// Handle key events in Help mode.
pub(super) fn handle_help_mode_key(app: &mut App, key: KeyCode) -> Result<TuiAction> {
    match key {
        KeyCode::Char('?') | KeyCode::Char('h') | KeyCode::Esc => {
            app.mode = AppMode::Normal;
            Ok(TuiAction::Continue)
        }
        _ => Ok(TuiAction::Continue),
    }
}
