//! Search input key handling for the TUI.
//!
//! Responsibilities:
//! - Capture search query input and apply it to filters.
//! - Exit search mode on submit or cancel.
//!
//! Not handled here:
//! - Rendering search UI.
//! - Regex validation or search execution details.
//!
//! Invariants/assumptions:
//! - Search input uses cursor-aware `TextInput` updates.

use super::super::input::{apply_text_input_key, TextInputEdit};
use super::super::{AppMode, TextInput};
use super::types::TuiAction;
use super::App;
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};

/// Handle key events in Searching mode.
pub(super) fn handle_searching_mode_key(
    app: &mut App,
    key: KeyEvent,
    mut current: TextInput,
) -> Result<TuiAction> {
    match key.code {
        KeyCode::Enter => {
            app.set_search_query(current.value().to_string());
            app.mode = AppMode::Normal;
            Ok(TuiAction::Continue)
        }
        KeyCode::Esc => {
            app.mode = AppMode::Normal;
            Ok(TuiAction::Continue)
        }
        _ => {
            if apply_text_input_key(&mut current, &key) == TextInputEdit::Changed {
                app.mode = AppMode::Searching(current);
            }
            Ok(TuiAction::Continue)
        }
    }
}
