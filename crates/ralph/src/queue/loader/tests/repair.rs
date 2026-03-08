//! Queue loader explicit-repair tests.

use super::*;

#[test]
fn repair_and_validate_queues_normalizes_non_utc_timestamps_and_persists() -> Result<()> {
    let temp = TempDir::new()?;
    let repo_root = temp.path();
    let ralph_dir = repo_root.join(".ralph");
    std::fs::create_dir_all(&ralph_dir)?;

    let queue_path = ralph_dir.join("queue.json");
    let done_path = ralph_dir.join("done.json");

    let mut active_task = task("RQ-0001");
    active_task.created_at = Some("2026-01-18T12:00:00-05:00".to_string());
    active_task.updated_at = Some("2026-01-18T13:00:00-05:00".to_string());
    save_queue(
        &queue_path,
        &QueueFile {
            version: 1,
            tasks: vec![active_task],
        },
    )?;

    let mut done_task = task("RQ-0002");
    done_task.status = TaskStatus::Done;
    done_task.created_at = Some("2026-01-18T10:00:00-07:00".to_string());
    done_task.updated_at = Some("2026-01-18T11:00:00-07:00".to_string());
    done_task.completed_at = Some("2026-01-18T12:00:00-07:00".to_string());
    save_queue(
        &done_path,
        &QueueFile {
            version: 1,
            tasks: vec![done_task],
        },
    )?;

    let resolved = resolved_with_paths(repo_root, queue_path.clone(), done_path.clone());

    let (queue, done) = repair_and_validate_queues(&resolved, true)?;
    let done = done.expect("done file should be present");

    let expected_active_created = crate::timeutil::format_rfc3339(crate::timeutil::parse_rfc3339(
        "2026-01-18T12:00:00-05:00",
    )?)?;
    let expected_done_completed = crate::timeutil::format_rfc3339(crate::timeutil::parse_rfc3339(
        "2026-01-18T12:00:00-07:00",
    )?)?;

    assert_eq!(
        queue.tasks[0].created_at.as_deref(),
        Some(expected_active_created.as_str())
    );
    assert_eq!(
        done.tasks[0].completed_at.as_deref(),
        Some(expected_done_completed.as_str())
    );

    let persisted_queue = load_queue(&queue_path)?;
    let persisted_done = load_queue(&done_path)?;
    assert_eq!(
        persisted_queue.tasks[0].created_at.as_deref(),
        Some(expected_active_created.as_str())
    );
    assert_eq!(
        persisted_done.tasks[0].completed_at.as_deref(),
        Some(expected_done_completed.as_str())
    );

    Ok(())
}

#[test]
fn repair_and_validate_queues_backfills_terminal_completed_at_and_persists() -> Result<()> {
    let temp = TempDir::new()?;
    let repo_root = temp.path();
    let ralph_dir = repo_root.join(".ralph");
    std::fs::create_dir_all(&ralph_dir)?;

    let queue_path = ralph_dir.join("queue.json");
    let done_path = ralph_dir.join("done.json");

    let mut queue_task = task("RQ-0001");
    queue_task.status = TaskStatus::Done;
    queue_task.completed_at = None;
    save_queue(
        &queue_path,
        &QueueFile {
            version: 1,
            tasks: vec![queue_task],
        },
    )?;
    save_queue(&done_path, &QueueFile::default())?;

    let resolved = resolved_with_paths(repo_root, queue_path.clone(), done_path);

    let (queue, _done) = repair_and_validate_queues(&resolved, true)?;
    let completed_at = queue.tasks[0]
        .completed_at
        .as_deref()
        .expect("completed_at should be backfilled");
    crate::timeutil::parse_rfc3339(completed_at)?;

    let persisted_queue = load_queue(&queue_path)?;
    let persisted_completed = persisted_queue.tasks[0]
        .completed_at
        .as_deref()
        .expect("completed_at should be saved");
    crate::timeutil::parse_rfc3339(persisted_completed)?;

    Ok(())
}
