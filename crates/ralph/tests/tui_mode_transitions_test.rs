//! Contract tests for TUI mode transitions.
//!
//! Responsibilities:
//! - Verify `AppMode` transitions caused by key events.
//! - Confirm mode changes occur without relying on rendering.
//!
//! Not handled here:
//! - Rendering output or terminal backend integration.
//! - Queue persistence or runner side effects.
//!
//! Invariants/assumptions:
//! - Tests use synthetic key events against in-memory queues.

mod test_support;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ralph::tui::{self, App, AppMode, MultiLineInput, TuiAction};
use test_support::make_test_queue;

fn key_event(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

#[test]
fn test_mode_transition_normal_to_editing() {
    let mut app = App::new(make_test_queue());
    assert_eq!(app.mode, AppMode::Normal);

    let _ = tui::handle_key_event(
        &mut app,
        key_event(KeyCode::Char('e')),
        "2026-01-19T00:00:00Z",
    )
    .unwrap();

    assert!(matches!(app.mode, AppMode::EditingTask { .. }));
}

#[test]
fn test_mode_transition_normal_to_delete() {
    let mut app = App::new(make_test_queue());
    assert_eq!(app.mode, AppMode::Normal);

    let _ = tui::handle_key_event(
        &mut app,
        key_event(KeyCode::Char('d')),
        "2026-01-19T00:00:00Z",
    )
    .unwrap();

    assert_eq!(app.mode, AppMode::ConfirmDelete);
}

#[test]
fn test_mode_transition_normal_to_executing() {
    let mut app = App::new(make_test_queue());
    assert_eq!(app.mode, AppMode::Normal);

    let _ =
        tui::handle_key_event(&mut app, key_event(KeyCode::Enter), "2026-01-19T00:00:00Z").unwrap();

    assert!(matches!(app.mode, AppMode::Executing { .. }));
}

#[test]
fn test_mode_transition_editing_to_list_on_save() {
    let mut app = App::new(make_test_queue());
    app.mode = AppMode::EditingTask {
        selected: 0,
        editing_value: Some(MultiLineInput::new("New Title", false)),
    };

    let _ =
        tui::handle_key_event(&mut app, key_event(KeyCode::Enter), "2026-01-19T00:00:00Z").unwrap();

    assert!(matches!(
        app.mode,
        AppMode::EditingTask {
            selected: 0,
            editing_value: None
        }
    ));
}

#[test]
fn test_mode_transition_editing_to_list_on_cancel() {
    let mut app = App::new(make_test_queue());
    app.mode = AppMode::EditingTask {
        selected: 0,
        editing_value: Some(MultiLineInput::new("New Title", false)),
    };

    let _ =
        tui::handle_key_event(&mut app, key_event(KeyCode::Esc), "2026-01-19T00:00:00Z").unwrap();

    assert!(matches!(
        app.mode,
        AppMode::EditingTask {
            selected: 0,
            editing_value: None
        }
    ));
}

#[test]
fn test_mode_transition_delete_to_normal_on_confirm() {
    let mut app = App::new(make_test_queue());
    app.mode = AppMode::ConfirmDelete;

    let _ = tui::handle_key_event(
        &mut app,
        key_event(KeyCode::Char('y')),
        "2026-01-19T00:00:00Z",
    )
    .unwrap();

    assert_eq!(app.mode, AppMode::Normal);
}

#[test]
fn test_mode_transition_delete_to_normal_on_cancel() {
    let mut app = App::new(make_test_queue());
    app.mode = AppMode::ConfirmDelete;

    let _ = tui::handle_key_event(
        &mut app,
        key_event(KeyCode::Char('n')),
        "2026-01-19T00:00:00Z",
    )
    .unwrap();

    assert_eq!(app.mode, AppMode::Normal);
}

#[test]
fn test_mode_transition_executing_to_normal() {
    let mut app = App::new(make_test_queue());
    app.mode = AppMode::Executing {
        task_id: "RQ-0001".to_string(),
    };

    let _ =
        tui::handle_key_event(&mut app, key_event(KeyCode::Esc), "2026-01-19T00:00:00Z").unwrap();

    assert_eq!(app.mode, AppMode::Normal);
}

#[test]
fn test_enter_key_in_executing_mode_does_not_quit() {
    let mut app = App::new(make_test_queue());
    app.mode = AppMode::Executing {
        task_id: "RQ-0001".to_string(),
    };

    let action =
        tui::handle_key_event(&mut app, key_event(KeyCode::Enter), "2026-01-19T00:00:00Z").unwrap();

    // Should continue, not quit
    assert_eq!(action, TuiAction::Continue);
    assert!(matches!(app.mode, AppMode::Executing { .. }));
}
