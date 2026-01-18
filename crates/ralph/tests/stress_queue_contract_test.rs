use anyhow::{Context, Result};
use ralph::contracts::{QueueFile, Task, TaskStatus};
use ralph::queue;
use std::path::PathBuf;
use tempfile::TempDir;

const ID_WIDTH: usize = 5;

fn make_task(id_num: u32, status: TaskStatus) -> Task {
    Task {
        id: format!("RQ-{id_num:0width$}", width = ID_WIDTH),
        status,
        title: format!("Task {id_num}"),
        tags: vec!["rust".to_string()],
        scope: vec!["crates/ralph".to_string()],
        evidence: vec!["stress fixture".to_string()],
        plan: vec!["exercise queue ops".to_string()],
        notes: vec![],
        request: None,
        agent: None,
        created_at: None,
        updated_at: None,
        completed_at: None,
        blocked_reason: None,
    }
}

fn write_queue_files(
    dir: &TempDir,
    active: &QueueFile,
    done: &QueueFile,
) -> Result<(PathBuf, PathBuf)> {
    let queue_path = dir.path().join("queue.yaml");
    let done_path = dir.path().join("done.yaml");
    queue::save_queue(&queue_path, active).with_context(|| "save active queue")?;
    queue::save_queue(&done_path, done).with_context(|| "save done queue")?;
    Ok((queue_path, done_path))
}

#[test]
fn stress_queue_ops_large_scale() -> Result<()> {
    let mut active = QueueFile {
        version: 1,
        tasks: Vec::new(),
    };
    let mut done = QueueFile {
        version: 1,
        tasks: Vec::new(),
    };

    // 10,000 tasks total, split across queue and done with non-overlapping IDs.
    for i in 1..=5000u32 {
        active.tasks.push(make_task(i, TaskStatus::Todo));
    }
    for i in 5001..=10000u32 {
        done.tasks.push(make_task(i, TaskStatus::Done));
    }

    queue::validate_queue_set(&active, Some(&done), "RQ", ID_WIDTH)
        .context("validate queue set")?;

    let next =
        queue::next_id_across(&active, Some(&done), "RQ", ID_WIDTH).context("next id across")?;
    anyhow::ensure!(next == "RQ-10001", "unexpected next id: {next}");

    // Serialize and parse roundtrip.
    let dir = TempDir::new().context("create temp dir")?;
    let (queue_path, done_path) = write_queue_files(&dir, &active, &done)?;

    let (reloaded_active, repaired_active) =
        queue::load_queue_with_repair(&queue_path).context("load active")?;
    anyhow::ensure!(!repaired_active, "unexpected repair on valid YAML (active)");

    let (reloaded_done, repaired_done) =
        queue::load_queue_with_repair(&done_path).context("load done")?;
    anyhow::ensure!(!repaired_done, "unexpected repair on valid YAML (done)");

    queue::validate_queue_set(&reloaded_active, Some(&reloaded_done), "RQ", ID_WIDTH)
        .context("validate reloaded queue set")?;

    Ok(())
}

#[test]
#[ignore]
fn stress_queue_ops_burn_in_long() -> Result<()> {
    // Burn-in: smaller queue, repeated archive + status updates + reload.
    // This is intentionally long-running and is executed by `make test` (which includes ignored tests).
    let dir = TempDir::new().context("create temp dir")?;
    let queue_path = dir.path().join("queue.yaml");
    let done_path = dir.path().join("done.yaml");

    let mut active = QueueFile {
        version: 1,
        tasks: Vec::new(),
    };
    let done = QueueFile {
        version: 1,
        tasks: Vec::new(),
    };

    // 2,000 tasks in active: first 200 start as done (eligible for archiving).
    for i in 1..=2000u32 {
        let status = if i <= 200 {
            TaskStatus::Done
        } else {
            TaskStatus::Todo
        };
        active.tasks.push(make_task(i, status));
    }

    queue::save_queue(&queue_path, &active).context("save initial active")?;
    queue::save_queue(&done_path, &done).context("save initial done")?;

    // Run a bounded number of iterations; each iteration archives done tasks and marks a few todo as done.
    for iter in 0..25u32 {
        let report = queue::archive_done_tasks(&queue_path, &done_path, "RQ", ID_WIDTH)
            .with_context(|| format!("archive iteration {iter}"))?;
        let _ = report;

        let mut current = queue::load_queue(&queue_path).context("load active")?;
        let now = "2026-01-18T00:00:00Z";

        // Mark a deterministic slice of todo tasks as done each iteration.
        let start = 201 + iter * 10;
        for id_num in start..start + 5 {
            let id = format!("RQ-{id_num:0width$}", width = ID_WIDTH);
            let _ = queue::set_status(&mut current, &id, TaskStatus::Done, now, None, None);
        }

        queue::save_queue(&queue_path, &current).context("save active")?;

        // Reload both and validate invariants.
        let active_reloaded = queue::load_queue(&queue_path).context("reload active")?;
        let done_reloaded = queue::load_queue_or_default(&done_path).context("reload done")?;
        queue::validate_queue_set(&active_reloaded, Some(&done_reloaded), "RQ", ID_WIDTH)
            .context("validate after iteration")?;
    }

    Ok(())
}
