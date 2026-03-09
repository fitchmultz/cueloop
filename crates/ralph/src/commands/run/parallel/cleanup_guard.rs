//! Cleanup guard for parallel run loop to ensure resources are cleaned up on any exit path.
//!
//! Responsibilities:
//! - Own and manage resources that need cleanup: in-flight workers,
//!   workspace directories, and parallel state.
//! - Perform best-effort cleanup on Drop to prevent resource leaks on early returns.
//!
//! Not handled here:
//! - Actual worker execution logic (see `super::worker`).
//! - Integration loop execution (see `super::integration`).
//! - State persistence format (see `super::state`).
//!
//! Invariants/assumptions:
//! - Cleanup is idempotent and best-effort; errors are logged but not propagated.
//! - Drop must never panic (no RefCell to avoid panic during unwinding).
//! - The guard owns resources and releases them during cleanup.

use crate::commands::run::parallel::state;
use crate::commands::run::parallel::worker::{FinishedWorker, WorkerState, terminate_workers};
use crate::commands::run::parallel::workspace_cleanup::remove_workspace_best_effort;
use crate::git::WorkspaceSpec;
use anyhow::Result;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::Duration;

/// Guard that ensures cleanup of parallel run resources on any exit path.
///
/// This guard owns all resources that must be cleaned up, so early returns
/// via `?` still trigger cleanup through the Drop implementation.
///
/// Uses &mut self methods (no RefCell) to guarantee Drop never panics.
pub(crate) struct ParallelCleanupGuard {
    /// Path to the parallel state file.
    state_path: PathBuf,
    /// In-memory parallel state (persisted during cleanup).
    state_file: state::ParallelStateFile,
    /// Map of in-flight worker processes (terminated during cleanup).
    in_flight: HashMap<String, WorkerState>,
    /// Map of all known workspaces (removed during cleanup).
    workspaces: HashMap<String, WorkspaceSpec>,
    /// Root directory for workspaces.
    workspace_root: PathBuf,
    /// Worker-exit event channel shared with monitor threads.
    worker_events_tx: Sender<FinishedWorker>,
    worker_events_rx: Receiver<FinishedWorker>,
    /// Whether cleanup has already been performed.
    completed: bool,
}

impl ParallelCleanupGuard {
    /// Create a new cleanup guard for direct-push parallel orchestration.
    pub fn new_simple(
        state_path: PathBuf,
        state_file: state::ParallelStateFile,
        workspace_root: PathBuf,
    ) -> Self {
        let (worker_events_tx, worker_events_rx) = mpsc::channel();
        Self {
            state_path,
            state_file,
            in_flight: HashMap::new(),
            workspaces: HashMap::new(),
            workspace_root,
            worker_events_tx,
            worker_events_rx,
            completed: false,
        }
    }

    /// Mark cleanup as completed successfully (disarms the guard).
    ///
    /// After calling this, Drop will be a no-op.
    pub fn mark_completed(&mut self) {
        self.completed = true;
    }

    /// Get mutable access to the state file.
    pub fn state_file_mut(&mut self) -> &mut state::ParallelStateFile {
        &mut self.state_file
    }

    /// Get immutable access to the state file.
    pub fn state_file(&self) -> &state::ParallelStateFile {
        &self.state_file
    }

    /// Get immutable access to in-flight workers.
    pub fn in_flight(&self) -> &HashMap<String, WorkerState> {
        &self.in_flight
    }

    /// Clone the shared worker-event sender for new monitor threads.
    pub fn worker_event_sender(&self) -> Sender<FinishedWorker> {
        self.worker_events_tx.clone()
    }

    /// Drain worker-exit events that are already available.
    pub fn drain_finished_workers(&mut self) -> Vec<FinishedWorker> {
        let mut finished = Vec::new();
        while let Ok(event) = self.worker_events_rx.try_recv() {
            finished.push(event);
        }
        finished
    }

    /// Wait for at least one worker-exit event up to the provided timeout, then drain the rest.
    pub fn wait_for_finished_workers(&mut self, timeout: Duration) -> Vec<FinishedWorker> {
        match self.worker_events_rx.recv_timeout(timeout) {
            Ok(first) => {
                let mut finished = Vec::new();
                finished.push(first);
                finished.extend(self.drain_finished_workers());
                finished
            }
            Err(mpsc::RecvTimeoutError::Timeout) => Vec::new(),
            Err(mpsc::RecvTimeoutError::Disconnected) => Vec::new(),
        }
    }

    /// Register a workspace for cleanup.
    pub fn register_workspace(&mut self, task_id: String, spec: WorkspaceSpec) {
        self.workspaces.insert(task_id, spec);
    }

    /// Register an in-flight worker for cleanup.
    pub fn register_worker(&mut self, task_id: String, worker: WorkerState) {
        self.in_flight.insert(task_id, worker);
    }

    /// Remove a worker from cleanup tracking (e.g., after completion).
    pub fn remove_worker(&mut self, task_id: &str) -> Option<WorkerState> {
        self.in_flight.remove(task_id)
    }

    /// Perform full cleanup and return any error.
    ///
    /// This is idempotent - safe to call multiple times.
    pub fn cleanup(&mut self) -> Result<()> {
        if self.completed {
            return Ok(());
        }

        log::debug!("ParallelCleanupGuard: performing cleanup");

        // Step 1: Terminate in-flight workers
        terminate_workers(&mut self.in_flight);

        // Step 2: Remove tracked workspaces except blocked_push workspaces.
        // Blocked workspaces are retained for explicit operator retry.
        let blocked_task_ids: HashSet<String> = self
            .state_file
            .workers
            .iter()
            .filter(|worker| matches!(worker.lifecycle, state::WorkerLifecycle::BlockedPush))
            .map(|worker| worker.task_id.trim().to_string())
            .collect();
        for (task_id, spec) in &self.workspaces {
            if blocked_task_ids.contains(task_id.trim()) {
                continue;
            }
            if spec.path.exists() {
                remove_workspace_best_effort(&self.workspace_root, spec, "cleanup guard");
            }
        }

        // Step 3: Drop non-terminal workers and persist state.
        // Terminal workers are retained for status/retry visibility.
        self.state_file
            .workers
            .retain(state::WorkerRecord::is_terminal);
        if let Err(err) = state::save_state(&self.state_path, &self.state_file) {
            log::warn!("Failed to save parallel state during cleanup: {:#}", err);
        }

        self.completed = true;
        Ok(())
    }

    /// Perform cleanup, logging and suppressing any errors.
    ///
    /// This is called from Drop to ensure cleanup never panics.
    fn cleanup_best_effort(&mut self) {
        if let Err(err) = self.cleanup() {
            log::warn!("ParallelCleanupGuard: cleanup error: {:#}", err);
        }
    }
}

impl Drop for ParallelCleanupGuard {
    fn drop(&mut self) {
        // Ensure cleanup runs even if the guard is dropped without explicit cleanup call
        self.cleanup_best_effort();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::run::parallel::worker::start_worker_monitor;
    use crate::lock;
    use std::process::{Child, Command};
    use tempfile::TempDir;

    fn create_test_guard(temp: &TempDir) -> ParallelCleanupGuard {
        let workspace_root = temp.path().join("workspaces");
        std::fs::create_dir_all(&workspace_root).unwrap();

        let state_path = temp.path().join("state.json");
        let state_file =
            state::ParallelStateFile::new("2026-02-20T00:00:00Z".to_string(), "main".to_string());

        ParallelCleanupGuard::new_simple(state_path, state_file, workspace_root)
    }

    fn register_sleeping_worker(
        guard: &mut ParallelCleanupGuard,
        temp: &TempDir,
        task_id: &str,
    ) -> Result<u32> {
        let child: Child = Command::new("sleep").arg("10").spawn()?;
        let pid = child.id();
        let workspace_path = temp.path().join("workspaces").join(task_id);
        std::fs::create_dir_all(&workspace_path)?;

        let worker = start_worker_monitor(
            task_id,
            "Test task".to_string(),
            WorkspaceSpec {
                path: workspace_path,
                branch: "main".to_string(),
            },
            child,
            guard.worker_event_sender(),
        );
        guard.register_worker(task_id.to_string(), worker);
        Ok(pid)
    }

    #[cfg(unix)]
    fn kill_test_process(pid: u32) {
        // SAFETY: test-owned child pid; failure is tolerated because the process may already exit.
        unsafe {
            let _ = libc::kill(pid as i32, libc::SIGKILL);
        }
    }

    #[cfg(windows)]
    fn kill_test_process(pid: u32) {
        let _ = Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .status();
    }

    #[cfg(all(not(unix), not(windows)))]
    fn kill_test_process(_pid: u32) {}

    #[test]
    fn guard_cleanup_kills_worker_and_clears_state() -> Result<()> {
        let temp = TempDir::new()?;
        let mut guard = create_test_guard(&temp);

        let pid = register_sleeping_worker(&mut guard, &temp, "RQ-0001")?;
        let workspace_path = temp.path().join("workspaces").join("RQ-0001");
        guard
            .state_file_mut()
            .upsert_worker(state::WorkerRecord::new(
                "RQ-0001",
                workspace_path.clone(),
                "2026-02-20T00:00:00Z".to_string(),
            ));

        // Verify worker is running
        assert_eq!(
            lock::pid_is_running(pid),
            Some(true),
            "Worker should be running before cleanup"
        );

        // Perform cleanup
        guard.cleanup()?;

        // Verify worker is terminated (allow for indeterminate result)
        let running = lock::pid_is_running(pid);
        assert!(
            running == Some(false) || running.is_none(),
            "Worker should be terminated after cleanup, got: {:?}",
            running
        );

        // Verify state is cleared
        assert!(
            guard.state_file.workers.is_empty(),
            "workers should be empty after cleanup"
        );

        Ok(())
    }

    #[test]
    fn guard_disarm_prevents_cleanup() -> Result<()> {
        let temp = TempDir::new()?;
        let mut guard = create_test_guard(&temp);

        let pid = register_sleeping_worker(&mut guard, &temp, "RQ-0001")?;

        // Disarm the guard
        guard.mark_completed();

        // Drop the guard (should not cleanup because it's disarmed)
        drop(guard);

        // Verify worker is still running
        assert_eq!(
            lock::pid_is_running(pid),
            Some(true),
            "Worker should still be running after disarmed drop"
        );

        // Clean up the child process
        kill_test_process(pid);

        Ok(())
    }

    #[test]
    fn guard_cleanup_is_idempotent() -> Result<()> {
        let temp = TempDir::new()?;
        let mut guard = create_test_guard(&temp);

        let pid = register_sleeping_worker(&mut guard, &temp, "RQ-0001")?;

        // First cleanup
        guard.cleanup()?;

        // Verify worker is terminated (allow for indeterminate result)
        let running = lock::pid_is_running(pid);
        assert!(
            running == Some(false) || running.is_none(),
            "Worker should be terminated after first cleanup, got: {:?}",
            running
        );

        // Second cleanup should be a no-op (idempotent)
        guard.cleanup()?;

        Ok(())
    }

    #[test]
    fn guard_cleanup_runs_on_drop() -> Result<()> {
        let temp = TempDir::new()?;

        let mut guard = create_test_guard(&temp);

        let pid = register_sleeping_worker(&mut guard, &temp, "RQ-0001")?;

        // Verify worker is running
        assert_eq!(
            lock::pid_is_running(pid),
            Some(true),
            "Worker should be running before drop"
        );

        // Explicitly drop the guard to trigger cleanup
        // This ensures temp dir is still valid during cleanup
        drop(guard);

        // Verify worker is terminated after guard is dropped
        // Allow for indeterminate result (None) as the process may have
        // been reaped by the time we check
        let running = lock::pid_is_running(pid);
        assert!(
            running == Some(false) || running.is_none(),
            "Worker should be terminated after guard drop, got: {:?}",
            running
        );

        Ok(())
    }

    #[test]
    fn guard_cleanup_retains_terminal_workers_for_status_retry() -> Result<()> {
        let temp = TempDir::new()?;
        let mut guard = create_test_guard(&temp);

        let running_workspace = temp.path().join("workspaces").join("RQ-0001");
        let completed_workspace = temp.path().join("workspaces").join("RQ-0002");
        std::fs::create_dir_all(&running_workspace)?;
        std::fs::create_dir_all(&completed_workspace)?;

        guard
            .state_file_mut()
            .upsert_worker(state::WorkerRecord::new(
                "RQ-0001",
                running_workspace,
                "2026-02-20T00:00:00Z".to_string(),
            ));

        let mut completed = state::WorkerRecord::new(
            "RQ-0002",
            completed_workspace,
            "2026-02-20T00:00:00Z".to_string(),
        );
        completed.mark_completed("2026-02-20T00:01:00Z".to_string());
        guard.state_file_mut().upsert_worker(completed);

        guard.cleanup()?;

        assert_eq!(guard.state_file.workers.len(), 1);
        assert_eq!(guard.state_file.workers[0].task_id, "RQ-0002");
        assert!(guard.state_file.workers[0].is_terminal());
        Ok(())
    }

    #[test]
    fn guard_cleanup_retains_blocked_workspace_for_retry() -> Result<()> {
        let temp = TempDir::new()?;
        let mut guard = create_test_guard(&temp);

        let blocked_workspace = temp.path().join("workspaces").join("RQ-0099");
        std::fs::create_dir_all(&blocked_workspace)?;

        guard.register_workspace(
            "RQ-0099".to_string(),
            WorkspaceSpec {
                path: blocked_workspace.clone(),
                branch: "main".to_string(),
            },
        );

        let mut blocked = state::WorkerRecord::new(
            "RQ-0099",
            blocked_workspace.clone(),
            "2026-02-20T00:00:00Z".to_string(),
        );
        blocked.mark_blocked("2026-02-20T00:05:00Z".to_string(), "blocked");
        guard.state_file_mut().upsert_worker(blocked);

        guard.cleanup()?;

        assert!(blocked_workspace.exists());
        assert_eq!(guard.state_file.workers.len(), 1);
        assert!(matches!(
            guard.state_file.workers[0].lifecycle,
            state::WorkerLifecycle::BlockedPush
        ));
        Ok(())
    }
}
