use super::types::TuiAction;
use super::App;
use anyhow::Result;
use crossterm::event::KeyCode;

/// Handle key events in Executing mode.
pub(super) fn handle_executing_mode_key(app: &mut App, key: KeyCode) -> Result<TuiAction> {
    let visible_lines = app.log_visible_lines();
    let page_lines = visible_lines.saturating_sub(1).max(1);
    match key {
        KeyCode::Esc => {
            app.mode = super::super::AppMode::Normal;
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
