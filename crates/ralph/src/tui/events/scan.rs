use super::super::AppMode;
use super::types::TuiAction;
use super::App;
use anyhow::Result;
use crossterm::event::KeyCode;

/// Handle key events in Scanning mode.
pub(super) fn handle_scanning_mode_key(
    app: &mut App,
    key: KeyCode,
    current: &str,
) -> Result<TuiAction> {
    match key {
        KeyCode::Enter => {
            if app.runner_active {
                app.set_status_message("Runner already active");
                return Ok(TuiAction::Continue);
            }
            let focus = current.trim().to_string();
            app.mode = AppMode::Normal;
            Ok(TuiAction::RunScan(focus))
        }
        KeyCode::Esc => {
            app.mode = AppMode::Normal;
            Ok(TuiAction::Continue)
        }
        KeyCode::Char(c) => {
            let mut next = current.to_string();
            next.push(c);
            app.mode = AppMode::Scanning(next);
            Ok(TuiAction::Continue)
        }
        KeyCode::Backspace => {
            let mut next = current.to_string();
            next.pop();
            app.mode = AppMode::Scanning(next);
            Ok(TuiAction::Continue)
        }
        _ => Ok(TuiAction::Continue),
    }
}
