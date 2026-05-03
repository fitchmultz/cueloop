//! Queue dependency graph validation.
//!
//! Purpose:
//! - Queue dependency graph validation.
//!
//! Responsibilities:
//! - Validate `depends_on` relationships and build the dependency graph.
//! - Detect dependency cycles, depth-limit warnings, and terminal-bad dependencies.
//!
//! Not handled here:
//! - Non-dependency relationship fields (`blocks`, `relates_to`, `duplicates`).
//! - Parent hierarchy validation.
//!
//!
//! Usage:
//! - Used through the crate module tree or integration test harness.
//!
//! Invariants/assumptions:
//! - All task lookups are resolved through `TaskCatalog`.
//! - Only active tasks receive dependency-depth warnings.

use super::{DependencyValidationResult, ValidationWarning, shared::TaskCatalog};
use crate::contracts::{Task, TaskStatus};
use anyhow::{Result, bail};
use std::collections::{HashMap, HashSet};

pub(crate) fn validate_dependency_graph(
    catalog: &TaskCatalog<'_>,
    max_dependency_depth: u8,
    result: &mut DependencyValidationResult,
) -> Result<()> {
    let graph = build_dependency_graph(catalog, result)?;
    ensure_acyclic(&graph)?;
    warn_on_dependency_depth(catalog.active_tasks(), &graph, max_dependency_depth, result);
    Ok(())
}

fn build_dependency_graph<'a>(
    catalog: &'a TaskCatalog<'a>,
    result: &mut DependencyValidationResult,
) -> Result<HashMap<&'a str, Vec<&'a str>>> {
    let mut graph: HashMap<&str, Vec<&str>> = HashMap::new();

    for task in &catalog.tasks {
        let task_id = task.id.trim();
        for dep_id in &task.depends_on {
            let dep_id = dep_id.trim();
            if dep_id.is_empty() {
                continue;
            }

            if dep_id == task_id {
                bail!(
                    "Self-dependency detected: task {} depends on itself. Remove the self-reference from the depends_on field.",
                    task_id
                );
            }

            if !catalog.all_task_ids.contains(dep_id) {
                bail!(
                    "Invalid dependency: task {} depends on non-existent task {}. Ensure the dependency task ID exists in .cueloop/queue.jsonc or .cueloop/done.jsonc.",
                    task_id,
                    dep_id
                );
            }

            if let Some(dep_task) = catalog.all_tasks.get(dep_id)
                && dep_task.status == TaskStatus::Rejected
            {
                result.warnings.push(ValidationWarning {
                    task_id: task_id.to_string(),
                    message: format!(
                        "Task {} depends on rejected task {}. This dependency will never be satisfied.",
                        task_id, dep_id
                    ),
                });
            }

            graph.entry(task_id).or_default().push(dep_id);
        }
    }

    Ok(graph)
}

fn ensure_acyclic(graph: &HashMap<&str, Vec<&str>>) -> Result<()> {
    let mut visited = HashSet::new();
    let mut recursion_stack = HashSet::new();

    for node in graph.keys() {
        if has_cycle(node, graph, &mut visited, &mut recursion_stack) {
            bail!(
                "Circular dependency detected involving task {}. Task dependencies must form a DAG (no cycles). Review the depends_on fields to break the cycle.",
                node
            );
        }
    }

    Ok(())
}

fn warn_on_dependency_depth(
    active_tasks: &[&Task],
    graph: &HashMap<&str, Vec<&str>>,
    max_dependency_depth: u8,
    result: &mut DependencyValidationResult,
) {
    let mut depth_cache = HashMap::new();
    for task in active_tasks {
        let task_id = task.id.trim();
        let depth = calculate_dependency_depth(task_id, graph, &mut depth_cache);
        if depth > max_dependency_depth as usize {
            result.warnings.push(ValidationWarning {
                task_id: task_id.to_string(),
                message: format!(
                    "Task {} has a dependency chain depth of {}, which exceeds the configured maximum of {}. This may indicate overly complex dependencies.",
                    task_id, depth, max_dependency_depth
                ),
            });
        }
    }
}

fn calculate_dependency_depth(
    task_id: &str,
    graph: &HashMap<&str, Vec<&str>>,
    cache: &mut HashMap<String, usize>,
) -> usize {
    if let Some(&depth) = cache.get(task_id) {
        return depth;
    }

    let depth = match graph.get(task_id) {
        Some(deps) if !deps.is_empty() => {
            1 + deps
                .iter()
                .map(|dep| calculate_dependency_depth(dep, graph, cache))
                .max()
                .unwrap_or(0)
        }
        _ => 0,
    };

    cache.insert(task_id.to_string(), depth);
    depth
}

fn has_cycle(
    node: &str,
    graph: &HashMap<&str, Vec<&str>>,
    visited: &mut HashSet<String>,
    recursion_stack: &mut HashSet<String>,
) -> bool {
    let key = node.to_string();
    visited.insert(key.clone());
    recursion_stack.insert(key.clone());

    if let Some(neighbors) = graph.get(node) {
        for neighbor in neighbors {
            if !visited.contains(*neighbor) {
                if has_cycle(neighbor, graph, visited, recursion_stack) {
                    return true;
                }
            } else if recursion_stack.contains(*neighbor) {
                return true;
            }
        }
    }

    recursion_stack.remove(&key);
    false
}
