//! CLI handler for `ralph undo` command.
//!
//! Responsibilities:
//! - Handle `ralph undo` to restore the most recent snapshot.
//! - Handle `ralph undo --list` to show available snapshots.
//! - Handle `ralph undo --dry-run` to preview restores.
//! - Handle `ralph undo --id <id>` to restore a specific snapshot.
//!
//! Not handled here:
//! - Core undo logic (see `crate::undo`).
//! - Queue lock management (delegated to queue module).

use crate::config;
use crate::queue;
use crate::undo;
use anyhow::Result;
use clap::Args;

#[derive(Args, Debug)]
pub struct UndoArgs {
    /// Snapshot ID to restore (defaults to most recent).
    #[arg(long, short)]
    pub id: Option<String>,

    /// List available snapshots instead of restoring.
    #[arg(long)]
    pub list: bool,

    /// Preview restore without modifying files.
    #[arg(long)]
    pub dry_run: bool,

    /// Show verbose output.
    #[arg(long, short)]
    pub verbose: bool,
}

/// Handle the `ralph undo` command.
pub fn handle(args: UndoArgs, force: bool) -> Result<()> {
    let resolved = config::resolve_from_cwd()?;

    if args.list {
        return handle_list(&resolved);
    }

    let _lock = queue::acquire_queue_lock(&resolved.repo_root, "undo", force)?;

    let result = undo::restore_from_snapshot(&resolved, args.id.as_deref(), args.dry_run)?;

    if args.dry_run {
        println!("Dry run - would restore from snapshot:");
    } else {
        println!("Restored from snapshot:");
    }

    println!("  Operation: {}", result.operation);
    println!("  Timestamp: {}", result.timestamp);
    println!("  Tasks affected: {}", result.tasks_affected);

    if args.verbose && !args.dry_run {
        println!("\nRun `ralph queue list` to see the restored queue state.");
    }

    Ok(())
}

fn handle_list(resolved: &config::Resolved) -> Result<()> {
    let list = undo::list_undo_snapshots(&resolved.repo_root)?;

    if list.snapshots.is_empty() {
        println!("No undo snapshots available.");
        println!("\nSnapshots are created automatically before queue mutations such as:");
        println!("  - ralph task done/reject");
        println!("  - ralph queue archive");
        println!("  - ralph queue prune");
        println!("  - ralph task batch operations");
        println!("  - ralph task edit");
        return Ok(());
    }

    println!("Available undo snapshots (newest first):\n");

    for (i, snap) in list.snapshots.iter().enumerate() {
        let num = i + 1;
        println!("  {}. {} [{}]", num, snap.operation, snap.timestamp);
        println!("     ID: {}", snap.id);
    }

    println!("\nTo restore the most recent: ralph undo");
    println!("To restore a specific one:  ralph undo --id <ID>");
    println!("To preview without applying: ralph undo --dry-run");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contracts::{QueueFile, Task, TaskStatus};
    use std::collections::HashMap;
    use tempfile::TempDir;

    fn create_test_resolved(temp_dir: &TempDir) -> config::Resolved {
        let repo_root = temp_dir.path();
        let ralph_dir = repo_root.join(".ralph");
        std::fs::create_dir_all(&ralph_dir).unwrap();

        let queue_path = ralph_dir.join("queue.json");
        let done_path = ralph_dir.join("done.json");

        // Create initial queue with one task
        let queue = QueueFile {
            version: 1,
            tasks: vec![Task {
                id: "RQ-0001".to_string(),
                title: "Test task".to_string(),
                status: TaskStatus::Todo,
                description: None,
                priority: Default::default(),
                tags: vec!["test".to_string()],
                scope: vec!["crates/ralph".to_string()],
                evidence: vec!["observed".to_string()],
                plan: vec!["do thing".to_string()],
                notes: vec![],
                request: Some("test request".to_string()),
                agent: None,
                created_at: Some("2026-01-18T00:00:00Z".to_string()),
                updated_at: Some("2026-01-18T00:00:00Z".to_string()),
                completed_at: None,
                started_at: None,
                scheduled_start: None,
                depends_on: vec![],
                blocks: vec![],
                relates_to: vec![],
                duplicates: None,
                custom_fields: HashMap::new(),
                parent_id: None,
            }],
        };

        queue::save_queue(&queue_path, &queue).unwrap();

        config::Resolved {
            config: crate::contracts::Config::default(),
            repo_root: repo_root.to_path_buf(),
            queue_path,
            done_path,
            id_prefix: "RQ".to_string(),
            id_width: 4,
            global_config_path: None,
            project_config_path: None,
        }
    }

    #[test]
    fn handle_list_shows_snapshots() {
        let temp = TempDir::new().unwrap();
        let resolved = create_test_resolved(&temp);

        // Create a snapshot
        undo::create_undo_snapshot(&resolved, "test operation").unwrap();

        // Test handle_list
        let result = handle_list(&resolved);
        assert!(result.is_ok());
    }

    #[test]
    fn handle_list_empty_shows_helpful_message() {
        let temp = TempDir::new().unwrap();
        let resolved = create_test_resolved(&temp);

        // Test handle_list with no snapshots
        let result = handle_list(&resolved);
        assert!(result.is_ok());
    }
}
