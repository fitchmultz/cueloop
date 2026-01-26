use super::super::AppMode;
use super::types::TuiAction;
use super::App;
use anyhow::Result;
use crossterm::event::KeyCode;

/// Result of handling a text-edit key.
enum TextEditKeyResult {
    Commit(String),
    Cancel,
    Update(String),
    Noop,
}

fn handle_text_edit_key(key: KeyCode, value: String) -> TextEditKeyResult {
    match key {
        KeyCode::Enter => TextEditKeyResult::Commit(value),
        KeyCode::Esc => TextEditKeyResult::Cancel,
        KeyCode::Char(c) => {
            let mut updated = value;
            updated.push(c);
            TextEditKeyResult::Update(updated)
        }
        KeyCode::Backspace => {
            let mut updated = value;
            updated.pop();
            TextEditKeyResult::Update(updated)
        }
        _ => TextEditKeyResult::Noop,
    }
}

/// Handle key events in EditingTask mode.
pub(super) fn handle_editing_task_key(
    app: &mut App,
    key: KeyCode,
    selected: usize,
    editing_value: Option<String>,
    now_rfc3339: &str,
) -> Result<TuiAction> {
    let entries = app.task_edit_entries();
    if entries.is_empty() {
        app.mode = AppMode::Normal;
        app.set_status_message("No task fields available");
        return Ok(TuiAction::Continue);
    }
    let max_index = entries.len().saturating_sub(1);
    let selected = selected.min(max_index);
    let entry = entries[selected].clone();

    if let Some(value) = editing_value {
        match handle_text_edit_key(key, value) {
            TextEditKeyResult::Commit(value) => {
                match app.apply_task_edit(entry.key, &value, now_rfc3339) {
                    Ok(()) => {
                        app.mode = AppMode::EditingTask {
                            selected,
                            editing_value: None,
                        };
                        Ok(TuiAction::Continue)
                    }
                    Err(e) => {
                        app.set_status_message(format!("Error: {}", e));
                        app.mode = AppMode::EditingTask {
                            selected,
                            editing_value: Some(value),
                        };
                        Ok(TuiAction::Continue)
                    }
                }
            }
            TextEditKeyResult::Cancel => {
                app.mode = AppMode::EditingTask {
                    selected,
                    editing_value: None,
                };
                Ok(TuiAction::Continue)
            }
            TextEditKeyResult::Update(value) => {
                app.mode = AppMode::EditingTask {
                    selected,
                    editing_value: Some(value),
                };
                Ok(TuiAction::Continue)
            }
            TextEditKeyResult::Noop => Ok(TuiAction::Continue),
        }
    } else {
        match key {
            KeyCode::Esc => {
                app.mode = AppMode::Normal;
                Ok(TuiAction::Continue)
            }
            KeyCode::Up | KeyCode::Char('k') => {
                let next_selected = selected.saturating_sub(1);
                app.mode = AppMode::EditingTask {
                    selected: next_selected,
                    editing_value: None,
                };
                Ok(TuiAction::Continue)
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let next_selected = (selected + 1).min(max_index);
                app.mode = AppMode::EditingTask {
                    selected: next_selected,
                    editing_value: None,
                };
                Ok(TuiAction::Continue)
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                match entry.kind {
                    crate::tui::TaskEditKind::Cycle => {
                        if let Err(e) = app.apply_task_edit(entry.key, "", now_rfc3339) {
                            app.set_status_message(format!("Error: {}", e));
                        }
                        app.mode = AppMode::EditingTask {
                            selected,
                            editing_value: None,
                        };
                    }
                    crate::tui::TaskEditKind::Text
                    | crate::tui::TaskEditKind::List
                    | crate::tui::TaskEditKind::Map
                    | crate::tui::TaskEditKind::OptionalText => {
                        let current = app.task_value_for_edit(entry.key);
                        app.mode = AppMode::EditingTask {
                            selected,
                            editing_value: Some(current),
                        };
                    }
                }
                Ok(TuiAction::Continue)
            }
            KeyCode::Char('x') => {
                match entry.kind {
                    crate::tui::TaskEditKind::Cycle => {}
                    crate::tui::TaskEditKind::Text
                    | crate::tui::TaskEditKind::List
                    | crate::tui::TaskEditKind::Map
                    | crate::tui::TaskEditKind::OptionalText => {
                        if let Err(e) = app.apply_task_edit(entry.key, "", now_rfc3339) {
                            app.set_status_message(format!("Error: {}", e));
                        }
                    }
                }
                Ok(TuiAction::Continue)
            }
            KeyCode::Char(c) => {
                match entry.kind {
                    crate::tui::TaskEditKind::Text
                    | crate::tui::TaskEditKind::List
                    | crate::tui::TaskEditKind::Map
                    | crate::tui::TaskEditKind::OptionalText => {
                        let mut current = app.task_value_for_edit(entry.key);
                        current.push(c);
                        app.mode = AppMode::EditingTask {
                            selected,
                            editing_value: Some(current),
                        };
                    }
                    crate::tui::TaskEditKind::Cycle => {}
                }
                Ok(TuiAction::Continue)
            }
            _ => Ok(TuiAction::Continue),
        }
    }
}

pub(super) fn handle_editing_config_key(
    app: &mut App,
    key: KeyCode,
    selected: usize,
    editing_value: Option<String>,
) -> Result<TuiAction> {
    let entries = app.config_entries();
    if entries.is_empty() {
        app.mode = AppMode::Normal;
        app.set_status_message("No config fields available");
        return Ok(TuiAction::Continue);
    }
    let max_index = entries.len().saturating_sub(1);
    let selected = selected.min(max_index);
    let entry = entries[selected].clone();

    if let Some(value) = editing_value {
        match handle_text_edit_key(key, value) {
            TextEditKeyResult::Commit(value) => {
                match app.apply_config_text_value(entry.key, &value) {
                    Ok(()) => {
                        app.mode = AppMode::EditingConfig {
                            selected,
                            editing_value: None,
                        };
                        app.set_status_message("Config updated");
                        Ok(TuiAction::Continue)
                    }
                    Err(e) => {
                        app.set_status_message(format!("Error: {}", e));
                        app.mode = AppMode::EditingConfig {
                            selected,
                            editing_value: Some(value),
                        };
                        Ok(TuiAction::Continue)
                    }
                }
            }
            TextEditKeyResult::Cancel => {
                app.mode = AppMode::EditingConfig {
                    selected,
                    editing_value: None,
                };
                Ok(TuiAction::Continue)
            }
            TextEditKeyResult::Update(value) => {
                app.mode = AppMode::EditingConfig {
                    selected,
                    editing_value: Some(value),
                };
                Ok(TuiAction::Continue)
            }
            TextEditKeyResult::Noop => Ok(TuiAction::Continue),
        }
    } else {
        match key {
            KeyCode::Esc => {
                app.mode = AppMode::Normal;
                Ok(TuiAction::Continue)
            }
            KeyCode::Up | KeyCode::Char('k') => {
                let next_selected = selected.saturating_sub(1);
                app.mode = AppMode::EditingConfig {
                    selected: next_selected,
                    editing_value: None,
                };
                Ok(TuiAction::Continue)
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let next_selected = (selected + 1).min(max_index);
                app.mode = AppMode::EditingConfig {
                    selected: next_selected,
                    editing_value: None,
                };
                Ok(TuiAction::Continue)
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                if entry.kind == crate::tui::ConfigFieldKind::Text {
                    let current = app.config_value_for_edit(entry.key);
                    app.mode = AppMode::EditingConfig {
                        selected,
                        editing_value: Some(current),
                    };
                } else {
                    app.cycle_config_value(entry.key);
                    app.set_status_message("Config updated");
                    app.mode = AppMode::EditingConfig {
                        selected,
                        editing_value: None,
                    };
                }
                Ok(TuiAction::Continue)
            }
            KeyCode::Char('x') => {
                app.clear_config_value(entry.key);
                app.set_status_message("Config cleared");
                Ok(TuiAction::Continue)
            }
            KeyCode::Char(c) => {
                if entry.kind == crate::tui::ConfigFieldKind::Text {
                    let mut current = app.config_value_for_edit(entry.key);
                    current.push(c);
                    app.mode = AppMode::EditingConfig {
                        selected,
                        editing_value: Some(current),
                    };
                }
                Ok(TuiAction::Continue)
            }
            _ => Ok(TuiAction::Continue),
        }
    }
}
