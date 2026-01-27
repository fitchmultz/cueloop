//! Filter input key handling for the TUI.
//!
//! Responsibilities:
//! - Accept tag/scope filter input and apply changes to `App`.
//! - Handle submit and cancel flow for filter inputs.
//!
//! Not handled here:
//! - Rendering of filter prompts or validation beyond parsing.
//! - Shortcut handling outside filter modes.
//!
//! Invariants/assumptions:
//! - Input uses cursor-aware `TextInput` edits.
//! - On submit or cancel, the mode returns to Normal.

use super::super::input::{apply_text_input_key, TextInputEdit};
use super::super::{AppMode, TextInput};
use super::types::TuiAction;
use super::App;
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};

/// Handle key events in FilteringTags mode.
pub(super) fn handle_filtering_tags_key(
    app: &mut App,
    key: KeyEvent,
    mut current: TextInput,
) -> Result<TuiAction> {
    match key.code {
        KeyCode::Enter => {
            let tags = App::parse_tags(current.value());
            app.set_tag_filters(tags);
            app.mode = AppMode::Normal;
            Ok(TuiAction::Continue)
        }
        KeyCode::Esc => {
            app.mode = AppMode::Normal;
            Ok(TuiAction::Continue)
        }
        _ => {
            if apply_text_input_key(&mut current, &key) == TextInputEdit::Changed {
                app.mode = AppMode::FilteringTags(current);
            }
            Ok(TuiAction::Continue)
        }
    }
}

pub(super) fn handle_filtering_scopes_key(
    app: &mut App,
    key: KeyEvent,
    mut current: TextInput,
) -> Result<TuiAction> {
    match key.code {
        KeyCode::Enter => {
            let scopes = App::parse_list(current.value());
            app.set_scope_filters(scopes);
            app.mode = AppMode::Normal;
            Ok(TuiAction::Continue)
        }
        KeyCode::Esc => {
            app.mode = AppMode::Normal;
            Ok(TuiAction::Continue)
        }
        _ => {
            if apply_text_input_key(&mut current, &key) == TextInputEdit::Changed {
                app.mode = AppMode::FilteringScopes(current);
            }
            Ok(TuiAction::Continue)
        }
    }
}
