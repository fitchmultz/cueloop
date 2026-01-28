//! Interactive Terminal UI for browsing and managing the task queue.
//!
//! Responsibilities:
//! - Wire TUI modules and expose the public TUI entrypoints/types.
//! - Provide the shared `App` state and configuration used by render/event layers.
//!
//! Not handled here:
//! - CLI argument parsing or command routing (see `crate::cli`).
//! - Rendering and event implementations (see `tui::render` and `tui::events`).
//!
//! Invariants/assumptions:
//! - `App` is the single source of truth for TUI state.
//! - Public exports remain cohesive to the TUI surface area.

mod app;
mod app_filters;
mod app_palette;
mod config_edit;
mod events;
mod help;
mod input;
mod keymap;
mod render;
mod task_edit;
pub mod terminal;

#[cfg(test)]
mod tests;

pub use app::{prepare_tui_session, run_tui, App, FilterState, RunningKind, TuiOptions};
pub use config_edit::{ConfigEntry, ConfigFieldKind, ConfigKey};
pub use events::{
    handle_key_event, AppMode, ConfirmDiscardAction, PaletteCommand, PaletteEntry, TuiAction,
};
pub use input::TextInput;
pub use render::draw_ui;
pub use task_edit::{TaskEditEntry, TaskEditKind};
