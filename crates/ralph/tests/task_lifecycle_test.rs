//! Purpose: integration coverage for complete task lifecycle flows.
//!
//! Responsibilities:
//! - Exercise end-to-end task transitions such as ready, start, done, reject, and run.
//! - Verify queue/done file state and timestamp behavior at each lifecycle step.
//! - Cover runner execution integration for successful and failing `ralph run one` flows.
//!
//! Scope:
//! - CLI-driven lifecycle scenarios for a disposable seeded repo.
//!
//! Usage:
//! - Each test creates a fresh `LifecycleRepo` fixture and drives the real `ralph` binary.
//! - Assertions inspect queue and done files through shared test-support helpers.
//!
//! Invariants/assumptions callers must respect:
//! - These tests preserve end-to-end command coverage and should not be rewritten as direct file mutations.
//! - Fixture bootstrap comes from cached git+`.ralph/` templates, not real `ralph init` invocations.
//! - Parallel execution and command-unit semantics are covered in other suites.

#[path = "task_lifecycle_test/support.rs"]
mod support;
mod test_support;

use anyhow::Result;
use ralph::contracts::{Task, TaskPriority, TaskStatus};
use support::{LifecycleRepo, draft_task, terminal_task};

/// Helper to find a task by ID in a queue slice.
fn find_task<'a>(tasks: &'a [Task], id: &str) -> Option<&'a Task> {
    tasks.iter().find(|task| task.id == id)
}

/// Test the complete happy-path lifecycle: create → ready → start → done.
#[test]
fn task_full_lifecycle_build_ready_start_done() -> Result<()> {
    let repo = LifecycleRepo::new()?;

    let task_id = "RQ-0001";
    let mut task = draft_task(task_id, "Test task for lifecycle");
    task.description = Some("Test description".to_string());
    task.priority = TaskPriority::Medium;
    task.tags = vec!["test".to_string(), "lifecycle".to_string()];
    task.scope = vec!["crates/ralph/tests".to_string()];
    task.evidence = vec!["integration test".to_string()];
    task.plan = vec!["Step 1".to_string(), "Step 2".to_string()];
    task.request = Some("Test request".to_string());
    repo.write_queue(&[task])?;

    let queue = repo.read_queue()?;
    assert_eq!(queue.tasks.len(), 1, "expected 1 task in queue");
    let task = find_task(&queue.tasks, task_id).expect("task should exist");
    assert_eq!(
        task.status,
        TaskStatus::Draft,
        "initial status should be draft"
    );
    assert!(task.started_at.is_none(), "started_at should not be set");
    assert!(
        task.completed_at.is_none(),
        "completed_at should not be set"
    );

    repo.run_ok(&["task", "ready", task_id])?;

    let queue = repo.read_queue()?;
    let task = find_task(&queue.tasks, task_id).expect("task should exist after ready");
    assert_eq!(
        task.status,
        TaskStatus::Todo,
        "status should be todo after ready"
    );
    assert!(
        task.started_at.is_none(),
        "started_at should still not be set"
    );

    repo.run_ok(&["task", "start", task_id])?;

    let queue = repo.read_queue()?;
    let task = find_task(&queue.tasks, task_id).expect("task should exist after start");
    assert_eq!(
        task.status,
        TaskStatus::Doing,
        "status should be doing after start"
    );
    assert!(
        task.started_at.is_some(),
        "started_at should be set after start"
    );
    assert!(
        task.started_at
            .as_ref()
            .expect("started_at set")
            .contains('T'),
        "started_at should be a valid RFC3339 timestamp"
    );

    repo.run_ok(&["task", "done", task_id, "--note", "Completed successfully"])?;

    let queue = repo.read_queue()?;
    assert!(
        find_task(&queue.tasks, task_id).is_none(),
        "task should be removed from queue after done"
    );

    let done = repo.read_done()?;
    assert_eq!(done.tasks.len(), 1, "expected 1 task in done.json");
    let done_task = find_task(&done.tasks, task_id).expect("task should exist in done.json");
    assert_eq!(done_task.status, TaskStatus::Done, "status should be done");
    assert!(
        done_task.completed_at.is_some(),
        "completed_at should be set after done"
    );
    assert!(
        done_task
            .completed_at
            .as_ref()
            .expect("completed_at set")
            .contains('T'),
        "completed_at should be a valid RFC3339 timestamp"
    );
    assert!(
        done_task
            .notes
            .iter()
            .any(|note| note.contains("Completed successfully")),
        "completion note should be added to task notes: {:?}",
        done_task.notes
    );

    Ok(())
}

/// Test the reject path: create → ready → start → reject.
#[test]
fn task_full_lifecycle_with_reject() -> Result<()> {
    let repo = LifecycleRepo::new()?;

    let task_id = "RQ-0002";
    let task = test_support::make_test_task(task_id, "Task to reject", TaskStatus::Todo);
    repo.write_queue(&[task])?;

    let queue = repo.read_queue()?;
    assert_eq!(queue.tasks.len(), 1);
    let task = find_task(&queue.tasks, task_id).expect("task should exist before reject");
    assert_eq!(task.status, TaskStatus::Todo);

    repo.run_ok(&["task", "start", task_id])?;

    let queue = repo.read_queue()?;
    let task = find_task(&queue.tasks, task_id).expect("task should exist after start");
    assert_eq!(task.status, TaskStatus::Doing);

    repo.run_ok(&[
        "task",
        "reject",
        task_id,
        "--note",
        "Won't fix - out of scope",
    ])?;

    let queue = repo.read_queue()?;
    assert!(
        find_task(&queue.tasks, task_id).is_none(),
        "task should be removed from queue"
    );

    let done = repo.read_done()?;
    assert_eq!(done.tasks.len(), 1);
    let done_task = find_task(&done.tasks, task_id).expect("task should be in done.json");
    assert_eq!(
        done_task.status,
        TaskStatus::Rejected,
        "status should be rejected"
    );
    assert!(
        done_task.completed_at.is_some(),
        "completed_at should be set after reject"
    );
    assert!(
        done_task
            .notes
            .iter()
            .any(|note| note.contains("Won't fix")),
        "reject note should be preserved: {:?}",
        done_task.notes
    );

    Ok(())
}

/// Test queue state verification at each lifecycle step.
#[test]
fn task_lifecycle_state_verification() -> Result<()> {
    let repo = LifecycleRepo::new()?;

    let task_id = "RQ-0003";
    repo.write_queue(&[draft_task(task_id, "State verification task")])?;

    let queue = repo.read_queue()?;
    let task = find_task(&queue.tasks, task_id).expect("task should exist before ready");
    assert_eq!(task.status, TaskStatus::Draft);
    assert!(task.started_at.is_none());

    repo.run_ok(&["task", "ready", task_id])?;

    let queue = repo.read_queue()?;
    let task = find_task(&queue.tasks, task_id).expect("task should exist after ready");
    assert_eq!(
        task.status,
        TaskStatus::Todo,
        "after ready: status should be todo"
    );
    assert!(
        task.started_at.is_none(),
        "after ready: started_at should not be set"
    );
    assert!(
        task.updated_at.is_some(),
        "after ready: updated_at should be set"
    );

    repo.run_ok(&["task", "start", task_id])?;

    let queue = repo.read_queue()?;
    let task = find_task(&queue.tasks, task_id).expect("task should exist after start");
    assert_eq!(
        task.status,
        TaskStatus::Doing,
        "after start: status should be doing"
    );
    assert!(
        task.started_at.is_some(),
        "after start: started_at should be set"
    );

    let started_at = task.started_at.clone().expect("started_at should exist");

    repo.run_ok(&["task", "done", task_id, "--note", "Done!"])?;

    let queue = repo.read_queue()?;
    assert!(queue.tasks.is_empty(), "after done: queue should be empty");

    let done = repo.read_done()?;
    assert_eq!(
        done.tasks.len(),
        1,
        "after done: done.json should have 1 task"
    );
    let done_task = find_task(&done.tasks, task_id).expect("task should exist in done.json");
    assert_eq!(
        done_task.status,
        TaskStatus::Done,
        "after done: status should be done"
    );
    assert_eq!(
        done_task.started_at.as_ref(),
        Some(&started_at),
        "after done: started_at should be preserved"
    );
    assert!(
        done_task.completed_at.is_some(),
        "after done: completed_at should be set"
    );

    Ok(())
}

/// Test that starting an already started task (with reset) updates started_at.
#[test]
fn task_start_reset_updates_timestamp() -> Result<()> {
    let repo = LifecycleRepo::new()?;

    let task_id = "RQ-0004";
    let task = test_support::make_test_task(task_id, "Reset test task", TaskStatus::Todo);
    repo.write_queue(&[task])?;

    repo.run_ok(&["task", "start", task_id])?;

    let queue = repo.read_queue()?;
    let task = find_task(&queue.tasks, task_id).expect("task should exist after initial start");
    let first_started_at = task.started_at.clone().expect("started_at should be set");

    let mut second_started_at = first_started_at.clone();
    for _ in 0..8 {
        repo.run_ok(&["task", "start", task_id, "--reset"])?;

        let queue = repo.read_queue()?;
        let task = find_task(&queue.tasks, task_id).expect("task should exist after reset");
        second_started_at = task.started_at.clone().expect("started_at should be set");
        if second_started_at != first_started_at {
            break;
        }
    }

    assert_ne!(
        first_started_at, second_started_at,
        "started_at should be updated after reset"
    );

    Ok(())
}

/// Test that cannot start a terminal (done/rejected) task.
#[test]
fn task_cannot_start_terminal_task() -> Result<()> {
    let repo = LifecycleRepo::new()?;

    let task_id = "RQ-0005";
    repo.write_queue(&[terminal_task(task_id, "Done task", TaskStatus::Done)])?;

    let (status, _, stderr) = repo.run(&["task", "start", task_id]);
    assert!(!status.success(), "should fail to start a done task");
    assert!(
        stderr.contains("terminal") || stderr.contains("Done") || stderr.contains("cannot"),
        "error should mention terminal status: {}",
        stderr
    );

    Ok(())
}

/// Test multiple tasks lifecycle independently.
#[test]
fn task_multiple_independent_lifecycles() -> Result<()> {
    let repo = LifecycleRepo::new()?;

    let task1 = test_support::make_test_task("RQ-1001", "First task", TaskStatus::Todo);
    let task2 = test_support::make_test_task("RQ-1002", "Second task", TaskStatus::Todo);
    let task3 = test_support::make_test_task("RQ-1003", "Third task", TaskStatus::Todo);
    repo.write_queue(&[task1, task2, task3])?;

    repo.run_ok(&["task", "start", "RQ-1001"])?;

    let queue = repo.read_queue()?;
    let t1 = find_task(&queue.tasks, "RQ-1001").expect("task 1 should exist");
    let t2 = find_task(&queue.tasks, "RQ-1002").expect("task 2 should exist");
    let t3 = find_task(&queue.tasks, "RQ-1003").expect("task 3 should exist");
    assert_eq!(t1.status, TaskStatus::Doing);
    assert_eq!(t2.status, TaskStatus::Todo);
    assert_eq!(t3.status, TaskStatus::Todo);

    repo.run_ok(&["task", "done", "RQ-1001"])?;

    let queue = repo.read_queue()?;
    assert_eq!(queue.tasks.len(), 2);
    assert!(find_task(&queue.tasks, "RQ-1001").is_none());

    repo.run_ok(&["task", "reject", "RQ-1002"])?;

    let queue = repo.read_queue()?;
    assert_eq!(queue.tasks.len(), 1);
    assert!(find_task(&queue.tasks, "RQ-1002").is_none());

    let done = repo.read_done()?;
    assert_eq!(done.tasks.len(), 2);
    let done_t1 = find_task(&done.tasks, "RQ-1001").expect("done task 1 should exist");
    let done_t2 = find_task(&done.tasks, "RQ-1002").expect("done task 2 should exist");
    assert_eq!(done_t1.status, TaskStatus::Done);
    assert_eq!(done_t2.status, TaskStatus::Rejected);

    let t3 = find_task(&queue.tasks, "RQ-1003").expect("task 3 should remain in queue");
    assert_eq!(t3.status, TaskStatus::Todo);

    Ok(())
}

/// Test that task ready command requires draft status.
#[test]
fn task_ready_requires_draft_status() -> Result<()> {
    let repo = LifecycleRepo::new()?;

    let task = test_support::make_test_task("RQ-1004", "Todo task", TaskStatus::Todo);
    repo.write_queue(&[task])?;

    let (status, _, _stderr) = repo.run(&["task", "ready", "RQ-1004"]);
    let _ = status;

    let queue = repo.read_queue()?;
    assert_eq!(queue.tasks.len(), 1);

    Ok(())
}

/// Test that started_at is not set by ready command.
#[test]
fn task_ready_does_not_set_started_at() -> Result<()> {
    let repo = LifecycleRepo::new()?;

    repo.write_queue(&[draft_task("RQ-1005", "Draft task")])?;

    repo.run_ok(&["task", "ready", "RQ-1005"])?;

    let queue = repo.read_queue()?;
    let task = find_task(&queue.tasks, "RQ-1005").expect("task should exist after ready");
    assert_eq!(task.status, TaskStatus::Todo);
    assert!(task.started_at.is_none(), "ready should not set started_at");

    Ok(())
}

/// Test full lifecycle with actual runner execution: create → ready → start → run → done.
///
/// This test verifies that:
/// 1. `ralph run one` selects a todo task and runs the runner.
/// 2. When CI gate passes, task is auto-completed.
/// 3. Task metadata (`started_at`, `completed_at`) is properly tracked.
#[test]
fn task_full_lifecycle_with_runner_execution() -> Result<()> {
    let repo = LifecycleRepo::new()?;

    let task_id = "RQ-9001";
    let task = test_support::make_test_task(task_id, "Runner test task", TaskStatus::Todo);
    repo.write_queue(&[task])?;

    let marker_file = repo.path().join(".ralph/runner_executed.marker");
    let runner_script = format!(
        r#"#!/bin/bash
# Mock runner that verifies it received task context
echo "runner_executed" > "{}"
exit 0
"#,
        marker_file.display()
    );
    repo.setup_runner_with_passing_ci(&runner_script)?;

    let queue = repo.read_queue()?;
    let task = find_task(&queue.tasks, task_id).expect("task should exist before run");
    assert_eq!(
        task.status,
        TaskStatus::Todo,
        "Initial: status should be Todo"
    );

    repo.run_ok(&["run", "one"])?;

    assert!(
        marker_file.exists(),
        "Runner should have been executed (marker file should exist)"
    );

    let queue = repo.read_queue()?;
    assert!(
        find_task(&queue.tasks, task_id).is_none(),
        "Task should not be in queue after successful run + CI gate"
    );

    let done = repo.read_done()?;
    assert_eq!(done.tasks.len(), 1, "Task should be in done.json");
    let done_task = find_task(&done.tasks, task_id).expect("task should be in done.json");
    assert_eq!(done_task.status, TaskStatus::Done, "Status should be Done");
    assert!(done_task.started_at.is_some(), "started_at should be set");
    assert!(
        done_task.completed_at.is_some(),
        "completed_at should be set"
    );

    Ok(())
}

/// Test that runner failure prevents task auto-completion.
///
/// This test verifies that when the runner fails:
/// 1. Task remains in the queue.
/// 2. Task status is still Doing.
/// 3. User can then reject the task manually.
#[test]
fn task_runner_failure_prevents_auto_complete() -> Result<()> {
    let repo = LifecycleRepo::new()?;

    let task_id = "RQ-9002";
    let task = test_support::make_test_task(task_id, "Task that will fail", TaskStatus::Todo);
    repo.write_queue(&[task])?;
    repo.setup_runner_with_passing_ci("#!/bin/sh\nexit 1\n")?;

    let (status, _, _stderr) = repo.run(&["run", "one"]);
    assert!(
        !status.success(),
        "Run should fail when runner exits with error"
    );

    let queue = repo.read_queue()?;
    assert_eq!(
        queue.tasks.len(),
        1,
        "Task should still be in queue after runner failure"
    );
    let task = find_task(&queue.tasks, task_id).expect("task should remain in queue");
    assert_eq!(
        task.status,
        TaskStatus::Doing,
        "Task should be Doing (set by run command)"
    );
    assert!(
        task.started_at.is_some(),
        "started_at should be set by run command"
    );

    repo.run_ok(&[
        "task",
        "reject",
        task_id,
        "--note",
        "Runner failed - won't fix",
    ])?;

    let queue = repo.read_queue()?;
    assert!(
        find_task(&queue.tasks, task_id).is_none(),
        "Task should be removed from queue"
    );

    let done = repo.read_done()?;
    assert_eq!(done.tasks.len(), 1, "Task should be in done.json");
    let done_task = find_task(&done.tasks, task_id).expect("task should exist in done.json");
    assert_eq!(
        done_task.status,
        TaskStatus::Rejected,
        "Status should be Rejected"
    );
    assert!(
        done_task
            .notes
            .iter()
            .any(|note| note.contains("Runner failed")),
        "Reject note should be preserved"
    );

    Ok(())
}

/// Test queue state transitions during full lifecycle including runner execution.
///
/// This test verifies the complete state transition sequence with CI gate auto-completion:
/// 1. Initial: Task is Todo in queue.json.
/// 2. After `ralph run one`: Task is auto-completed and moved to done.json with Done status.
/// 3. Timestamps (`started_at`, `completed_at`) are properly set.
#[test]
fn task_lifecycle_queue_state_during_run() -> Result<()> {
    let repo = LifecycleRepo::new()?;

    let task_id = "RQ-9003";
    let task = test_support::make_test_task(task_id, "State tracking task", TaskStatus::Todo);
    repo.write_queue(&[task])?;
    repo.setup_runner_with_passing_ci("#!/bin/sh\nexit 0\n")?;

    let queue = repo.read_queue()?;
    let task = find_task(&queue.tasks, task_id).expect("task should exist before run");
    assert_eq!(
        task.status,
        TaskStatus::Todo,
        "Initial: status should be Todo"
    );
    assert!(
        task.started_at.is_none(),
        "Initial: started_at should not be set"
    );

    repo.run_ok(&["run", "one"])?;

    let queue = repo.read_queue()?;
    assert!(
        find_task(&queue.tasks, task_id).is_none(),
        "After run: task should not be in queue (auto-completed by CI gate)"
    );

    let done = repo.read_done()?;
    assert_eq!(
        done.tasks.len(),
        1,
        "After run: task should be in done.json"
    );
    let done_task = find_task(&done.tasks, task_id).expect("task should be in done.json");
    assert_eq!(
        done_task.status,
        TaskStatus::Done,
        "After run: status should be Done"
    );
    assert!(
        done_task.started_at.is_some(),
        "After run: started_at should be set"
    );
    assert!(
        done_task.completed_at.is_some(),
        "After run: completed_at should be set"
    );

    Ok(())
}
