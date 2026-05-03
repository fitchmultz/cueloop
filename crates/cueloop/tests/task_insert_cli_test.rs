//! Integration tests for `cueloop task insert`.
//!
//! Purpose:
//! - Verify atomic task insertion allocates IDs under the queue lock.
//!
//! Responsibilities:
//! - Cover success, dry-run, sequential insertions, and live lock rejection.

mod test_support;

use anyhow::Result;
use cueloop::config::project_runtime_dir;
use cueloop::contracts::{
    QueueFile, TASK_INSERT_VERSION, TaskInsertRequest, TaskInsertSpec, TaskStatus,
};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;

fn insert_spec(key: &str, title: &str) -> TaskInsertSpec {
    TaskInsertSpec {
        key: key.to_string(),
        title: title.to_string(),
        description: Some(format!("{title} description")),
        priority: Default::default(),
        status: TaskStatus::Todo,
        kind: Default::default(),
        tags: vec!["scan".to_string()],
        scope: vec!["docs/queue-and-tasks.md".to_string()],
        evidence: vec!["path: docs/queue-and-tasks.md :: atomic insert".to_string()],
        plan: vec!["Insert task".to_string(), "Run queue validate".to_string()],
        notes: vec![],
        request: Some("scan: queue safety".to_string()),
        depends_on_keys: vec![],
        depends_on: vec![],
        blocks: vec![],
        relates_to: vec![],
        duplicates: None,
        parent_key: None,
        parent_id: None,
        custom_fields: HashMap::new(),
        estimated_minutes: None,
        agent: None,
    }
}

fn write_request(
    dir: &std::path::Path,
    name: &str,
    tasks: Vec<TaskInsertSpec>,
) -> Result<std::path::PathBuf> {
    let path = dir.join(name);
    let request = TaskInsertRequest {
        version: TASK_INSERT_VERSION,
        tasks,
    };
    fs::write(&path, serde_json::to_string_pretty(&request)?)?;
    Ok(path)
}

fn read_queue(dir: &std::path::Path) -> Result<QueueFile> {
    let path = project_runtime_dir(dir).join("queue.jsonc");
    Ok(serde_json::from_str(&fs::read_to_string(path)?)?)
}

#[test]
fn task_insert_creates_tasks_and_queue_validates() -> Result<()> {
    let dir = test_support::temp_dir_outside_repo();
    test_support::git_init(dir.path())?;
    test_support::seed_cueloop_dir(dir.path())?;

    let request_path = write_request(
        dir.path(),
        "task-insert.json",
        vec![insert_spec("alpha", "Alpha"), insert_spec("beta", "Beta")],
    )?;

    let (status, stdout, stderr) = test_support::run_in_dir(
        dir.path(),
        &[
            "task",
            "insert",
            "--format",
            "json",
            "--input",
            request_path.to_str().expect("utf-8 request path"),
        ],
    );
    anyhow::ensure!(
        status.success(),
        "task insert failed\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );

    let document: Value = serde_json::from_str(&stdout)?;
    assert_eq!(document["version"], TASK_INSERT_VERSION);
    assert_eq!(document["created_count"], 2);
    assert_eq!(document["tasks"][0]["task"]["id"], "RQ-0001");
    assert_eq!(document["tasks"][1]["task"]["id"], "RQ-0002");

    let (validate_status, validate_stdout, validate_stderr) =
        test_support::run_in_dir(dir.path(), &["queue", "validate"]);
    anyhow::ensure!(
        validate_status.success(),
        "queue validate failed after task insert\nstdout:\n{validate_stdout}\nstderr:\n{validate_stderr}"
    );

    let queue = read_queue(dir.path())?;
    assert_eq!(queue.tasks.len(), 2);
    Ok(())
}

#[test]
fn task_insert_dry_run_does_not_mutate_queue() -> Result<()> {
    let dir = test_support::temp_dir_outside_repo();
    test_support::git_init(dir.path())?;
    test_support::seed_cueloop_dir(dir.path())?;

    let before = read_queue(dir.path())?;
    let request_path = write_request(
        dir.path(),
        "task-insert-dry-run.json",
        vec![insert_spec("alpha", "Alpha")],
    )?;

    let (status, stdout, stderr) = test_support::run_in_dir(
        dir.path(),
        &[
            "task",
            "insert",
            "--dry-run",
            "--format",
            "json",
            "--input",
            request_path.to_str().expect("utf-8 request path"),
        ],
    );
    anyhow::ensure!(
        status.success(),
        "task insert --dry-run failed\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );

    let document: Value = serde_json::from_str(&stdout)?;
    assert_eq!(document["dry_run"], true);
    assert_eq!(document["tasks"][0]["task"]["id"], "RQ-0001");
    assert_eq!(read_queue(dir.path())?.tasks.len(), before.tasks.len());
    Ok(())
}

#[test]
fn task_insert_allocates_fresh_ids_for_sequential_requests() -> Result<()> {
    let dir = test_support::temp_dir_outside_repo();
    test_support::git_init(dir.path())?;
    test_support::seed_cueloop_dir(dir.path())?;

    let first = write_request(
        dir.path(),
        "first.json",
        vec![insert_spec("alpha", "Alpha")],
    )?;
    let second = write_request(
        dir.path(),
        "second.json",
        vec![insert_spec("alpha", "Alpha again")],
    )?;

    let (first_status, first_stdout, first_stderr) = test_support::run_in_dir(
        dir.path(),
        &[
            "task",
            "insert",
            "--format",
            "json",
            "--input",
            first.to_str().unwrap(),
        ],
    );
    anyhow::ensure!(
        first_status.success(),
        "first task insert failed\nstdout:\n{first_stdout}\nstderr:\n{first_stderr}"
    );

    let (second_status, second_stdout, second_stderr) = test_support::run_in_dir(
        dir.path(),
        &[
            "task",
            "insert",
            "--format",
            "json",
            "--input",
            second.to_str().unwrap(),
        ],
    );
    anyhow::ensure!(
        second_status.success(),
        "second task insert failed\nstdout:\n{second_stdout}\nstderr:\n{second_stderr}"
    );

    let first_document: Value = serde_json::from_str(&first_stdout)?;
    let second_document: Value = serde_json::from_str(&second_stdout)?;
    assert_eq!(first_document["tasks"][0]["task"]["id"], "RQ-0001");
    assert_eq!(second_document["tasks"][0]["task"]["id"], "RQ-0002");

    let (validate_status, _validate_stdout, validate_stderr) =
        test_support::run_in_dir(dir.path(), &["queue", "validate"]);
    anyhow::ensure!(
        validate_status.success(),
        "queue validate failed after sequential inserts\nstderr:\n{validate_stderr}"
    );
    Ok(())
}

#[test]
fn task_insert_fails_cleanly_when_queue_lock_is_held() -> Result<()> {
    let dir = test_support::temp_dir_outside_repo();
    test_support::git_init(dir.path())?;
    test_support::seed_cueloop_dir(dir.path())?;

    let _lock = cueloop::queue::acquire_queue_lock(dir.path(), "integration-test", false)?;
    let request_path = write_request(
        dir.path(),
        "lock-contention.json",
        vec![insert_spec("alpha", "Alpha")],
    )?;

    let (status, _stdout, stderr) = test_support::run_in_dir(
        dir.path(),
        &["task", "insert", "--input", request_path.to_str().unwrap()],
    );
    anyhow::ensure!(
        !status.success(),
        "task insert unexpectedly succeeded while lock held"
    );
    assert!(stderr.contains("queue") || stderr.contains("lock"));
    assert!(read_queue(dir.path())?.tasks.is_empty());
    Ok(())
}
