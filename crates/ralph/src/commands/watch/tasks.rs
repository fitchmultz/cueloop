//! Task handling for the watch command.
//!
//! Responsibilities:
//! - Create watch tasks from detected comments.
//! - Deduplicate comments against active watch tasks using versioned metadata.
//! - Reconcile removals only for files processed in the current watch batch.
//!
//! Not handled here:
//! - Comment detection (see `comments.rs`).
//! - File watching and debounce orchestration (see `event_loop.rs` / `processor.rs`).
//!
//! Invariants/assumptions:
//! - V2 watch identity is explicit and location-aware.
//! - Legacy watch tasks are only matched via structured metadata.
//! - Title or notes text is never used for deduplication or reconciliation.

use crate::commands::watch::identity::{
    ParsedWatchIdentity, WATCH_FIELD_COMMENT_TYPE, WATCH_FIELD_CONTENT_HASH, WATCH_FIELD_FILE,
    WATCH_FIELD_FINGERPRINT, WATCH_FIELD_IDENTITY_KEY, WATCH_FIELD_LINE, WATCH_FIELD_LOCATION_KEY,
    WATCH_FIELD_VERSION, WATCH_VERSION_V2, WatchCommentIdentity, parse_task_watch_identity,
    path_key, upgrade_task_to_v2,
};
use crate::commands::watch::types::{DetectedComment, WatchOptions};
use crate::config::Resolved;
use crate::contracts::{QueueFile, Task, TaskPriority, TaskStatus};
use crate::notification::{NotificationConfig, notify_watch_new_task};
use crate::queue::{load_queue, load_queue_or_default, save_queue, suggest_new_task_insert_index};
use crate::timeutil;
use anyhow::{Context, Result};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

/// Handle detected comments by creating tasks or suggesting them.
pub fn handle_detected_comments(
    resolved: &Resolved,
    comments: &[DetectedComment],
    processed_files: &[PathBuf],
    opts: &WatchOptions,
) -> Result<()> {
    let mut queue = load_queue(&resolved.queue_path)
        .with_context(|| format!("load queue {}", resolved.queue_path.display()))?;
    let now = timeutil::now_utc_rfc3339_or_fallback();
    let mut created_tasks: Vec<(String, String)> = Vec::new();
    let mut queue_changed = false;

    for comment in comments {
        let identity = WatchCommentIdentity::from_detected_comment(comment);

        if let Some(index) = find_matching_active_watch_task_index(&queue, &identity) {
            if matches!(
                parse_task_watch_identity(&queue.tasks[index]),
                Some(ParsedWatchIdentity::LegacyStructured(_))
            ) {
                let task = &mut queue.tasks[index];
                upgrade_task_to_v2(task, &identity);
                task.updated_at = Some(now.clone());
                task.notes.push(format!(
                    "[watch] Automatically upgraded metadata to watch.version=2 at {}",
                    now
                ));
                queue_changed = true;
            }
            continue;
        }

        let task = create_task_from_comment(comment, resolved)?;

        if opts.auto_queue {
            let insert_at = suggest_new_task_insert_index(&queue);
            created_tasks.push((task.id.clone(), task.title.clone()));
            queue.tasks.insert(insert_at, task);
            queue_changed = true;
        } else {
            let type_str = format!("{:?}", comment.comment_type).to_uppercase();
            log::info!(
                "[SUGGESTION] {} at {}:{}",
                type_str,
                comment.file_path.display(),
                comment.line_number
            );
            log::info!("  Content: {}", comment.content);
            log::info!("  Suggested task: {}", task.title);
        }
    }

    if opts.close_removed {
        let closed = reconcile_watch_tasks_in_queue(&mut queue, comments, processed_files, &now);
        if !closed.is_empty() {
            queue_changed = true;
            log::info!(
                "Reconciled {} watch task(s) due to removed comments",
                closed.len()
            );
        }
    }

    if queue_changed {
        save_queue(&resolved.queue_path, &queue)
            .with_context(|| format!("save queue {}", resolved.queue_path.display()))?;
    }

    if opts.auto_queue && !created_tasks.is_empty() {
        log::info!("Added {} task(s) to queue", created_tasks.len());
        if opts.notify {
            let config = NotificationConfig::new();
            notify_watch_new_task(created_tasks.len(), &config);
        }
    }

    Ok(())
}

/// Reconcile watch tasks against currently detected comments for the processed files.
#[cfg(test)]
pub fn reconcile_watch_tasks(
    resolved: &Resolved,
    detected_comments: &[DetectedComment],
    processed_files: &[PathBuf],
    _opts: &WatchOptions,
) -> Result<Vec<String>> {
    let mut queue = load_queue(&resolved.queue_path)
        .with_context(|| format!("load queue {}", resolved.queue_path.display()))?;
    let now = timeutil::now_utc_rfc3339_or_fallback();
    let closed =
        reconcile_watch_tasks_in_queue(&mut queue, detected_comments, processed_files, &now);

    if !closed.is_empty() {
        save_queue(&resolved.queue_path, &queue)
            .with_context(|| format!("save queue {}", resolved.queue_path.display()))?;
    }

    Ok(closed)
}

fn reconcile_watch_tasks_in_queue(
    queue: &mut QueueFile,
    detected_comments: &[DetectedComment],
    processed_files: &[PathBuf],
    now: &str,
) -> Vec<String> {
    let processed_files: HashSet<String> =
        processed_files.iter().map(|path| path_key(path)).collect();
    let current_comments: Vec<WatchCommentIdentity> = detected_comments
        .iter()
        .map(WatchCommentIdentity::from_detected_comment)
        .collect();
    let current_identity_keys: HashSet<&str> = current_comments
        .iter()
        .map(|identity| identity.identity_key.as_str())
        .collect();
    let mut closed = Vec::new();

    for task in &mut queue.tasks {
        if !is_active_watch_task(task) {
            continue;
        }

        let Some(parsed_identity) = parse_task_watch_identity(task) else {
            continue;
        };

        let task_file = match &parsed_identity {
            ParsedWatchIdentity::V2(identity) => identity.file.as_str(),
            ParsedWatchIdentity::LegacyStructured(identity) => identity.file.as_str(),
            ParsedWatchIdentity::LegacyUnstructured => continue,
        };

        if !processed_files.contains(task_file) {
            continue;
        }

        let comment_still_exists = match &parsed_identity {
            ParsedWatchIdentity::V2(identity) => {
                current_identity_keys.contains(identity.identity_key.as_str())
            }
            ParsedWatchIdentity::LegacyStructured(identity) => current_comments
                .iter()
                .any(|current| identity.matches_comment(current)),
            ParsedWatchIdentity::LegacyUnstructured => true,
        };

        if !comment_still_exists {
            mark_task_done_from_removed_comment(task, now);
            closed.push(task.id.clone());
        }
    }

    closed
}

/// Check if an active watch task already exists for a given comment.
#[cfg(test)]
pub fn task_exists_for_comment(queue: &QueueFile, comment: &DetectedComment) -> bool {
    let identity = WatchCommentIdentity::from_detected_comment(comment);
    find_matching_active_watch_task_index(queue, &identity).is_some()
}

fn find_matching_active_watch_task_index(
    queue: &QueueFile,
    identity: &WatchCommentIdentity,
) -> Option<usize> {
    queue.tasks.iter().enumerate().find_map(|(index, task)| {
        if !is_active_watch_task(task) {
            return None;
        }

        match parse_task_watch_identity(task)? {
            ParsedWatchIdentity::V2(existing) if existing.identity_key == identity.identity_key => {
                Some(index)
            }
            ParsedWatchIdentity::LegacyStructured(existing)
                if existing.matches_comment(identity) =>
            {
                Some(index)
            }
            _ => None,
        }
    })
}

fn is_active_watch_task(task: &Task) -> bool {
    task.tags.iter().any(|tag| tag == "watch")
        && task.status != TaskStatus::Done
        && task.status != TaskStatus::Rejected
}

fn mark_task_done_from_removed_comment(task: &mut Task, now: &str) {
    task.status = TaskStatus::Done;
    task.completed_at = Some(now.to_string());
    task.updated_at = Some(now.to_string());
    task.notes.push(format!(
        "[watch] Automatically marked done: originating comment was removed at {}",
        now
    ));
}

/// Create a task from a detected comment using V2 watch metadata.
pub fn create_task_from_comment(comment: &DetectedComment, resolved: &Resolved) -> Result<Task> {
    let type_str = format!("{:?}", comment.comment_type).to_uppercase();
    let file_name = comment
        .file_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("unknown");
    let title = format!(
        "{}: {} in {}",
        type_str,
        comment.content.chars().take(50).collect::<String>(),
        file_name
    );
    let now = timeutil::now_utc_rfc3339_or_fallback();
    let task_id = generate_task_id(resolved)?;
    let identity = WatchCommentIdentity::from_detected_comment(comment);

    let mut custom_fields = HashMap::new();
    custom_fields.insert(
        WATCH_FIELD_VERSION.to_string(),
        WATCH_VERSION_V2.to_string(),
    );
    custom_fields.insert(WATCH_FIELD_FILE.to_string(), identity.file.clone());
    custom_fields.insert(WATCH_FIELD_LINE.to_string(), identity.line.to_string());
    custom_fields.insert(
        WATCH_FIELD_COMMENT_TYPE.to_string(),
        identity.comment_type.clone(),
    );
    custom_fields.insert(
        WATCH_FIELD_CONTENT_HASH.to_string(),
        identity.content_hash.clone(),
    );
    custom_fields.insert(
        WATCH_FIELD_LOCATION_KEY.to_string(),
        identity.location_key.clone(),
    );
    custom_fields.insert(
        WATCH_FIELD_IDENTITY_KEY.to_string(),
        identity.identity_key.clone(),
    );
    custom_fields.insert(
        WATCH_FIELD_FINGERPRINT.to_string(),
        identity.content_hash.clone(),
    );

    Ok(Task {
        id: task_id,
        status: TaskStatus::Todo,
        title,
        description: None,
        priority: TaskPriority::Medium,
        tags: vec![
            "watch".to_string(),
            format!("{:?}", comment.comment_type).to_lowercase(),
        ],
        scope: vec![identity.file.clone()],
        evidence: Vec::new(),
        plan: Vec::new(),
        notes: vec![
            format!(
                "Detected in: {}:{}",
                comment.file_path.display(),
                comment.line_number
            ),
            format!("Full content: {}", comment.content),
            format!("Context: {}", comment.context),
        ],
        request: Some(format!("Address {} comment", type_str)),
        agent: None,
        created_at: Some(now.clone()),
        updated_at: Some(now),
        completed_at: None,
        started_at: None,
        estimated_minutes: None,
        actual_minutes: None,
        scheduled_start: None,
        depends_on: Vec::new(),
        blocks: Vec::new(),
        relates_to: Vec::new(),
        duplicates: None,
        custom_fields,
        parent_id: None,
    })
}

/// Generate a unique task ID using the shared queue helper.
fn generate_task_id(resolved: &Resolved) -> Result<String> {
    let active_queue = load_queue_or_default(&resolved.queue_path)?;
    let done_queue = if resolved.done_path.exists() {
        Some(load_queue_or_default(&resolved.done_path)?)
    } else {
        None
    };

    let max_depth = resolved.config.queue.max_dependency_depth.unwrap_or(10);

    crate::queue::next_id_across(
        &active_queue,
        done_queue.as_ref(),
        &resolved.id_prefix,
        resolved.id_width,
        max_depth,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::watch::identity::generate_comment_fingerprint;
    use crate::commands::watch::types::CommentType;
    use crate::contracts::Config;
    use std::collections::HashMap;
    use tempfile::TempDir;

    fn create_test_resolved(temp_dir: &TempDir) -> Resolved {
        let queue_path = temp_dir.path().join("queue.json");
        let done_path = temp_dir.path().join("done.json");
        let queue = QueueFile::default();
        let queue_json = serde_json::to_string_pretty(&queue).unwrap();
        std::fs::write(&queue_path, queue_json).unwrap();

        Resolved {
            config: Config::default(),
            repo_root: temp_dir.path().to_path_buf(),
            queue_path,
            done_path,
            id_prefix: "RQ".to_string(),
            id_width: 4,
            global_config_path: None,
            project_config_path: None,
        }
    }

    fn watch_opts(auto_queue: bool, close_removed: bool) -> WatchOptions {
        WatchOptions {
            patterns: vec!["*.rs".to_string()],
            debounce_ms: 100,
            auto_queue,
            notify: false,
            ignore_patterns: vec![],
            comment_types: vec![CommentType::All],
            paths: vec![PathBuf::from(".")],
            force: false,
            close_removed,
        }
    }

    fn detected_comment(
        path: &str,
        line: usize,
        comment_type: CommentType,
        content: &str,
    ) -> DetectedComment {
        DetectedComment {
            file_path: PathBuf::from(path),
            line_number: line,
            comment_type,
            content: content.to_string(),
            context: format!("{path}:{line} - {content}"),
        }
    }

    fn task_with_custom_fields(
        id: &str,
        status: TaskStatus,
        custom_fields: HashMap<String, String>,
    ) -> Task {
        Task {
            id: id.to_string(),
            status,
            title: "watch task".to_string(),
            description: None,
            priority: TaskPriority::Medium,
            tags: vec!["watch".to_string(), "todo".to_string()],
            scope: vec![],
            evidence: vec![],
            plan: vec![],
            notes: vec![],
            request: None,
            agent: None,
            created_at: Some("2026-01-01T00:00:00Z".to_string()),
            updated_at: Some("2026-01-01T00:00:00Z".to_string()),
            completed_at: None,
            started_at: None,
            estimated_minutes: None,
            actual_minutes: None,
            scheduled_start: None,
            depends_on: vec![],
            blocks: vec![],
            relates_to: vec![],
            duplicates: None,
            custom_fields,
            parent_id: None,
        }
    }

    fn v1_custom_fields(
        path: &str,
        line: usize,
        comment_type: &str,
        content: &str,
    ) -> HashMap<String, String> {
        let mut fields = HashMap::new();
        fields.insert(WATCH_FIELD_VERSION.to_string(), "1".to_string());
        fields.insert(WATCH_FIELD_FILE.to_string(), path.to_string());
        fields.insert(WATCH_FIELD_LINE.to_string(), line.to_string());
        fields.insert(
            WATCH_FIELD_COMMENT_TYPE.to_string(),
            comment_type.to_string(),
        );
        fields.insert(
            WATCH_FIELD_FINGERPRINT.to_string(),
            generate_comment_fingerprint(content),
        );
        fields
    }

    #[test]
    fn generate_task_id_first_id_format() {
        let temp_dir = TempDir::new().unwrap();
        let resolved = create_test_resolved(&temp_dir);

        let task_id = generate_task_id(&resolved).unwrap();

        assert_eq!(task_id, "RQ-0001");
    }

    #[test]
    fn create_task_from_comment_populates_v2_custom_fields() {
        let temp_dir = TempDir::new().unwrap();
        let resolved = create_test_resolved(&temp_dir);
        let comment = detected_comment(
            "/test/file.rs",
            42,
            CommentType::Todo,
            "Fix the error handling",
        );

        let task = create_task_from_comment(&comment, &resolved).unwrap();

        assert_eq!(
            task.custom_fields.get(WATCH_FIELD_VERSION),
            Some(&WATCH_VERSION_V2.to_string())
        );
        assert_eq!(
            task.custom_fields.get(WATCH_FIELD_FILE),
            Some(&"/test/file.rs".to_string())
        );
        assert_eq!(
            task.custom_fields.get(WATCH_FIELD_LINE),
            Some(&"42".to_string())
        );
        assert_eq!(
            task.custom_fields.get(WATCH_FIELD_COMMENT_TYPE),
            Some(&"todo".to_string())
        );
        assert!(task.custom_fields.contains_key(WATCH_FIELD_CONTENT_HASH));
        assert!(task.custom_fields.contains_key(WATCH_FIELD_LOCATION_KEY));
        assert!(task.custom_fields.contains_key(WATCH_FIELD_IDENTITY_KEY));
    }

    #[test]
    fn same_comment_text_in_two_files_creates_two_tasks() {
        let temp_dir = TempDir::new().unwrap();
        let resolved = create_test_resolved(&temp_dir);
        let comments = vec![
            detected_comment("/src/a.rs", 10, CommentType::Todo, "fix this"),
            detected_comment("/src/b.rs", 20, CommentType::Todo, "fix this"),
        ];
        let processed_files = vec![PathBuf::from("/src/a.rs"), PathBuf::from("/src/b.rs")];

        handle_detected_comments(
            &resolved,
            &comments,
            &processed_files,
            &watch_opts(true, false),
        )
        .unwrap();

        let queue = load_queue(&resolved.queue_path).unwrap();
        assert_eq!(queue.tasks.len(), 2);
        assert_ne!(
            queue.tasks[0].custom_fields.get(WATCH_FIELD_IDENTITY_KEY),
            queue.tasks[1].custom_fields.get(WATCH_FIELD_IDENTITY_KEY)
        );
    }

    #[test]
    fn removing_one_of_identical_comments_only_closes_its_task() {
        let temp_dir = TempDir::new().unwrap();
        let resolved = create_test_resolved(&temp_dir);
        let comments = vec![
            detected_comment("/src/a.rs", 10, CommentType::Todo, "fix this"),
            detected_comment("/src/b.rs", 20, CommentType::Todo, "fix this"),
        ];

        handle_detected_comments(
            &resolved,
            &comments,
            &[PathBuf::from("/src/a.rs"), PathBuf::from("/src/b.rs")],
            &watch_opts(true, false),
        )
        .unwrap();

        handle_detected_comments(
            &resolved,
            &[],
            &[PathBuf::from("/src/a.rs")],
            &watch_opts(true, true),
        )
        .unwrap();

        let queue = load_queue(&resolved.queue_path).unwrap();
        let a_task = queue
            .tasks
            .iter()
            .find(|task| task.custom_fields.get(WATCH_FIELD_FILE) == Some(&"/src/a.rs".to_string()))
            .unwrap();
        let b_task = queue
            .tasks
            .iter()
            .find(|task| task.custom_fields.get(WATCH_FIELD_FILE) == Some(&"/src/b.rs".to_string()))
            .unwrap();

        assert_eq!(a_task.status, TaskStatus::Done);
        assert_eq!(b_task.status, TaskStatus::Todo);
    }

    #[test]
    fn same_comment_text_twice_in_one_file_creates_two_tasks() {
        let temp_dir = TempDir::new().unwrap();
        let resolved = create_test_resolved(&temp_dir);
        let comments = vec![
            detected_comment("/src/a.rs", 10, CommentType::Todo, "fix this"),
            detected_comment("/src/a.rs", 50, CommentType::Todo, "fix this"),
        ];

        handle_detected_comments(
            &resolved,
            &comments,
            &[PathBuf::from("/src/a.rs")],
            &watch_opts(true, false),
        )
        .unwrap();

        let queue = load_queue(&resolved.queue_path).unwrap();
        assert_eq!(queue.tasks.len(), 2);
        let mut lines: Vec<String> = queue
            .tasks
            .iter()
            .map(|task| task.custom_fields.get(WATCH_FIELD_LINE).unwrap().clone())
            .collect();
        lines.sort();
        assert_eq!(lines, vec!["10".to_string(), "50".to_string()]);
    }

    #[test]
    fn moved_comment_closes_old_task_and_creates_new_task() {
        let temp_dir = TempDir::new().unwrap();
        let resolved = create_test_resolved(&temp_dir);

        handle_detected_comments(
            &resolved,
            &[detected_comment(
                "/src/a.rs",
                10,
                CommentType::Todo,
                "fix this",
            )],
            &[PathBuf::from("/src/a.rs")],
            &watch_opts(true, false),
        )
        .unwrap();

        handle_detected_comments(
            &resolved,
            &[detected_comment(
                "/src/a.rs",
                50,
                CommentType::Todo,
                "fix this",
            )],
            &[PathBuf::from("/src/a.rs")],
            &watch_opts(true, true),
        )
        .unwrap();

        let queue = load_queue(&resolved.queue_path).unwrap();
        assert_eq!(queue.tasks.len(), 2);
        assert_eq!(
            queue
                .tasks
                .iter()
                .filter(|task| task.status == TaskStatus::Todo)
                .count(),
            1
        );
        assert_eq!(
            queue
                .tasks
                .iter()
                .filter(|task| task.status == TaskStatus::Done)
                .count(),
            1
        );
    }

    #[test]
    fn renamed_file_closes_old_task_and_creates_new_task() {
        let temp_dir = TempDir::new().unwrap();
        let resolved = create_test_resolved(&temp_dir);

        handle_detected_comments(
            &resolved,
            &[detected_comment(
                "/src/old.rs",
                10,
                CommentType::Todo,
                "fix this",
            )],
            &[PathBuf::from("/src/old.rs")],
            &watch_opts(true, false),
        )
        .unwrap();

        handle_detected_comments(
            &resolved,
            &[detected_comment(
                "/src/new.rs",
                10,
                CommentType::Todo,
                "fix this",
            )],
            &[PathBuf::from("/src/old.rs"), PathBuf::from("/src/new.rs")],
            &watch_opts(true, true),
        )
        .unwrap();

        let queue = load_queue(&resolved.queue_path).unwrap();
        assert_eq!(queue.tasks.len(), 2);
        let old_task = queue
            .tasks
            .iter()
            .find(|task| {
                task.custom_fields.get(WATCH_FIELD_FILE) == Some(&"/src/old.rs".to_string())
            })
            .unwrap();
        let new_task = queue
            .tasks
            .iter()
            .find(|task| {
                task.custom_fields.get(WATCH_FIELD_FILE) == Some(&"/src/new.rs".to_string())
            })
            .unwrap();

        assert_eq!(old_task.status, TaskStatus::Done);
        assert_eq!(new_task.status, TaskStatus::Todo);
    }

    #[test]
    fn repeated_scan_of_unchanged_file_does_not_duplicate_task() {
        let temp_dir = TempDir::new().unwrap();
        let resolved = create_test_resolved(&temp_dir);
        let comments = vec![detected_comment(
            "/src/a.rs",
            10,
            CommentType::Todo,
            "fix this",
        )];
        let processed_files = vec![PathBuf::from("/src/a.rs")];

        handle_detected_comments(
            &resolved,
            &comments,
            &processed_files,
            &watch_opts(true, false),
        )
        .unwrap();
        handle_detected_comments(
            &resolved,
            &comments,
            &processed_files,
            &watch_opts(true, false),
        )
        .unwrap();

        let queue = load_queue(&resolved.queue_path).unwrap();
        assert_eq!(queue.tasks.len(), 1);
    }

    #[test]
    fn legacy_v1_task_is_upgraded_in_place_when_comment_still_exists() {
        let temp_dir = TempDir::new().unwrap();
        let resolved = create_test_resolved(&temp_dir);
        let mut queue = QueueFile::default();
        queue.tasks.push(task_with_custom_fields(
            "RQ-0001",
            TaskStatus::Todo,
            v1_custom_fields("/src/a.rs", 10, "todo", "fix this"),
        ));
        save_queue(&resolved.queue_path, &queue).unwrap();

        handle_detected_comments(
            &resolved,
            &[detected_comment(
                "/src/a.rs",
                10,
                CommentType::Todo,
                "fix this",
            )],
            &[PathBuf::from("/src/a.rs")],
            &watch_opts(true, false),
        )
        .unwrap();

        let queue = load_queue(&resolved.queue_path).unwrap();
        assert_eq!(queue.tasks.len(), 1);
        assert_eq!(
            queue.tasks[0].custom_fields.get(WATCH_FIELD_VERSION),
            Some(&WATCH_VERSION_V2.to_string())
        );
        assert!(
            queue.tasks[0]
                .custom_fields
                .contains_key(WATCH_FIELD_IDENTITY_KEY)
        );
    }

    #[test]
    fn old_format_watch_task_without_structured_metadata_is_left_untouched() {
        let temp_dir = TempDir::new().unwrap();
        let resolved = create_test_resolved(&temp_dir);
        let mut queue = QueueFile::default();
        queue.tasks.push(task_with_custom_fields(
            "RQ-0001",
            TaskStatus::Todo,
            HashMap::new(),
        ));
        save_queue(&resolved.queue_path, &queue).unwrap();

        handle_detected_comments(
            &resolved,
            &[detected_comment(
                "/src/a.rs",
                10,
                CommentType::Todo,
                "fix this",
            )],
            &[PathBuf::from("/src/a.rs")],
            &watch_opts(true, true),
        )
        .unwrap();

        let queue = load_queue(&resolved.queue_path).unwrap();
        assert_eq!(queue.tasks.len(), 2);
        let legacy_task = queue
            .tasks
            .iter()
            .find(|task| task.id == "RQ-0001")
            .unwrap();
        assert_eq!(legacy_task.status, TaskStatus::Todo);
        assert!(legacy_task.custom_fields.is_empty());
    }

    #[test]
    fn task_exists_for_comment_ignores_done_and_rejected_watch_tasks() {
        let comment = detected_comment("/src/a.rs", 10, CommentType::Todo, "fix this");
        let mut queue = QueueFile::default();
        queue.tasks.push(task_with_custom_fields(
            "RQ-0001",
            TaskStatus::Done,
            v1_custom_fields("/src/a.rs", 10, "todo", "fix this"),
        ));
        queue.tasks.push(task_with_custom_fields(
            "RQ-0002",
            TaskStatus::Rejected,
            v1_custom_fields("/src/a.rs", 10, "todo", "fix this"),
        ));

        assert!(!task_exists_for_comment(&queue, &comment));
    }

    #[test]
    fn reconcile_only_closes_tasks_for_processed_files() {
        let temp_dir = TempDir::new().unwrap();
        let resolved = create_test_resolved(&temp_dir);
        let mut queue = QueueFile::default();
        queue.tasks.push(
            create_task_from_comment(
                &detected_comment("/src/a.rs", 10, CommentType::Todo, "fix this"),
                &resolved,
            )
            .unwrap(),
        );
        queue.tasks.push(
            create_task_from_comment(
                &detected_comment("/src/b.rs", 20, CommentType::Todo, "fix this"),
                &resolved,
            )
            .unwrap(),
        );
        save_queue(&resolved.queue_path, &queue).unwrap();

        let closed = reconcile_watch_tasks(
            &resolved,
            &[],
            &[PathBuf::from("/src/a.rs")],
            &watch_opts(true, true),
        )
        .unwrap();

        assert_eq!(closed.len(), 1);
        let queue = load_queue(&resolved.queue_path).unwrap();
        let a_task = queue
            .tasks
            .iter()
            .find(|task| task.custom_fields.get(WATCH_FIELD_FILE) == Some(&"/src/a.rs".to_string()))
            .unwrap();
        let b_task = queue
            .tasks
            .iter()
            .find(|task| task.custom_fields.get(WATCH_FIELD_FILE) == Some(&"/src/b.rs".to_string()))
            .unwrap();
        assert_eq!(a_task.status, TaskStatus::Done);
        assert_eq!(b_task.status, TaskStatus::Todo);
    }
}
