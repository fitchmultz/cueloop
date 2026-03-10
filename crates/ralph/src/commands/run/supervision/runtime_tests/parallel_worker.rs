//! Parallel-worker bookkeeping restore regressions.
//!
//! Responsibilities:
//! - Verify worker post-run supervision restores queue/done/productivity bookkeeping files.
//! - Ensure bookkeeping restoration leaves the git worktree clean for those paths.
//!
//! Not handled here:
//! - Regular post-run supervision commit/push behavior.
//! - Continue-session resume flows.
//!
//! Invariants/assumptions:
//! - The test repo snapshots queue, done, and productivity files before dirtying them.

use super::support::{resolved_for_repo, write_queue};
use crate::commands::run::supervision::{PushPolicy, post_run_supervise_parallel_worker};
use crate::contracts::GitRevertMode;
use crate::contracts::{QueueFile, TaskStatus};
use crate::git;
use crate::queue;
use crate::testsupport::git as git_test;
use tempfile::TempDir;

#[test]
fn post_run_parallel_worker_restores_bookkeeping_without_signals() -> anyhow::Result<()> {
    let temp_dir = TempDir::new()?;
    let repo_root = temp_dir.path();
    git_test::init_repo(repo_root)?;

    let cache_dir = repo_root.join(".ralph/cache");
    std::fs::create_dir_all(&cache_dir)?;

    write_queue(repo_root, TaskStatus::Todo)?;
    queue::save_queue(
        &repo_root.join(".ralph/done.jsonc"),
        &QueueFile {
            version: 1,
            tasks: vec![],
        },
    )?;
    let productivity_path = cache_dir.join("productivity.json");
    std::fs::write(&productivity_path, r#"{"stats":[]}"#)?;
    git_test::commit_all(repo_root, "init queue/done/productivity")?;

    let resolved = resolved_for_repo(repo_root);
    let queue_before = std::fs::read_to_string(&resolved.queue_path)?;
    let done_before = std::fs::read_to_string(&resolved.done_path)?;
    let productivity_before = std::fs::read_to_string(&productivity_path)?;

    std::fs::write(&resolved.queue_path, r#"{"version":1,"tasks":[]}"#)?;
    std::fs::write(&resolved.done_path, r#"{"version":1,"tasks":[]}"#)?;
    std::fs::write(&productivity_path, r#"{"stats":["changed"]}"#)?;

    post_run_supervise_parallel_worker(
        &resolved,
        "RQ-0001",
        GitRevertMode::Disabled,
        false,
        PushPolicy::RequireUpstream,
        None,
        None,
        false,
        None,
    )?;

    assert_eq!(std::fs::read_to_string(&resolved.queue_path)?, queue_before);
    assert_eq!(std::fs::read_to_string(&resolved.done_path)?, done_before);
    assert_eq!(
        std::fs::read_to_string(&productivity_path)?,
        productivity_before
    );

    let status_paths = git::status_paths(repo_root)?;
    let queue_rel = resolved
        .queue_path
        .strip_prefix(repo_root)
        .unwrap()
        .to_string_lossy()
        .to_string();
    let done_rel = resolved
        .done_path
        .strip_prefix(repo_root)
        .unwrap()
        .to_string_lossy()
        .to_string();
    let productivity_rel = productivity_path
        .strip_prefix(repo_root)
        .unwrap()
        .to_string_lossy()
        .to_string();

    assert!(
        !status_paths.contains(&queue_rel),
        "queue.jsonc should be restored to HEAD"
    );
    assert!(
        !status_paths.contains(&done_rel),
        "done.jsonc should be restored to HEAD"
    );
    assert!(
        !status_paths.contains(&productivity_rel),
        "productivity.json should be restored to HEAD"
    );
    Ok(())
}
