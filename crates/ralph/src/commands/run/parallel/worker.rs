//! Worker lifecycle facade for parallel task execution.
//!
//! Responsibilities:
//! - Re-export task selection, worker command construction, and process helpers.
//! - Keep implementation modules focused while preserving the existing worker API.
//!
//! Does not handle:
//! - Parallel orchestration loop control.
//! - Persistent worker state serialization.

#[path = "worker_command.rs"]
mod command;
#[path = "worker_process.rs"]
mod process;
#[path = "worker_selection.rs"]
mod selection;

#[cfg(test)]
pub(crate) use command::build_worker_command;
pub(crate) use process::{WorkerState, spawn_worker, terminate_workers};
pub(crate) use selection::{collect_excluded_ids, select_next_task_locked};

#[cfg(test)]
#[path = "worker_tests.rs"]
mod tests;
