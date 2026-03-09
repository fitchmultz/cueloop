//! Shared queue validation data structures.
//!
//! Responsibilities:
//! - Build stable task lookup structures shared across validators.
//! - Expose active-task and all-task views without repeating collection code.
//!
//! Not handled here:
//! - Validation policy or warning generation.
//! - Queue mutation or repair.
//!
//! Invariants/assumptions:
//! - Task IDs are trimmed before use as lookup keys.
//! - The active queue is the only source used for dependency-depth warnings.

use crate::contracts::{QueueFile, Task};
use std::collections::{HashMap, HashSet};

pub(crate) struct TaskCatalog<'a> {
    pub(crate) active_tasks: Vec<&'a Task>,
    pub(crate) tasks: Vec<&'a Task>,
    pub(crate) all_tasks: HashMap<&'a str, &'a Task>,
    pub(crate) all_task_ids: HashSet<&'a str>,
}

impl<'a> TaskCatalog<'a> {
    pub(crate) fn new(active: &'a QueueFile, done: Option<&'a QueueFile>) -> Self {
        let active_tasks: Vec<&Task> = active.tasks.iter().collect();
        let mut tasks = active_tasks.clone();
        if let Some(done_file) = done {
            tasks.extend(done_file.tasks.iter());
        }

        let mut all_tasks = HashMap::new();
        for task in &tasks {
            all_tasks.insert(task.id.trim(), *task);
        }

        let all_task_ids = all_tasks.keys().copied().collect();

        Self {
            active_tasks,
            tasks,
            all_tasks,
            all_task_ids,
        }
    }
}
