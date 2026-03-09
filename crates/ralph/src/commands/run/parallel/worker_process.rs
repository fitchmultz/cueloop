//! Worker subprocess lifecycle helpers.
//!
//! Responsibilities:
//! - Spawn worker subprocesses in isolated workspaces.
//! - Emit explicit worker-exit events instead of requiring orchestrators to poll children.
//! - Terminate workers gracefully and wait for monitor confirmation during cleanup.
//!
//! Does not handle:
//! - Task selection.
//! - Parallel orchestration loop state transitions.
//!
//! Assumptions/invariants:
//! - Worker commands create isolated process groups on unix.
//! - Each worker has exactly one monitor thread that owns `Child::wait()`.

use crate::agent::AgentOverrides;
use crate::config;
use crate::git::WorkspaceSpec;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::Path;
use std::process::{Child, ExitStatus};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Duration;

use super::command::build_worker_command;

const WORKER_INTERRUPT_GRACE: Duration = Duration::from_millis(1_500);
const WORKER_EXIT_WAIT_SLICE: Duration = Duration::from_millis(100);

#[derive(Debug)]
pub(crate) struct WorkerExitEvent {
    pub task_id: String,
    pub status: ExitStatus,
}

pub(crate) struct WorkerState {
    pub task_id: String,
    pub task_title: String,
    pub workspace: WorkspaceSpec,
    pub pid: u32,
    exit_rx: Receiver<std::io::Result<ExitStatus>>,
}

impl WorkerState {
    fn recv_exit_timeout(&self, timeout: Duration) -> Option<std::io::Result<ExitStatus>> {
        match self.exit_rx.recv_timeout(timeout) {
            Ok(status) => Some(status),
            Err(mpsc::RecvTimeoutError::Timeout) => None,
            Err(mpsc::RecvTimeoutError::Disconnected) => Some(Err(std::io::Error::other(
                "worker exit monitor disconnected",
            ))),
        }
    }
}

pub(crate) fn terminate_workers(in_flight: &mut HashMap<String, WorkerState>) {
    for worker in in_flight.values_mut() {
        terminate_worker_process(worker);
    }
}

fn terminate_worker_process(worker: &mut WorkerState) {
    #[cfg(unix)]
    {
        let pid = worker.pid as i32;
        send_signal(pid, libc::SIGINT, &worker.task_id, "SIGINT");

        if worker.recv_exit_timeout(WORKER_INTERRUPT_GRACE).is_some() {
            return;
        }

        send_signal(pid, libc::SIGKILL, &worker.task_id, "SIGKILL");
        let _ = worker.recv_exit_timeout(WORKER_EXIT_WAIT_SLICE);
    }

    #[cfg(not(unix))]
    {
        let _ = worker.recv_exit_timeout(WORKER_INTERRUPT_GRACE);
    }
}

#[cfg(unix)]
fn send_signal(pid: i32, signal: i32, task_id: &str, label: &str) {
    let group_result = unsafe { libc::kill(-pid, signal) };
    if group_result == 0 {
        return;
    }

    let group_err = std::io::Error::last_os_error();
    let direct_result = unsafe { libc::kill(pid, signal) };
    if direct_result == 0 {
        return;
    }

    let direct_err = std::io::Error::last_os_error();
    if direct_err.raw_os_error() != Some(libc::ESRCH) {
        log::warn!(
            "Failed to send {} to worker {} via pgid {} ({}) and pid {} ({}).",
            label,
            task_id,
            pid,
            group_err,
            pid,
            direct_err
        );
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
    cmd.spawn().context("spawn parallel worker")
}

pub(crate) fn start_worker_monitor(
    task_id: &str,
    task_title: String,
    workspace: WorkspaceSpec,
    mut child: Child,
    worker_events: Sender<WorkerExitEvent>,
) -> WorkerState {
    let pid = child.id();
    let task_id_owned = task_id.to_string();
    let (exit_tx, exit_rx) = mpsc::channel();
    let event_task_id = task_id_owned.clone();

    thread::spawn(move || {
        let result = child.wait();
        match result {
            Ok(status) => {
                let _ = worker_events.send(WorkerExitEvent {
                    task_id: event_task_id.clone(),
                    status,
                });
                let _ = exit_tx.send(Ok(status));
            }
            Err(err) => {
                let _ = exit_tx.send(Err(err));
            }
        }
    });

    WorkerState {
        task_id: task_id_owned,
        task_title,
        workspace,
        pid,
        exit_rx,
    }
}
