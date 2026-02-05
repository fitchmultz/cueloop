//! Batch task operations for efficient multi-task updates.
//!
//! Responsibilities:
//! - Apply operations to multiple tasks atomically or with partial success.
//! - Filter tasks by tags, status, priority, scope, and age for batch selection.
//! - Batch delete, archive, clone, split, and plan operations.
//! - Provide detailed progress and error reporting.
//!
//! Does not handle:
//! - CLI argument parsing or user interaction.
//! - Individual task validation beyond what's in the single-task operations.
//! - Persistence to disk (callers save the queue after batch operations).
//!
//! Assumptions/invariants:
//! - Callers provide a loaded QueueFile and valid RFC3339 timestamp.
//! - Tag filtering is case-insensitive and OR-based (any tag matches).
//! - Status/priority/scope filters use OR logic within each filter type.
//! - Task IDs are unique within the queue.

use crate::contracts::{QueueFile, Task, TaskPriority, TaskStatus};
use crate::queue;
use crate::queue::TaskEditKey;
use crate::queue::operations::{CloneTaskOptions, SplitTaskOptions};
use anyhow::{Result, bail};

/// Result of a batch operation on a single task.
#[derive(Debug, Clone)]
pub struct BatchTaskResult {
    pub task_id: String,
    pub success: bool,
    pub error: Option<String>,
    pub created_task_ids: Vec<String>,
}

/// Overall result of a batch operation.
#[derive(Debug, Clone)]
pub struct BatchOperationResult {
    pub total: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub results: Vec<BatchTaskResult>,
}

impl BatchOperationResult {
    pub fn all_succeeded(&self) -> bool {
        self.failed == 0
    }

    pub fn has_failures(&self) -> bool {
        self.failed > 0
    }
}

/// Filters for batch task selection.
#[derive(Debug, Clone, Default)]
pub struct BatchTaskFilters {
    pub status_filter: Vec<TaskStatus>,
    pub priority_filter: Vec<TaskPriority>,
    pub scope_filter: Vec<String>,
    pub older_than: Option<String>,
}

/// Filter tasks by tags (case-insensitive, OR-based).
///
/// Returns tasks where ANY of the task's tags match ANY of the filter tags (case-insensitive).
pub fn filter_tasks_by_tags<'a>(queue: &'a QueueFile, tags: &[String]) -> Vec<&'a Task> {
    if tags.is_empty() {
        return Vec::new();
    }

    let normalized_filter_tags: Vec<String> = tags
        .iter()
        .map(|t| t.trim().to_lowercase())
        .filter(|t| !t.is_empty())
        .collect();

    queue
        .tasks
        .iter()
        .filter(|task| {
            task.tags.iter().any(|task_tag| {
                let normalized_task_tag = task_tag.trim().to_lowercase();
                normalized_filter_tags
                    .iter()
                    .any(|filter_tag| filter_tag == &normalized_task_tag)
            })
        })
        .collect()
}

/// Collect unique task IDs from a list of tasks.
pub fn collect_task_ids(tasks: &[&Task]) -> Vec<String> {
    tasks.iter().map(|t| t.id.clone()).collect()
}

/// Validate that all task IDs exist in the queue.
///
/// Returns an error if any task ID is not found.
fn validate_task_ids_exist(queue: &QueueFile, task_ids: &[String]) -> Result<()> {
    for task_id in task_ids {
        let needle = task_id.trim();
        if needle.is_empty() {
            bail!("Empty task ID provided");
        }
        if !queue.tasks.iter().any(|t| t.id.trim() == needle) {
            bail!("Task not found: {}", needle);
        }
    }
    Ok(())
}

/// Deduplicate task IDs while preserving order.
pub(crate) fn deduplicate_task_ids(task_ids: &[String]) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut result = Vec::new();
    for id in task_ids {
        let trimmed = id.trim().to_string();
        if !trimmed.is_empty() && seen.insert(trimmed.clone()) {
            result.push(trimmed);
        }
    }
    result
}

/// Parse an "older than" specification into an RFC3339 cutoff.
///
/// Supports:
/// - Duration expressions: "7d", "1w" (weeks), "30d" (days)
/// - Date: "2026-01-01"
/// - RFC3339: "2026-01-01T00:00:00Z"
pub fn parse_older_than_cutoff(now_rfc3339: &str, spec: &str) -> Result<String> {
    let trimmed = spec.trim();
    if trimmed.is_empty() {
        bail!("Empty older_than specification");
    }

    let lower = trimmed.to_lowercase();

    // Try to parse as RFC3339 first
    if let Ok(dt) = crate::timeutil::parse_rfc3339(trimmed) {
        return crate::timeutil::format_rfc3339(dt);
    }

    // Try duration patterns like "7d", "1w"
    if let Some(days) = lower.strip_suffix('d') {
        let num_days: i64 = days
            .parse()
            .map_err(|_| anyhow::anyhow!("Invalid days in older_than: {}", spec))?;
        let now = crate::timeutil::parse_rfc3339(now_rfc3339)?;
        let cutoff = now - time::Duration::days(num_days);
        return crate::timeutil::format_rfc3339(cutoff);
    }

    if let Some(weeks) = lower.strip_suffix('w') {
        let num_weeks: i64 = weeks
            .parse()
            .map_err(|_| anyhow::anyhow!("Invalid weeks in older_than: {}", spec))?;
        let now = crate::timeutil::parse_rfc3339(now_rfc3339)?;
        let cutoff = now - time::Duration::weeks(num_weeks);
        return crate::timeutil::format_rfc3339(cutoff);
    }

    // Try date-only format "YYYY-MM-DD"
    if lower.len() == 10 && lower.contains('-') {
        let date_str = format!("{}T00:00:00Z", lower);
        if let Ok(dt) = crate::timeutil::parse_rfc3339(&date_str) {
            return crate::timeutil::format_rfc3339(dt);
        }
    }

    bail!(
        "Unable to parse older_than: '{}'. Supported formats: '7d', '1w', '2026-01-01', RFC3339",
        spec
    )
}

/// Resolve task IDs from explicit list or tag filter, then apply additional filters.
///
/// If `tag_filter` is provided, returns tasks matching any of the tags.
/// Otherwise, returns the explicit task IDs (after deduplication).
/// Then applies status, priority, scope, and older_than filters if provided.
///
/// # Arguments
/// * `queue` - The queue file to search
/// * `task_ids` - Explicit list of task IDs
/// * `tag_filter` - Optional list of tags to filter by
/// * `filters` - Additional filters to apply
/// * `now_rfc3339` - Current timestamp for age-based filtering
///
/// # Returns
/// A deduplicated list of task IDs to operate on.
pub fn resolve_task_ids_filtered(
    queue: &QueueFile,
    task_ids: &[String],
    tag_filter: &[String],
    filters: &BatchTaskFilters,
    now_rfc3339: &str,
) -> Result<Vec<String>> {
    // Check if any selection criteria is provided
    let has_task_ids = !task_ids.is_empty();
    let has_tag_filter = !tag_filter.is_empty();
    let has_other_filters = !filters.status_filter.is_empty()
        || !filters.priority_filter.is_empty()
        || !filters.scope_filter.is_empty()
        || filters.older_than.is_some();

    if !has_task_ids && !has_tag_filter && !has_other_filters {
        bail!(
            "No tasks specified. Provide task IDs, use --tag-filter, or use other filters like --status-filter, --priority-filter, --scope-filter, or --older-than."
        );
    }

    // First resolve base IDs via existing logic
    let base_ids = if has_tag_filter {
        let matching_tasks = filter_tasks_by_tags(queue, tag_filter);
        if matching_tasks.is_empty() {
            let tags_str = tag_filter.join(", ");
            bail!("No tasks found with tags: {}", tags_str);
        }
        collect_task_ids(&matching_tasks)
    } else if has_task_ids {
        deduplicate_task_ids(task_ids)
    } else {
        // No tag filter and no explicit IDs - use all tasks from the queue
        // (other filters will be applied below)
        queue.tasks.iter().map(|t| t.id.clone()).collect()
    };

    // If no additional filters, return early
    if filters.status_filter.is_empty()
        && filters.priority_filter.is_empty()
        && filters.scope_filter.is_empty()
        && filters.older_than.is_none()
    {
        return Ok(base_ids);
    }

    // Apply additional filters
    let cutoff = filters
        .older_than
        .as_ref()
        .map(|spec| parse_older_than_cutoff(now_rfc3339, spec))
        .transpose()?;

    let normalized_scope_filters: Vec<String> = filters
        .scope_filter
        .iter()
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty())
        .collect();

    let filtered: Vec<String> = base_ids
        .into_iter()
        .filter(|id| {
            let task = match queue.tasks.iter().find(|t| t.id == *id) {
                Some(t) => t,
                None => return false,
            };

            // Status filter (OR logic)
            if !filters.status_filter.is_empty() && !filters.status_filter.contains(&task.status) {
                return false;
            }

            // Priority filter (OR logic)
            if !filters.priority_filter.is_empty()
                && !filters.priority_filter.contains(&task.priority)
            {
                return false;
            }

            // Scope filter (OR logic, case-insensitive substring match)
            if !normalized_scope_filters.is_empty() {
                let matches_scope = task.scope.iter().any(|s| {
                    let s_lower = s.to_lowercase();
                    normalized_scope_filters.iter().any(|f| s_lower.contains(f))
                });
                if !matches_scope {
                    return false;
                }
            }

            // Age filter
            if let Some(ref cutoff_str) = cutoff
                && let Some(ref updated_at) = task.updated_at
                && let Ok(updated) = crate::timeutil::parse_rfc3339(updated_at)
                && let Ok(cutoff_dt) = crate::timeutil::parse_rfc3339(cutoff_str)
                && updated > cutoff_dt
            {
                return false;
            }

            true
        })
        .collect();

    Ok(filtered)
}

/// Resolve task IDs from either explicit list or tag filter (legacy, without additional filters).
///
/// If `tag_filter` is provided, returns tasks matching any of the tags.
/// Otherwise, returns the explicit task IDs (after deduplication).
///
/// # Arguments
/// * `queue` - The queue file to search
/// * `task_ids` - Explicit list of task IDs
/// * `tag_filter` - Optional list of tags to filter by
///
/// # Returns
/// A deduplicated list of task IDs to operate on.
pub fn resolve_task_ids(
    queue: &QueueFile,
    task_ids: &[String],
    tag_filter: &[String],
) -> Result<Vec<String>> {
    let filters = BatchTaskFilters::default();
    let now = crate::timeutil::now_utc_rfc3339_or_fallback();
    resolve_task_ids_filtered(queue, task_ids, tag_filter, &filters, &now)
}

/// Batch set status for multiple tasks.
///
/// # Arguments
/// * `queue` - The queue file to modify
/// * `task_ids` - List of task IDs to update
/// * `status` - The new status to set
/// * `now_rfc3339` - Current timestamp for updated_at
/// * `note` - Optional note to append to each task
/// * `continue_on_error` - If true, continue processing on individual failures
///
/// # Returns
/// A `BatchOperationResult` with details of successes and failures.
pub fn batch_set_status(
    queue: &mut QueueFile,
    task_ids: &[String],
    status: TaskStatus,
    now_rfc3339: &str,
    note: Option<&str>,
    continue_on_error: bool,
) -> Result<BatchOperationResult> {
    let unique_ids = deduplicate_task_ids(task_ids);

    if unique_ids.is_empty() {
        bail!("No task IDs provided for batch status update");
    }

    // In atomic mode, validate all IDs exist first
    if !continue_on_error {
        validate_task_ids_exist(queue, &unique_ids)?;
    }

    let mut results = Vec::new();
    let mut succeeded = 0;
    let mut failed = 0;

    for task_id in &unique_ids {
        match queue::set_status(queue, task_id, status, now_rfc3339, note) {
            Ok(()) => {
                results.push(BatchTaskResult {
                    task_id: task_id.clone(),
                    success: true,
                    error: None,
                    created_task_ids: Vec::new(),
                });
                succeeded += 1;
            }
            Err(e) => {
                let error_msg = e.to_string();
                results.push(BatchTaskResult {
                    task_id: task_id.clone(),
                    success: false,
                    error: Some(error_msg.clone()),
                    created_task_ids: Vec::new(),
                });
                failed += 1;

                if !continue_on_error {
                    // In atomic mode, we should have already validated, but just in case
                    bail!(
                        "Batch operation failed at task {}: {}. Use --continue-on-error to process remaining tasks.",
                        task_id,
                        error_msg
                    );
                }
            }
        }
    }

    Ok(BatchOperationResult {
        total: unique_ids.len(),
        succeeded,
        failed,
        results,
    })
}

/// Batch set custom field for multiple tasks.
///
/// # Arguments
/// * `queue` - The queue file to modify
/// * `task_ids` - List of task IDs to update
/// * `key` - The custom field key
/// * `value` - The custom field value
/// * `now_rfc3339` - Current timestamp for updated_at
/// * `continue_on_error` - If true, continue processing on individual failures
///
/// # Returns
/// A `BatchOperationResult` with details of successes and failures.
pub fn batch_set_field(
    queue: &mut QueueFile,
    task_ids: &[String],
    key: &str,
    value: &str,
    now_rfc3339: &str,
    continue_on_error: bool,
) -> Result<BatchOperationResult> {
    let unique_ids = deduplicate_task_ids(task_ids);

    if unique_ids.is_empty() {
        bail!("No task IDs provided for batch field update");
    }

    // In atomic mode, validate all IDs exist first
    if !continue_on_error {
        validate_task_ids_exist(queue, &unique_ids)?;
    }

    let mut results = Vec::new();
    let mut succeeded = 0;
    let mut failed = 0;

    for task_id in &unique_ids {
        match queue::set_field(queue, task_id, key, value, now_rfc3339) {
            Ok(()) => {
                results.push(BatchTaskResult {
                    task_id: task_id.clone(),
                    success: true,
                    error: None,
                    created_task_ids: Vec::new(),
                });
                succeeded += 1;
            }
            Err(e) => {
                let error_msg = e.to_string();
                results.push(BatchTaskResult {
                    task_id: task_id.clone(),
                    success: false,
                    error: Some(error_msg.clone()),
                    created_task_ids: Vec::new(),
                });
                failed += 1;

                if !continue_on_error {
                    bail!(
                        "Batch operation failed at task {}: {}. Use --continue-on-error to process remaining tasks.",
                        task_id,
                        error_msg
                    );
                }
            }
        }
    }

    Ok(BatchOperationResult {
        total: unique_ids.len(),
        succeeded,
        failed,
        results,
    })
}

/// Batch edit field for multiple tasks.
///
/// # Arguments
/// * `queue` - The queue file to modify
/// * `done` - Optional done file for validation
/// * `task_ids` - List of task IDs to update
/// * `key` - The field to edit
/// * `value` - The new value
/// * `now_rfc3339` - Current timestamp for updated_at
/// * `id_prefix` - Task ID prefix for validation
/// * `id_width` - Task ID width for validation
/// * `max_dependency_depth` - Maximum dependency depth for validation
/// * `continue_on_error` - If true, continue processing on individual failures
///
/// # Returns
/// A `BatchOperationResult` with details of successes and failures.
#[allow(clippy::too_many_arguments)]
pub fn batch_apply_edit(
    queue: &mut QueueFile,
    done: Option<&QueueFile>,
    task_ids: &[String],
    key: TaskEditKey,
    value: &str,
    now_rfc3339: &str,
    id_prefix: &str,
    id_width: usize,
    max_dependency_depth: u8,
    continue_on_error: bool,
) -> Result<BatchOperationResult> {
    let unique_ids = deduplicate_task_ids(task_ids);

    if unique_ids.is_empty() {
        bail!("No task IDs provided for batch edit");
    }

    // In atomic mode, validate all IDs exist first
    if !continue_on_error {
        validate_task_ids_exist(queue, &unique_ids)?;
    }

    let mut results = Vec::new();
    let mut succeeded = 0;
    let mut failed = 0;

    for task_id in &unique_ids {
        match queue::apply_task_edit(
            queue,
            done,
            task_id,
            key,
            value,
            now_rfc3339,
            id_prefix,
            id_width,
            max_dependency_depth,
        ) {
            Ok(()) => {
                results.push(BatchTaskResult {
                    task_id: task_id.clone(),
                    success: true,
                    error: None,
                    created_task_ids: Vec::new(),
                });
                succeeded += 1;
            }
            Err(e) => {
                let error_msg = e.to_string();
                results.push(BatchTaskResult {
                    task_id: task_id.clone(),
                    success: false,
                    error: Some(error_msg.clone()),
                    created_task_ids: Vec::new(),
                });
                failed += 1;

                if !continue_on_error {
                    bail!(
                        "Batch operation failed at task {}: {}. Use --continue-on-error to process remaining tasks.",
                        task_id,
                        error_msg
                    );
                }
            }
        }
    }

    Ok(BatchOperationResult {
        total: unique_ids.len(),
        succeeded,
        failed,
        results,
    })
}

/// Batch delete multiple tasks from the queue.
///
/// # Arguments
/// * `queue` - The queue file to modify
/// * `task_ids` - List of task IDs to delete
/// * `continue_on_error` - If true, continue processing on individual failures
///
/// # Returns
/// A `BatchOperationResult` with details of successes and failures.
pub fn batch_delete_tasks(
    queue: &mut QueueFile,
    task_ids: &[String],
    continue_on_error: bool,
) -> Result<BatchOperationResult> {
    let unique_ids = deduplicate_task_ids(task_ids);

    if unique_ids.is_empty() {
        bail!("No task IDs provided for batch delete");
    }

    // In atomic mode, validate all IDs exist first
    if !continue_on_error {
        validate_task_ids_exist(queue, &unique_ids)?;
    }

    // Build set of IDs to delete for O(1) lookup
    let ids_to_delete: std::collections::HashSet<String> = unique_ids.iter().cloned().collect();
    let _initial_count = queue.tasks.len();

    // Filter out tasks to delete
    let mut results = Vec::new();
    let mut succeeded = 0;
    let mut failed = 0;

    // First pass: validate all exist if atomic
    for task_id in &unique_ids {
        let exists = queue.tasks.iter().any(|t| t.id == *task_id);
        if !exists {
            results.push(BatchTaskResult {
                task_id: task_id.clone(),
                success: false,
                error: Some(format!("Task not found: {}", task_id)),
                created_task_ids: Vec::new(),
            });
            failed += 1;

            if !continue_on_error {
                bail!("Task not found: {}", task_id);
            }
        }
    }

    // Second pass: actually remove tasks (in reverse order to maintain indices if we used them)
    queue.tasks.retain(|task| {
        if ids_to_delete.contains(&task.id) {
            // Find the result entry for this task and mark success
            if let Some(_result) = results.iter_mut().find(|r| r.task_id == task.id) {
                // Already marked as failed, keep it that way
            } else {
                results.push(BatchTaskResult {
                    task_id: task.id.clone(),
                    success: true,
                    error: None,
                    created_task_ids: Vec::new(),
                });
                succeeded += 1;
            }
            false // Remove this task
        } else {
            true // Keep this task
        }
    });

    // Ensure we have results for all tasks
    for task_id in &unique_ids {
        if !results.iter().any(|r| r.task_id == *task_id) {
            results.push(BatchTaskResult {
                task_id: task_id.clone(),
                success: true,
                error: None,
                created_task_ids: Vec::new(),
            });
            succeeded += 1;
        }
    }

    Ok(BatchOperationResult {
        total: unique_ids.len(),
        succeeded,
        failed,
        results,
    })
}

/// Batch archive terminal tasks (Done/Rejected) from active queue to done.
///
/// # Arguments
/// * `active` - The active queue file to modify
/// * `done` - The done archive file to append to
/// * `task_ids` - List of task IDs to archive
/// * `now_rfc3339` - Current timestamp for updated_at/completed_at
/// * `continue_on_error` - If true, continue processing on individual failures
///
/// # Returns
/// A `BatchOperationResult` with details of successes and failures.
pub fn batch_archive_tasks(
    active: &mut QueueFile,
    done: &mut QueueFile,
    task_ids: &[String],
    now_rfc3339: &str,
    continue_on_error: bool,
) -> Result<BatchOperationResult> {
    let unique_ids = deduplicate_task_ids(task_ids);

    if unique_ids.is_empty() {
        bail!("No task IDs provided for batch archive");
    }

    let mut results = Vec::new();
    let mut succeeded = 0;
    let mut failed = 0;

    for task_id in &unique_ids {
        // Find the task in active queue
        let task_idx = active.tasks.iter().position(|t| t.id == *task_id);

        match task_idx {
            Some(idx) => {
                let task = &active.tasks[idx];

                // Check if task is terminal (Done or Rejected)
                if !matches!(task.status, TaskStatus::Done | TaskStatus::Rejected) {
                    let err_msg = format!(
                        "Task {} has status '{}' which is not terminal (Done/Rejected)",
                        task_id, task.status
                    );
                    results.push(BatchTaskResult {
                        task_id: task_id.clone(),
                        success: false,
                        error: Some(err_msg.clone()),
                        created_task_ids: Vec::new(),
                    });
                    failed += 1;

                    if !continue_on_error {
                        bail!("{}", err_msg);
                    }
                    continue;
                }

                // Remove task from active and add to done
                let mut task = active.tasks.remove(idx);

                // Stamp completed_at if missing
                if task.completed_at.is_none() || task.completed_at.as_ref().unwrap().is_empty() {
                    task.completed_at = Some(now_rfc3339.to_string());
                }
                task.updated_at = Some(now_rfc3339.to_string());

                done.tasks.push(task);

                results.push(BatchTaskResult {
                    task_id: task_id.clone(),
                    success: true,
                    error: None,
                    created_task_ids: Vec::new(),
                });
                succeeded += 1;
            }
            None => {
                let err_msg = format!("Task not found in active queue: {}", task_id);
                results.push(BatchTaskResult {
                    task_id: task_id.clone(),
                    success: false,
                    error: Some(err_msg.clone()),
                    created_task_ids: Vec::new(),
                });
                failed += 1;

                if !continue_on_error {
                    bail!("{}", err_msg);
                }
            }
        }
    }

    Ok(BatchOperationResult {
        total: unique_ids.len(),
        succeeded,
        failed,
        results,
    })
}

/// Batch clone multiple tasks.
///
/// # Arguments
/// * `queue` - The active queue to insert cloned tasks into
/// * `done` - Optional done queue to search for source tasks
/// * `task_ids` - List of task IDs to clone
/// * `status` - Status for cloned tasks
/// * `title_prefix` - Optional prefix for cloned task titles
/// * `now_rfc3339` - Current timestamp for created_at/updated_at
/// * `id_prefix` - Task ID prefix
/// * `id_width` - Task ID numeric width
/// * `max_dependency_depth` - Max dependency depth for validation
/// * `continue_on_error` - If true, continue processing on individual failures
///
/// # Returns
/// A `BatchOperationResult` with details of successes and failures, including created task IDs.
#[allow(clippy::too_many_arguments)]
pub fn batch_clone_tasks(
    queue: &mut QueueFile,
    done: Option<&QueueFile>,
    task_ids: &[String],
    status: TaskStatus,
    title_prefix: Option<&str>,
    now_rfc3339: &str,
    id_prefix: &str,
    id_width: usize,
    max_dependency_depth: u8,
    continue_on_error: bool,
) -> Result<BatchOperationResult> {
    let unique_ids = deduplicate_task_ids(task_ids);

    if unique_ids.is_empty() {
        bail!("No task IDs provided for batch clone");
    }

    // In atomic mode, validate all source tasks exist first
    if !continue_on_error {
        for task_id in &unique_ids {
            let exists_in_active = queue.tasks.iter().any(|t| t.id == *task_id);
            let exists_in_done = done.is_some_and(|d| d.tasks.iter().any(|t| t.id == *task_id));
            if !exists_in_active && !exists_in_done {
                bail!("Source task not found: {}", task_id);
            }
        }
    }

    let mut results = Vec::new();
    let mut succeeded = 0;
    let mut failed = 0;
    let mut all_created_ids: Vec<String> = Vec::new();

    // Create a working copy for atomic mode
    let original_queue = if !continue_on_error {
        Some(queue.clone())
    } else {
        None
    };

    for task_id in &unique_ids {
        let opts = CloneTaskOptions::new(task_id, status, now_rfc3339, id_prefix, id_width)
            .with_title_prefix(title_prefix)
            .with_max_depth(max_dependency_depth);

        match queue::operations::clone_task(queue, done, &opts) {
            Ok((new_id, cloned_task)) => {
                // Insert the cloned task at a good position
                let insert_idx = queue::operations::suggest_new_task_insert_index(queue);
                queue.tasks.insert(insert_idx, cloned_task);

                all_created_ids.push(new_id.clone());

                results.push(BatchTaskResult {
                    task_id: task_id.clone(),
                    success: true,
                    error: None,
                    created_task_ids: vec![new_id],
                });
                succeeded += 1;
            }
            Err(e) => {
                let error_msg = e.to_string();
                results.push(BatchTaskResult {
                    task_id: task_id.clone(),
                    success: false,
                    error: Some(error_msg.clone()),
                    created_task_ids: Vec::new(),
                });
                failed += 1;

                if !continue_on_error {
                    // Rollback: restore original queue
                    if let Some(ref original) = original_queue {
                        *queue = original.clone();
                    }
                    bail!(
                        "Batch clone failed at task {}: {}. Use --continue-on-error to process remaining tasks.",
                        task_id,
                        error_msg
                    );
                }
            }
        }
    }

    Ok(BatchOperationResult {
        total: unique_ids.len(),
        succeeded,
        failed,
        results,
    })
}

/// Batch split multiple tasks into child tasks.
///
/// # Arguments
/// * `queue` - The active queue to modify
/// * `task_ids` - List of task IDs to split
/// * `number` - Number of child tasks to create per source
/// * `status` - Status for child tasks
/// * `title_prefix` - Optional prefix for child task titles
/// * `distribute_plan` - Whether to distribute plan items across children
/// * `now_rfc3339` - Current timestamp for timestamps
/// * `id_prefix` - Task ID prefix
/// * `id_width` - Task ID numeric width
/// * `max_dependency_depth` - Max dependency depth for validation
/// * `continue_on_error` - If true, continue processing on individual failures
///
/// # Returns
/// A `BatchOperationResult` with details of successes and failures.
#[allow(clippy::too_many_arguments)]
pub fn batch_split_tasks(
    queue: &mut QueueFile,
    task_ids: &[String],
    number: usize,
    status: TaskStatus,
    title_prefix: Option<&str>,
    distribute_plan: bool,
    now_rfc3339: &str,
    id_prefix: &str,
    id_width: usize,
    max_dependency_depth: u8,
    continue_on_error: bool,
) -> Result<BatchOperationResult> {
    if number < 2 {
        bail!("Number of child tasks must be at least 2");
    }

    let unique_ids = deduplicate_task_ids(task_ids);

    if unique_ids.is_empty() {
        bail!("No task IDs provided for batch split");
    }

    // In atomic mode, validate all source tasks exist first
    if !continue_on_error {
        for task_id in &unique_ids {
            if !queue.tasks.iter().any(|t| t.id == *task_id) {
                bail!("Source task not found in active queue: {}", task_id);
            }
        }
    }

    let mut results = Vec::new();
    let mut succeeded = 0;
    let mut failed = 0;

    // Create a working copy for atomic mode
    let original_queue = if !continue_on_error {
        Some(queue.clone())
    } else {
        None
    };

    for task_id in &unique_ids {
        let opts = SplitTaskOptions::new(task_id, number, status, now_rfc3339, id_prefix, id_width)
            .with_title_prefix(title_prefix)
            .with_distribute_plan(distribute_plan)
            .with_max_depth(max_dependency_depth);

        match queue::operations::split_task(queue, None, &opts) {
            Ok((updated_source, child_tasks)) => {
                // Find source task position
                if let Some(idx) = queue.tasks.iter().position(|t| t.id == *task_id) {
                    // Replace source with updated version
                    queue.tasks[idx] = updated_source;

                    // Insert children after the source
                    let child_ids: Vec<String> = child_tasks.iter().map(|t| t.id.clone()).collect();
                    for (i, child) in child_tasks.into_iter().enumerate() {
                        queue.tasks.insert(idx + 1 + i, child);
                    }

                    results.push(BatchTaskResult {
                        task_id: task_id.clone(),
                        success: true,
                        error: None,
                        created_task_ids: child_ids,
                    });
                    succeeded += 1;
                } else {
                    // This shouldn't happen since we validated above
                    let err_msg = "Source task disappeared during split".to_string();
                    results.push(BatchTaskResult {
                        task_id: task_id.clone(),
                        success: false,
                        error: Some(err_msg.clone()),
                        created_task_ids: Vec::new(),
                    });
                    failed += 1;

                    if !continue_on_error {
                        if let Some(ref original) = original_queue {
                            *queue = original.clone();
                        }
                        bail!("{}", err_msg);
                    }
                }
            }
            Err(e) => {
                let error_msg = e.to_string();
                results.push(BatchTaskResult {
                    task_id: task_id.clone(),
                    success: false,
                    error: Some(error_msg.clone()),
                    created_task_ids: Vec::new(),
                });
                failed += 1;

                if !continue_on_error {
                    if let Some(ref original) = original_queue {
                        *queue = original.clone();
                    }
                    bail!(
                        "Batch split failed at task {}: {}. Use --continue-on-error to process remaining tasks.",
                        task_id,
                        error_msg
                    );
                }
            }
        }
    }

    Ok(BatchOperationResult {
        total: unique_ids.len(),
        succeeded,
        failed,
        results,
    })
}

/// Batch append plan items to multiple tasks.
///
/// # Arguments
/// * `queue` - The queue file to modify
/// * `task_ids` - List of task IDs to update
/// * `plan_items` - Plan items to append
/// * `now_rfc3339` - Current timestamp for updated_at
/// * `continue_on_error` - If true, continue processing on individual failures
///
/// # Returns
/// A `BatchOperationResult` with details of successes and failures.
pub fn batch_plan_append(
    queue: &mut QueueFile,
    task_ids: &[String],
    plan_items: &[String],
    now_rfc3339: &str,
    continue_on_error: bool,
) -> Result<BatchOperationResult> {
    let unique_ids = deduplicate_task_ids(task_ids);

    if unique_ids.is_empty() {
        bail!("No task IDs provided for batch plan append");
    }

    if plan_items.is_empty() {
        bail!("No plan items provided for batch plan append");
    }

    // In atomic mode, validate all IDs exist first
    if !continue_on_error {
        validate_task_ids_exist(queue, &unique_ids)?;
    }

    let mut results = Vec::new();
    let mut succeeded = 0;
    let mut failed = 0;

    for task_id in &unique_ids {
        match queue.tasks.iter_mut().find(|t| t.id == *task_id) {
            Some(task) => {
                task.plan.extend(plan_items.iter().cloned());
                task.updated_at = Some(now_rfc3339.to_string());

                results.push(BatchTaskResult {
                    task_id: task_id.clone(),
                    success: true,
                    error: None,
                    created_task_ids: Vec::new(),
                });
                succeeded += 1;
            }
            None => {
                let err_msg = format!("Task not found: {}", task_id);
                results.push(BatchTaskResult {
                    task_id: task_id.clone(),
                    success: false,
                    error: Some(err_msg.clone()),
                    created_task_ids: Vec::new(),
                });
                failed += 1;

                if !continue_on_error {
                    bail!("{}", err_msg);
                }
            }
        }
    }

    Ok(BatchOperationResult {
        total: unique_ids.len(),
        succeeded,
        failed,
        results,
    })
}

/// Batch prepend plan items to multiple tasks.
///
/// # Arguments
/// * `queue` - The queue file to modify
/// * `task_ids` - List of task IDs to update
/// * `plan_items` - Plan items to prepend
/// * `now_rfc3339` - Current timestamp for updated_at
/// * `continue_on_error` - If true, continue processing on individual failures
///
/// # Returns
/// A `BatchOperationResult` with details of successes and failures.
pub fn batch_plan_prepend(
    queue: &mut QueueFile,
    task_ids: &[String],
    plan_items: &[String],
    now_rfc3339: &str,
    continue_on_error: bool,
) -> Result<BatchOperationResult> {
    let unique_ids = deduplicate_task_ids(task_ids);

    if unique_ids.is_empty() {
        bail!("No task IDs provided for batch plan prepend");
    }

    if plan_items.is_empty() {
        bail!("No plan items provided for batch plan prepend");
    }

    // In atomic mode, validate all IDs exist first
    if !continue_on_error {
        validate_task_ids_exist(queue, &unique_ids)?;
    }

    let mut results = Vec::new();
    let mut succeeded = 0;
    let mut failed = 0;

    for task_id in &unique_ids {
        match queue.tasks.iter_mut().find(|t| t.id == *task_id) {
            Some(task) => {
                // Prepend items: new items first, then existing
                let mut new_plan = plan_items.to_vec();
                new_plan.append(&mut task.plan);
                task.plan = new_plan;
                task.updated_at = Some(now_rfc3339.to_string());

                results.push(BatchTaskResult {
                    task_id: task_id.clone(),
                    success: true,
                    error: None,
                    created_task_ids: Vec::new(),
                });
                succeeded += 1;
            }
            None => {
                let err_msg = format!("Task not found: {}", task_id);
                results.push(BatchTaskResult {
                    task_id: task_id.clone(),
                    success: false,
                    error: Some(err_msg.clone()),
                    created_task_ids: Vec::new(),
                });
                failed += 1;

                if !continue_on_error {
                    bail!("{}", err_msg);
                }
            }
        }
    }

    Ok(BatchOperationResult {
        total: unique_ids.len(),
        succeeded,
        failed,
        results,
    })
}

/// Print batch operation results in a user-friendly format.
pub fn print_batch_results(result: &BatchOperationResult, operation_name: &str, dry_run: bool) {
    if dry_run {
        println!(
            "Dry run - would perform {} on {} tasks:",
            operation_name, result.total
        );
        for r in &result.results {
            if r.success {
                println!("  - {}: would update", r.task_id);
            } else {
                println!(
                    "  - {}: would fail - {}",
                    r.task_id,
                    r.error.as_deref().unwrap_or("unknown error")
                );
            }
        }
        println!("Dry run complete. No changes made.");
        return;
    }

    // Collect created task IDs for operations that create tasks
    let created_count: usize = result
        .results
        .iter()
        .map(|r| r.created_task_ids.len())
        .sum();

    if result.has_failures() {
        println!("{} completed with errors:", operation_name);
        for r in &result.results {
            if r.success {
                println!("  ✓ {}: updated", r.task_id);
                if !r.created_task_ids.is_empty() {
                    for created_id in &r.created_task_ids {
                        println!("    → Created: {}", created_id);
                    }
                }
            } else {
                println!(
                    "  ✗ {}: failed - {}",
                    r.task_id,
                    r.error.as_deref().unwrap_or("unknown error")
                );
            }
        }
        println!(
            "Completed: {}/{} tasks updated successfully.",
            result.succeeded, result.total
        );
        if created_count > 0 {
            println!("Created {} new tasks.", created_count);
        }
    } else {
        println!("{} completed successfully:", operation_name);
        for r in &result.results {
            println!("  ✓ {}", r.task_id);
            if !r.created_task_ids.is_empty() {
                for created_id in &r.created_task_ids {
                    println!("    → Created: {}", created_id);
                }
            }
        }
        println!("Updated {} tasks.", result.succeeded);
        if created_count > 0 {
            println!("Created {} new tasks.", created_count);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_older_than_cutoff_parses_days() {
        let now = "2026-02-05T00:00:00Z";
        let result = parse_older_than_cutoff(now, "7d").unwrap();
        assert!(result.contains("2026-01-29"));
    }

    #[test]
    fn parse_older_than_cutoff_parses_weeks() {
        let now = "2026-02-05T00:00:00Z";
        let result = parse_older_than_cutoff(now, "2w").unwrap();
        assert!(result.contains("2026-01-22"));
    }

    #[test]
    fn parse_older_than_cutoff_parses_date() {
        let result = parse_older_than_cutoff("2026-02-05T00:00:00Z", "2026-01-01").unwrap();
        assert!(result.contains("2026-01-01"));
    }

    #[test]
    fn parse_older_than_cutoff_parses_rfc3339() {
        let result =
            parse_older_than_cutoff("2026-02-05T00:00:00Z", "2026-01-15T12:00:00Z").unwrap();
        assert!(result.contains("2026-01-15"));
    }

    #[test]
    fn parse_older_than_cutoff_rejects_invalid() {
        let result = parse_older_than_cutoff("2026-02-05T00:00:00Z", "invalid");
        assert!(result.is_err());
    }

    #[test]
    fn batch_delete_tasks_removes_tasks() {
        let mut queue = QueueFile {
            version: 1,
            tasks: vec![
                Task {
                    id: "RQ-0001".to_string(),
                    title: "Task 1".to_string(),
                    ..Default::default()
                },
                Task {
                    id: "RQ-0002".to_string(),
                    title: "Task 2".to_string(),
                    ..Default::default()
                },
                Task {
                    id: "RQ-0003".to_string(),
                    title: "Task 3".to_string(),
                    ..Default::default()
                },
            ],
        };

        let result = batch_delete_tasks(
            &mut queue,
            &["RQ-0001".to_string(), "RQ-0002".to_string()],
            false,
        )
        .unwrap();

        assert_eq!(result.succeeded, 2);
        assert_eq!(result.failed, 0);
        assert_eq!(queue.tasks.len(), 1);
        assert_eq!(queue.tasks[0].id, "RQ-0003");
    }

    #[test]
    fn batch_delete_tasks_atomic_fails_on_missing() {
        let mut queue = QueueFile {
            version: 1,
            tasks: vec![Task {
                id: "RQ-0001".to_string(),
                title: "Task 1".to_string(),
                ..Default::default()
            }],
        };

        let result = batch_delete_tasks(
            &mut queue,
            &["RQ-0001".to_string(), "RQ-9999".to_string()],
            false,
        );
        assert!(result.is_err());
    }

    #[test]
    fn batch_plan_append_adds_items() {
        let mut queue = QueueFile {
            version: 1,
            tasks: vec![Task {
                id: "RQ-0001".to_string(),
                title: "Task 1".to_string(),
                plan: vec!["Step 1".to_string()],
                ..Default::default()
            }],
        };

        let result = batch_plan_append(
            &mut queue,
            &["RQ-0001".to_string()],
            &["Step 2".to_string(), "Step 3".to_string()],
            "2026-02-05T00:00:00Z",
            false,
        )
        .unwrap();

        assert_eq!(result.succeeded, 1);
        assert_eq!(queue.tasks[0].plan.len(), 3);
        assert_eq!(queue.tasks[0].plan[0], "Step 1");
        assert_eq!(queue.tasks[0].plan[1], "Step 2");
        assert_eq!(queue.tasks[0].plan[2], "Step 3");
    }

    #[test]
    fn batch_plan_prepend_adds_items_first() {
        let mut queue = QueueFile {
            version: 1,
            tasks: vec![Task {
                id: "RQ-0001".to_string(),
                title: "Task 1".to_string(),
                plan: vec!["Step 2".to_string()],
                ..Default::default()
            }],
        };

        let result = batch_plan_prepend(
            &mut queue,
            &["RQ-0001".to_string()],
            &["Step 1".to_string()],
            "2026-02-05T00:00:00Z",
            false,
        )
        .unwrap();

        assert_eq!(result.succeeded, 1);
        assert_eq!(queue.tasks[0].plan.len(), 2);
        assert_eq!(queue.tasks[0].plan[0], "Step 1");
        assert_eq!(queue.tasks[0].plan[1], "Step 2");
    }
}
