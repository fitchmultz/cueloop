//! Purpose: Detect and normalize parent-reference cycles in task hierarchies.
//!
//! Responsibilities:
//! - Follow `parent_id` chains to find cycles.
//! - Normalize detected cycles for deduplication.
//! - Preserve the existing cycle-reporting behavior used by validation and CLI rendering.
//!
//! Scope:
//! - Cycle detection and normalization only.
//!
//! Usage:
//! - Called by queue validation and hierarchy-aware CLI commands before or alongside rendering.
//!
//! Invariants/Assumptions:
//! - Empty or whitespace-only task IDs are ignored.
//! - Empty or whitespace-only parent IDs are treated as unset.
//! - Equivalent cycles are deduplicated by normalized rotation.

use crate::contracts::Task;
use std::collections::{HashMap, HashSet};

/// Detect cycles in the parent_id chain.
/// Returns a list of task IDs involved in cycles.
pub(crate) fn detect_parent_cycles(all_tasks: &[&Task]) -> Vec<Vec<String>> {
    let mut child_to_parent: HashMap<&str, &str> = HashMap::new();

    for task in all_tasks {
        let task_id = task.id.trim();
        if task_id.is_empty() {
            continue;
        }
        if let Some(parent_id) = task.parent_id.as_deref() {
            let parent_id_trimmed = parent_id.trim();
            if !parent_id_trimmed.is_empty() {
                child_to_parent.insert(task_id, parent_id_trimmed);
            }
        }
    }

    let mut visited: HashSet<&str> = HashSet::new();
    let mut cycles: Vec<Vec<String>> = Vec::new();

    for &start_task in all_tasks {
        let start_id = start_task.id.trim();
        if start_id.is_empty() || visited.contains(start_id) {
            continue;
        }

        let mut path: Vec<&str> = Vec::new();
        let mut current = Some(start_id);
        let mut path_set: HashSet<&str> = HashSet::new();

        while let Some(node) = current {
            if path_set.contains(node) {
                let cycle_start = path
                    .iter()
                    .position(|&candidate| candidate == node)
                    .unwrap_or(0);
                let cycle: Vec<String> = path[cycle_start..]
                    .iter()
                    .map(|&task_id| task_id.to_string())
                    .collect();

                let cycle_normalized = normalize_cycle(&cycle);
                if !cycles
                    .iter()
                    .any(|existing| normalize_cycle(existing) == cycle_normalized)
                {
                    cycles.push(cycle);
                }
                break;
            }

            if visited.contains(node) {
                break;
            }

            path.push(node);
            path_set.insert(node);
            current = child_to_parent.get(node).copied();
        }

        for &node in &path {
            visited.insert(node);
        }
    }

    cycles
}

fn normalize_cycle(cycle: &[String]) -> Vec<String> {
    if cycle.is_empty() {
        return cycle.to_vec();
    }

    let min_idx = cycle
        .iter()
        .enumerate()
        .min_by_key(|(_, value)| *value)
        .map(|(index, _)| index)
        .unwrap_or(0);

    let mut normalized: Vec<String> = cycle[min_idx..].to_vec();
    normalized.extend_from_slice(&cycle[..min_idx]);
    normalized
}
