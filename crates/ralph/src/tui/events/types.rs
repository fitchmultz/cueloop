use std::sync::mpsc;

use crate::runutil::RevertDecision;

/// Actions that can result from handling a key event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TuiAction {
    /// Continue running the TUI
    Continue,
    /// Exit the TUI
    Quit,
    /// Reload the queue from disk
    ReloadQueue,
    /// Run a scan with the provided focus string.
    RunScan(String),
    /// Run a specific task (transitions to Executing mode)
    RunTask(String),
    /// Trigger task builder agent with the given description
    BuildTask(String),
}

/// Interaction modes for the TUI.
#[derive(Debug, Clone)]
pub enum AppMode {
    /// Normal navigation mode
    Normal,
    /// Full-screen help overlay
    Help,
    /// Editing task fields
    EditingTask {
        selected: usize,
        editing_value: Option<String>,
    },
    /// Creating a new task (title input)
    CreatingTask(String),
    /// Creating a new task via task builder agent (description input)
    CreatingTaskDescription(String),
    /// Searching tasks (query input)
    Searching(String),
    /// Filtering tasks by tag list (comma-separated input)
    FilteringTags(String),
    /// Filtering tasks by scope list (comma-separated input)
    FilteringScopes(String),
    /// Editing project configuration
    EditingConfig {
        selected: usize,
        editing_value: Option<String>,
    },
    /// Running a scan (focus input)
    Scanning(String),
    /// Command palette (":" style)
    CommandPalette { query: String, selected: usize },
    /// Confirming task deletion
    ConfirmDelete,
    /// Confirming archive of done/rejected tasks
    ConfirmArchive,
    /// Confirming quit while a task is running
    ConfirmQuit,
    /// Confirming revert of uncommitted changes.
    ConfirmRevert {
        label: String,
        allow_proceed: bool,
        selected: usize,
        input: String,
        reply_sender: mpsc::Sender<RevertDecision>,
        previous_mode: Box<AppMode>,
    },
    /// Executing a task (live output view)
    Executing { task_id: String },
}

impl PartialEq for AppMode {
    fn eq(&self, other: &Self) -> bool {
        use AppMode::*;
        match (self, other) {
            (Normal, Normal) => true,
            (Help, Help) => true,
            (
                EditingTask {
                    selected: left_selected,
                    editing_value: left_value,
                },
                EditingTask {
                    selected: right_selected,
                    editing_value: right_value,
                },
            ) => left_selected == right_selected && left_value == right_value,
            (CreatingTask(left), CreatingTask(right)) => left == right,
            (CreatingTaskDescription(left), CreatingTaskDescription(right)) => left == right,
            (Searching(left), Searching(right)) => left == right,
            (FilteringTags(left), FilteringTags(right)) => left == right,
            (FilteringScopes(left), FilteringScopes(right)) => left == right,
            (
                EditingConfig {
                    selected: left_selected,
                    editing_value: left_value,
                },
                EditingConfig {
                    selected: right_selected,
                    editing_value: right_value,
                },
            ) => left_selected == right_selected && left_value == right_value,
            (Scanning(left), Scanning(right)) => left == right,
            (
                CommandPalette {
                    query: left_query,
                    selected: left_selected,
                },
                CommandPalette {
                    query: right_query,
                    selected: right_selected,
                },
            ) => left_query == right_query && left_selected == right_selected,
            (ConfirmDelete, ConfirmDelete) => true,
            (ConfirmArchive, ConfirmArchive) => true,
            (ConfirmQuit, ConfirmQuit) => true,
            (
                ConfirmRevert {
                    label: left_label,
                    allow_proceed: left_allow_proceed,
                    selected: left_selected,
                    input: left_input,
                    previous_mode: left_previous,
                    ..
                },
                ConfirmRevert {
                    label: right_label,
                    allow_proceed: right_allow_proceed,
                    selected: right_selected,
                    input: right_input,
                    previous_mode: right_previous,
                    ..
                },
            ) => {
                left_label == right_label
                    && left_allow_proceed == right_allow_proceed
                    && left_selected == right_selected
                    && left_input == right_input
                    && left_previous == right_previous
            }
            (Executing { task_id: left_id }, Executing { task_id: right_id }) => {
                left_id == right_id
            }
            _ => false,
        }
    }
}

impl Eq for AppMode {}
