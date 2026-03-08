//! Worker subprocess lifecycle helpers.
//!
//! Responsibilities:
//! - Spawn worker subprocesses in isolated workspaces.
//! - Track worker state and terminate workers gracefully.
//!
//! Does not handle:
//! - Task selection.
//! - Parallel orchestration loop state transitions.

use crate::agent::AgentOverrides;
use crate::config;
use crate::git::WorkspaceSpec;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::Path;
use std::process::Child;
use std::time::Duration;

use super::command::build_worker_command;

pub(crate) struct WorkerState {
    pub task_id: String,
    pub task_title: String,
    pub workspace: WorkspaceSpec,
    pub child: Child,
}

pub(crate) fn terminate_workers(in_flight: &mut HashMap<String, WorkerState>) {
    for worker in in_flight.values_mut() {
        terminate_worker_process(worker);
    }

    for worker in in_flight.values_mut() {
        if let Err(e) = worker.child.wait() {
            log::debug!("Failed to wait for worker {}: {}", worker.task_id, e);
        }
    }
}

fn terminate_worker_process(worker: &mut WorkerState) {
    #[cfg(unix)]
    {
        let pid = worker.child.id() as i32;
        let sigint_result = unsafe { libc::kill(pid, libc::SIGINT) };
        if sigint_result != 0 {
            let err = std::io::Error::last_os_error();
            if err.raw_os_error() != Some(libc::ESRCH) {
                log::debug!(
                    "Failed to send SIGINT to worker {} (pid {}): {}",
                    worker.task_id,
                    pid,
                    err
                );
            }
        }

        let grace = Duration::from_millis(1_500);
        let deadline = std::time::Instant::now() + grace;
        while std::time::Instant::now() < deadline {
            match worker.child.try_wait() {
                Ok(Some(_)) => return,
                Ok(None) => std::thread::sleep(Duration::from_millis(50)),
                Err(err) => {
                    log::debug!(
                        "Failed to poll worker {} during graceful shutdown: {}",
                        worker.task_id,
                        err
                    );
                    break;
                }
            }
        }

        if let Err(err) = worker.child.kill()
            && err.kind() != std::io::ErrorKind::InvalidInput
        {
            log::warn!("Failed to terminate worker {}: {}", worker.task_id, err);
        }
    }

    #[cfg(not(unix))]
    {
        if let Err(err) = worker.child.kill() {
            log::warn!("Failed to terminate worker {}: {}", worker.task_id, err);
        }
    }
}

pub(crate) fn spawn_worker(
    resolved: &config::Resolved,
    workspace_path: &Path,
    task_id: &str,
    target_branch: &str,
    overrides: &AgentOverrides,
    force: bool,
) -> Result<Child> {
    let (mut cmd, args) = build_worker_command(
        resolved,
        workspace_path,
        task_id,
        target_branch,
        overrides,
        force,
    )?;
    log::debug!(
        "Spawning parallel worker {} in {} with args: {:?}",
        task_id,
        workspace_path.display(),
        args
    );
    cmd.args(args);
    let child = cmd.spawn().context("spawn parallel worker")?;
    Ok(child)
}
