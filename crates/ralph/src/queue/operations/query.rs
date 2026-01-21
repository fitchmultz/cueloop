//! Query helpers for queue tasks.

use crate::contracts::{QueueFile, Task, TaskStatus};

pub fn find_task<'a>(queue: &'a QueueFile, task_id: &str) -> Option<&'a Task> {
    let needle = task_id.trim();
    if needle.is_empty() {
        return None;
    }
    queue.tasks.iter().find(|task| task.id.trim() == needle)
}

pub fn find_task_across<'a>(
    active: &'a QueueFile,
    done: Option<&'a QueueFile>,
    task_id: &str,
) -> Option<&'a Task> {
    find_task(active, task_id).or_else(|| done.and_then(|d| find_task(d, task_id)))
}

/// Return the first todo task by file order (top-of-file wins).
pub fn next_todo_task(queue: &QueueFile) -> Option<&Task> {
    queue
        .tasks
        .iter()
        .find(|task| task.status == TaskStatus::Todo)
}

/// Check if a task's dependencies are met.
///
/// Dependencies are met if `depends_on` is empty OR all referenced tasks exist and have `status == TaskStatus::Done`.
pub fn are_dependencies_met(task: &Task, active: &QueueFile, done: Option<&QueueFile>) -> bool {
    if task.depends_on.is_empty() {
        return true;
    }

    for dep_id in &task.depends_on {
        let dep_task = find_task_across(active, done, dep_id);
        match dep_task {
            Some(t) => {
                if t.status != TaskStatus::Done {
                    return false;
                }
            }
            None => return false, // Dependency not found means not met
        }
    }

    true
}

/// Return the first runnable task (Todo and dependencies met).
pub fn next_runnable_task<'a>(
    active: &'a QueueFile,
    done: Option<&'a QueueFile>,
) -> Option<&'a Task> {
    active
        .tasks
        .iter()
        .find(|task| task.status == TaskStatus::Todo && are_dependencies_met(task, active, done))
}
