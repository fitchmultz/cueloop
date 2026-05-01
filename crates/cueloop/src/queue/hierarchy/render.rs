//! Purpose: Render task hierarchies as deterministic ASCII trees.
//!
//! Responsibilities:
//! - Traverse hierarchy indexes with caller-provided line formatting.
//! - Prevent infinite recursion when cycles exist.
//! - Preserve orphan reporting and done-task filtering semantics.
//!
//! Scope:
//! - Hierarchy rendering and traversal-state management only.
//!
//! Usage:
//! - Called by CLI task/queue commands to render parent-child relationships.
//!
//! Invariants/Assumptions:
//! - Traversal order is deterministic because `HierarchyIndex` preserves file order.
//! - Revisited tasks are rendered once more as cycle markers and are not descended into again.
//! - Missing root references are skipped rather than treated as hard errors.

use super::index::{HierarchyIndex, TaskSource};
use crate::contracts::Task;
use std::collections::HashSet;

struct RenderCtx<'idx, 'task, 'state, F>
where
    F: Fn(&Task, usize, bool, Option<&str>) -> String,
{
    idx: &'idx HierarchyIndex<'task>,
    max_depth: usize,
    include_done: bool,
    visited: &'state mut HashSet<String>,
    output: &'state mut String,
    format_line: &'state F,
}

fn render_tree_recursive<'idx, 'task, 'state, F>(
    ctx: &mut RenderCtx<'idx, 'task, 'state, F>,
    task_id: &str,
    depth: usize,
) where
    F: Fn(&Task, usize, bool, Option<&str>) -> String,
{
    if depth > ctx.max_depth {
        return;
    }

    let trimmed_id = task_id.trim();
    if ctx.visited.contains(trimmed_id) {
        if let Some(task_ref) = ctx.idx.get(trimmed_id) {
            let line = (ctx.format_line)(task_ref.task, depth, true, None);
            ctx.output.push_str(&line);
            ctx.output.push('\n');
        }
        return;
    }

    let task_ref = match ctx.idx.get(trimmed_id) {
        Some(task_ref) => task_ref,
        None => return,
    };

    if !ctx.include_done && matches!(task_ref.source, TaskSource::Done) {
        return;
    }

    ctx.visited.insert(trimmed_id.to_string());

    let is_orphan = task_ref
        .task
        .parent_id
        .as_deref()
        .map(|parent_id| !parent_id.trim().is_empty() && !ctx.idx.contains(parent_id))
        .unwrap_or(false);

    let orphan_parent = if is_orphan {
        task_ref.task.parent_id.as_deref()
    } else {
        None
    };

    let line = (ctx.format_line)(task_ref.task, depth, false, orphan_parent);
    ctx.output.push_str(&line);
    ctx.output.push('\n');

    for child in ctx.idx.children_of(trimmed_id) {
        render_tree_recursive(ctx, &child.task.id, depth + 1);
    }
}

/// Render a task hierarchy tree as ASCII art.
///
/// Arguments:
/// - `idx`: The hierarchy index
/// - `roots`: Root task IDs to start rendering from
/// - `max_depth`: Maximum depth to render
/// - `include_done`: Whether to include done tasks in output
/// - `format_line`: Closure to format a task line
pub(crate) fn render_tree<F>(
    idx: &HierarchyIndex<'_>,
    roots: &[&str],
    max_depth: usize,
    include_done: bool,
    format_line: F,
) -> String
where
    F: Fn(&Task, usize, bool, Option<&str>) -> String,
{
    let mut output = String::new();
    let mut visited: HashSet<String> = HashSet::new();
    let mut ctx = RenderCtx {
        idx,
        max_depth,
        include_done,
        visited: &mut visited,
        output: &mut output,
        format_line: &format_line,
    };

    for &root_id in roots {
        render_tree_recursive(&mut ctx, root_id, 0);
    }

    output
}
