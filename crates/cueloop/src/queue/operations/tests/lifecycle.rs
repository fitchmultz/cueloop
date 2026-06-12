//! Regression tests for shared lifecycle queue operations.

use super::*;
use crate::queue::{load_queue, load_queue_or_default, save_queue};

fn queue(tasks: Vec<Task>) -> QueueFile {
    QueueFile { version: 1, tasks }
}

fn done_task(id: &str) -> Task {
    let mut item = task_with(id, TaskStatus::Done, vec!["done".to_string()]);
    item.completed_at = Some("2026-06-12T00:00:00Z".to_string());
    item
}

#[test]
fn find_task_with_location_prefers_active_then_done_and_trims_ids() {
    let active = queue(vec![task("CL-0001")]);
    let done = queue(vec![done_task("CL-0001")]);

    let located = find_task_with_location(&active, Some(&done), "  CL-0001  ")
        .expect("trimmed lookup should find active task first");

    assert_eq!(located.location, QueueTaskLocation::Active);
    assert_eq!(located.task.status, TaskStatus::Todo);
}

#[test]
fn find_task_with_location_finds_done_and_rejects_empty_ids() {
    let active = queue(vec![]);
    let done = queue(vec![done_task("CL-0002")]);

    let located = find_task_with_location(&active, Some(&done), "CL-0002")
        .expect("done task should be found");
    assert_eq!(located.location, QueueTaskLocation::Done);
    assert_eq!(located.task.id, "CL-0002");
    assert!(find_task_with_location(&active, Some(&done), "   ").is_none());
    assert!(find_task_with_location(&active, Some(&done), "CL-9999").is_none());
}

#[test]
fn apply_lifecycle_update_rejects_terminal_statuses() {
    let mut item = task("CL-0001");
    let err = apply_lifecycle_update(
        &mut item,
        TaskStatus::Done,
        "2026-06-12T00:00:00Z",
        &[],
        &[],
        LifecycleNoteMode::Items,
    )
    .expect_err("terminal status should be rejected");

    assert!(err.to_string().contains("terminal lifecycle updates"));
}

#[test]
fn apply_lifecycle_update_appends_items_or_joined_notes_and_filters_empty_redacted_values()
-> anyhow::Result<()> {
    let mut items_task = task("CL-0001");
    apply_lifecycle_update(
        &mut items_task,
        TaskStatus::Doing,
        "2026-06-12T00:00:00Z",
        &[
            " first note ".to_string(),
            " ".to_string(),
            "token abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789".to_string(),
        ],
        &[" evidence ".to_string(), " ".to_string()],
        LifecycleNoteMode::Items,
    )?;
    assert_eq!(
        items_task.notes,
        vec!["first note".to_string(), "token [REDACTED]".to_string()]
    );
    assert_eq!(
        items_task.evidence,
        vec!["observed".to_string(), "evidence".to_string()]
    );

    let mut joined_task = task("CL-0002");
    apply_lifecycle_update(
        &mut joined_task,
        TaskStatus::Doing,
        "2026-06-12T00:00:00Z",
        &[" one ".to_string(), " two ".to_string()],
        &[],
        LifecycleNoteMode::Joined,
    )?;
    assert_eq!(joined_task.notes, vec!["one\ntwo".to_string()]);
    Ok(())
}

#[test]
fn load_active_task_trims_task_id_and_rejects_empty_ids() -> anyhow::Result<()> {
    let temp = tempfile::TempDir::new()?;
    let queue_path = temp.path().join("queue.jsonc");
    save_queue(&queue_path, &queue(vec![task("CL-0001")]))?;

    let loaded = load_active_task(&queue_path, "  CL-0001  ")?;

    assert_eq!(loaded.id, "CL-0001");
    let err = load_active_task(&queue_path, "   ").expect_err("empty IDs should fail");
    assert!(err.to_string().contains("Task ID cannot be empty"));
    Ok(())
}

#[test]
fn mutate_active_task_on_disk_mutates_only_active_task_and_returns_saved_state()
-> anyhow::Result<()> {
    let temp = tempfile::TempDir::new()?;
    let queue_path = temp.path().join("queue.jsonc");
    let done_path = temp.path().join("done.jsonc");
    save_queue(&queue_path, &queue(vec![task("CL-0001")]))?;
    save_queue(&done_path, &queue(vec![done_task("CL-0002")]))?;

    let updated = mutate_active_task_on_disk(
        &queue_path,
        &done_path,
        "  CL-0001  ",
        "CL",
        4,
        10,
        |task, now| {
            task.title = "Updated title".to_string();
            task.updated_at = Some(now.to_string());
            Ok(())
        },
    )?;

    assert_eq!(updated.id, "CL-0001");
    assert_eq!(updated.title, "Updated title");
    let saved = load_queue(&queue_path)?;
    assert_eq!(saved.tasks[0].title, "Updated title");
    let done = load_queue_or_default(&done_path)?;
    assert_eq!(done.tasks[0].title, "Test task");
    Ok(())
}

#[test]
fn mutate_active_task_on_disk_validation_failure_does_not_save() -> anyhow::Result<()> {
    let temp = tempfile::TempDir::new()?;
    let queue_path = temp.path().join("queue.jsonc");
    let done_path = temp.path().join("done.jsonc");
    save_queue(&queue_path, &queue(vec![task("CL-0001")]))?;
    save_queue(&done_path, &queue(vec![]))?;

    let err = mutate_active_task_on_disk(
        &queue_path,
        &done_path,
        "CL-0001",
        "CL",
        4,
        10,
        |task, _now| {
            task.id = "bad-id".to_string();
            Ok(())
        },
    )
    .expect_err("invalid queue should fail before save");

    assert!(
        err.to_string().contains("Mismatched task ID prefix"),
        "unexpected error: {err}"
    );
    let saved = load_queue(&queue_path)?;
    assert_eq!(saved.tasks[0].id, "CL-0001");
    Ok(())
}

#[test]
fn mutate_active_task_on_disk_save_only_preserves_unrelated_preexisting_invalidity()
-> anyhow::Result<()> {
    let temp = tempfile::TempDir::new()?;
    let queue_path = temp.path().join("queue.jsonc");
    let done_path = temp.path().join("done.jsonc");
    save_queue(&queue_path, &queue(vec![task("CL-0001"), task("bad-id")]))?;
    save_queue(&done_path, &queue(vec![]))?;

    let updated = mutate_active_task_on_disk_with_validation(
        &queue_path,
        &done_path,
        "CL-0001",
        "CL",
        4,
        10,
        ActiveTaskMutationValidation::SaveOnly,
        |task, now| {
            task.status = TaskStatus::Doing;
            task.updated_at = Some(now.to_string());
            Ok(())
        },
    )?;

    assert_eq!(updated.status, TaskStatus::Doing);
    let saved = load_queue(&queue_path)?;
    assert_eq!(saved.tasks[0].status, TaskStatus::Doing);
    assert_eq!(saved.tasks[1].id, "bad-id");
    Ok(())
}

#[test]
fn complete_active_task_to_archive_returns_reloaded_archived_task() -> anyhow::Result<()> {
    let temp = tempfile::TempDir::new()?;
    let queue_path = temp.path().join("queue.jsonc");
    let done_path = temp.path().join("done.jsonc");
    save_queue(&queue_path, &queue(vec![task("CL-0001")]))?;
    save_queue(&done_path, &queue(vec![]))?;

    let archived = complete_active_task_to_archive(
        &queue_path,
        &done_path,
        " CL-0001 ",
        TaskStatus::Done,
        &[" done note ".to_string()],
        &[" done evidence ".to_string()],
        "CL",
        4,
        10,
    )?;

    assert_eq!(archived.id, "CL-0001");
    assert_eq!(archived.status, TaskStatus::Done);
    assert_eq!(archived.notes, vec!["done note".to_string()]);
    assert_eq!(
        archived.evidence,
        vec!["observed".to_string(), "done evidence".to_string()]
    );
    assert!(load_queue(&queue_path)?.tasks.is_empty());
    assert_eq!(load_queue_or_default(&done_path)?.tasks[0].id, "CL-0001");
    Ok(())
}
