//! Cleanup helpers for runner process execution.
//!
//! Purpose:
//! - Cleanup helpers for runner process execution.
//!
//! Responsibilities:
//! - Clear active process-group tracking on exit.
//! - Join stdout/stderr reader threads so stream buffers are complete before use.
//!
//! Non-scope:
//! - Process waiting or signal escalation.
//! - Runner output parsing.
//!
//!
//! Usage:
//! - Used through the crate module tree or integration test harness.
//!
//! Invariants:
//! - Cleanup is idempotent.
//! - Drop must never panic.

use std::thread;

#[cfg(unix)]
use libc::{ESRCH, SIGKILL};

use super::CtrlCState;

type ReaderResult = anyhow::Result<()>;

pub(super) struct ProcessCleanupGuard<'a> {
    ctrlc: &'a CtrlCState,
    stdout_handle: Option<thread::JoinHandle<ReaderResult>>,
    stderr_handle: Option<thread::JoinHandle<ReaderResult>>,
    completed: bool,
}

impl<'a> ProcessCleanupGuard<'a> {
    pub(super) fn new(
        ctrlc: &'a CtrlCState,
        stdout_handle: thread::JoinHandle<ReaderResult>,
        stderr_handle: thread::JoinHandle<ReaderResult>,
    ) -> Self {
        Self {
            ctrlc,
            stdout_handle: Some(stdout_handle),
            stderr_handle: Some(stderr_handle),
            completed: false,
        }
    }

    pub(super) fn cleanup(&mut self) {
        if self.completed {
            return;
        }

        #[cfg(unix)]
        {
            let pgid = self
                .ctrlc
                .active_pgid
                .lock()
                .map(|mut guard| {
                    let pgid = *guard;
                    *guard = None;
                    pgid
                })
                .inspect_err(|e| log::debug!("cleanup: failed to lock active_pgid: {}", e))
                .ok()
                .flatten();
            terminate_lingering_process_group(pgid);
        }

        if let Some(handle) = self.stdout_handle.take()
            && let Err(e) = handle.join()
        {
            log::debug!("Stdout reader thread panicked: {:?}", e);
        }

        if let Some(handle) = self.stderr_handle.take()
            && let Err(e) = handle.join()
        {
            log::debug!("Stderr reader thread panicked: {:?}", e);
        }

        self.completed = true;
    }
}

impl Drop for ProcessCleanupGuard<'_> {
    fn drop(&mut self) {
        self.cleanup();
    }
}

#[cfg(unix)]
fn terminate_lingering_process_group(pgid: Option<i32>) {
    let Some(pgid) = pgid.filter(|value| *value > 0) else {
        return;
    };

    // The direct runner process has already exited by the time normal cleanup runs,
    // but orphaned grandchildren can keep inherited stdout/stderr pipes open. Kill
    // any remaining members of the isolated process group before joining reader
    // threads so the CLI can exit cleanly after the runner returns.
    // SAFETY: `pgid` was captured from the child process group created for this
    // runner invocation. Sending a signal to `-pgid` targets that group only.
    let result = unsafe { libc::kill(-pgid, SIGKILL) };
    if result == -1 {
        let err = std::io::Error::last_os_error();
        if err.raw_os_error() != Some(ESRCH) {
            log::debug!("cleanup: failed to kill runner process group {pgid}: {err}");
        }
    }
}
