use super::super::AppMode;
use super::types::TuiAction;
use super::App;
use anyhow::Result;
use crossterm::event::KeyCode;

/// Handle key events in FilteringTags mode.
pub(super) fn handle_filtering_tags_key(
    app: &mut App,
    key: KeyCode,
    current: &str,
) -> Result<TuiAction> {
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

pub(super) fn handle_filtering_scopes_key(
    app: &mut App,
    key: KeyCode,
    current: &str,
) -> Result<TuiAction> {
    match key {
        KeyCode::Enter => {
            let scopes = App::parse_list(current);
            app.set_scope_filters(scopes);
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
            app.mode = AppMode::FilteringScopes(next);
            Ok(TuiAction::Continue)
        }
        KeyCode::Backspace => {
            let mut next = current.to_string();
            next.pop();
            app.mode = AppMode::FilteringScopes(next);
            Ok(TuiAction::Continue)
        }
        _ => Ok(TuiAction::Continue),
    }
}
