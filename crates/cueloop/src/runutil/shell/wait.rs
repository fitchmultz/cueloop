//! Managed subprocess wait helpers.
//!
//! Purpose:
//! - Adapt the shared child wait state machine to managed operational subprocesses.
//!
//! Responsibilities:
//! - Map shared timeout/cancellation outcomes into managed subprocess semantics.
//! - Apply SIGINT-before-SIGKILL escalation with platform-appropriate signaling.
//!
//! Scope:
//! - Managed subprocess signaling and outcome mapping only.
//!
//! Usage:
//! - Invoked exclusively by `crate::runutil::shell` during managed subprocess execution.
//!
//! Invariants/assumptions:
//! - Unix children run in isolated process groups before these helpers signal them.
//! - Successful exits after soft interruption are treated as successful command completion.

use std::process::{Child, ExitStatus};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

use crate::constants::timeouts;
use crate::runutil::{ChildTerminationReason, ChildWaitOptions, wait_for_child_with_callbacks};

use super::types::TerminationReason;

pub(super) struct ManagedChildOutcome {
    pub status: ExitStatus,
    pub termination: Option<TerminationReason>,
}

pub(super) fn wait_for_child_owned(
    child: Child,
    timeout: Duration,
    cancellation: Option<Arc<AtomicBool>>,
) -> std::io::Result<ManagedChildOutcome> {
    let pid = child.id();
    let outcome = wait_for_child_with_callbacks(
        child,
        ChildWaitOptions {
            timeout: Some(timeout),
            cancellation: cancellation.as_deref(),
            poll_interval: timeouts::MANAGED_SUBPROCESS_POLL_INTERVAL,
            interrupt_grace: timeouts::MANAGED_SUBPROCESS_INTERRUPT_GRACE,
        },
        move |child| signal_child(child, pid, false),
        move |child| signal_child(child, pid, true),
    )?;

    Ok(ManagedChildOutcome {
        status: outcome.status,
        termination: if outcome.status.success() {
            None
        } else {
            outcome.termination.map(map_termination_reason)
        },
    })
}

fn map_termination_reason(reason: ChildTerminationReason) -> TerminationReason {
    match reason {
        ChildTerminationReason::Timeout => TerminationReason::Timeout,
        ChildTerminationReason::Cancelled => TerminationReason::Cancelled,
    }
}

#[cfg(unix)]
fn signal_child(_child: &mut Child, pid: u32, hard_kill: bool) {
    let signal = if hard_kill {
        libc::SIGKILL
    } else {
        libc::SIGINT
    };
    // SAFETY: signal is sent to the isolated child process group started by execute_managed_command.
    unsafe {
        let _ = libc::kill(-(pid as i32), signal);
    }
}

#[cfg(not(unix))]
fn signal_child(child: &mut Child, _pid: u32, _hard_kill: bool) {
    let _ = child.kill();
}
