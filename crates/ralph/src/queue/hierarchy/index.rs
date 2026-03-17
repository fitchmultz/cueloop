//! Purpose: Build and query deterministic task-hierarchy indexes across active and done queues.
//!
//! Responsibilities:
//! - Define hierarchy source/reference/index types.
//! - Build stable parent/child lookup structures from queue files.
//! - Expose parent, child, containment, and root-navigation helpers.
//!
//! Scope:
//! - In-memory hierarchy indexing and navigation only.
//!
//! Usage:
//! - Used by hierarchy rendering, CLI tree views, validation, and task-decomposition helpers.
//!
//! Invariants/Assumptions:
//! - Empty or whitespace-only task IDs are ignored.
//! - Empty or whitespace-only parent IDs are treated as unset.
//! - Child ordering follows active-file order first, then done-file order.

use crate::contracts::{QueueFile, Task};
use std::collections::HashMap;

/// Source of a task in the combined active+done set.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TaskSource {
    Active,
    Done,
}

/// Reference to a task with metadata for ordering and source tracking.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TaskRef<'a> {
    pub(crate) task: &'a Task,
    pub(crate) source: TaskSource,
    pub(crate) order: usize,
}

/// Index for efficient parent/child navigation.
#[derive(Debug)]
pub(crate) struct HierarchyIndex<'a> {
    by_id: HashMap<&'a str, TaskRef<'a>>,
    children_by_parent: HashMap<&'a str, Vec<TaskRef<'a>>>,
}

impl<'a> HierarchyIndex<'a> {
    /// Build a hierarchy index from active and optional done queues.
    ///
    /// Ordering: active tasks first (by file order), then done tasks (by file order).
    /// Children within each parent are stored in this deterministic order.
    pub(crate) fn build(active: &'a QueueFile, done: Option<&'a QueueFile>) -> Self {
        let mut by_id: HashMap<&'a str, TaskRef<'a>> = HashMap::new();
        let mut children_by_parent: HashMap<&'a str, Vec<TaskRef<'a>>> = HashMap::new();

        let mut order_counter: usize = 0;

        for task in &active.tasks {
            let id = task.id.trim();
            if id.is_empty() {
                continue;
            }
            let task_ref = TaskRef {
                task,
                source: TaskSource::Active,
                order: order_counter,
            };
            order_counter += 1;
            by_id.insert(id, task_ref);
        }

        if let Some(done_file) = done {
            for task in &done_file.tasks {
                let id = task.id.trim();
                if id.is_empty() {
                    continue;
                }
                let task_ref = TaskRef {
                    task,
                    source: TaskSource::Done,
                    order: order_counter,
                };
                order_counter += 1;
                by_id.insert(id, task_ref);
            }
        }

        for task_ref in by_id.values() {
            let task = task_ref.task;
            if let Some(parent_id) = task.parent_id.as_deref() {
                let parent_id_trimmed = parent_id.trim();
                if parent_id_trimmed.is_empty() {
                    continue;
                }

                if by_id.contains_key(parent_id_trimmed) {
                    children_by_parent
                        .entry(parent_id_trimmed)
                        .or_default()
                        .push(*task_ref);
                }
            }
        }

        for children in children_by_parent.values_mut() {
            children.sort_by_key(|child| child.order);
        }

        Self {
            by_id,
            children_by_parent,
        }
    }

    /// Get a task by ID.
    pub(crate) fn get(&self, id: &str) -> Option<TaskRef<'a>> {
        self.by_id.get(id.trim()).copied()
    }

    /// Get children of a specific parent task.
    pub(crate) fn children_of(&self, parent_id: &str) -> &[TaskRef<'a>] {
        self.children_by_parent
            .get(parent_id.trim())
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    /// Check if a task ID exists in the index.
    pub(crate) fn contains(&self, id: &str) -> bool {
        self.by_id.contains_key(id.trim())
    }

    /// Get all root tasks (tasks with no parent or with orphaned parent references).
    /// Roots are returned in deterministic order (by their order field).
    pub(crate) fn roots(&self) -> Vec<TaskRef<'a>> {
        let mut roots: Vec<TaskRef<'a>> = self
            .by_id
            .values()
            .filter(|task_ref| match task_ref.task.parent_id.as_deref() {
                None => true,
                Some(parent_id) => {
                    let parent_id_trimmed = parent_id.trim();
                    parent_id_trimmed.is_empty() || !self.by_id.contains_key(parent_id_trimmed)
                }
            })
            .copied()
            .collect();

        roots.sort_by_key(|root| root.order);
        roots
    }
}
