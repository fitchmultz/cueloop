//! Purpose: Centralize child-process wait state transitions for managed subprocesses and runners.
//!
//! Responsibilities:
//! - Own timeout and cancellation state transitions while a child process is alive.
//! - Prefer event-driven exit waiting on Unix and keep non-Unix polling localized.
//! - Provide one shared outcome model for callers that map exit/termination into domain errors.
//!
//! Scope:
//! - Child waiting, interrupt escalation, and deadline slicing only.
//! - Callers remain responsible for signal delivery and final error mapping.
//!
//! Usage:
//! - Used by `crate::runutil::shell` and `crate::runner::execution::process`.
//!
//! Invariants/Assumptions:
//! - Timeout checks take precedence over cooperative cancellation when both fire together.
//! - Unix callers run children in isolated process groups before signaling.
//! - Hard-kill escalation is sent at most once per child.

use std::process::{Child, ExitStatus};
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(unix)]
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;

/// Shared termination reasons surfaced by the wait state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ChildTerminationReason {
    Timeout,
    Cancelled,
}

/// Shared wait outcome returned to shell and runner callers.
#[derive(Debug)]
pub(crate) struct ChildWaitOutcome {
    pub status: ExitStatus,
    pub termination: Option<ChildTerminationReason>,
}

/// Runtime configuration for waiting on a single child process.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ChildWaitOptions<'a> {
    pub timeout: Option<Duration>,
    pub cancellation: Option<&'a AtomicBool>,
    pub poll_interval: Duration,
    pub interrupt_grace: Duration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChildWaitState {
    Running,
    Terminating {
        reason: ChildTerminationReason,
        started_at: Instant,
        hard_kill_sent: bool,
    },
}

impl ChildWaitState {
    fn termination_reason(self) -> Option<ChildTerminationReason> {
        match self {
            Self::Running => None,
            Self::Terminating { reason, .. } => Some(reason),
        }
    }

    fn interrupt_started_at(self) -> Option<Instant> {
        match self {
            Self::Running => None,
            Self::Terminating { started_at, .. } => Some(started_at),
        }
    }

    fn hard_kill_sent(self) -> bool {
        match self {
            Self::Running => false,
            Self::Terminating { hard_kill_sent, .. } => hard_kill_sent,
        }
    }
}

pub(crate) fn wait_for_child_with_callbacks<FSoft, FHard>(
    child: Child,
    options: ChildWaitOptions<'_>,
    soft_interrupt: FSoft,
    hard_kill: FHard,
) -> std::io::Result<ChildWaitOutcome>
where
    FSoft: FnMut(&mut Child),
    FHard: FnMut(&mut Child),
{
    #[cfg(unix)]
    {
        wait_for_child_unix(child, options, soft_interrupt, hard_kill)
    }
    #[cfg(not(unix))]
    {
        wait_for_child_polling(child, options, soft_interrupt, hard_kill)
    }
}

#[cfg(unix)]
fn wait_for_child_unix<FSoft, FHard>(
    mut child: Child,
    options: ChildWaitOptions<'_>,
    mut soft_interrupt: FSoft,
    mut hard_kill: FHard,
) -> std::io::Result<ChildWaitOutcome>
where
    FSoft: FnMut(&mut Child),
    FHard: FnMut(&mut Child),
{
    let start = Instant::now();
    let pid = child.id() as i32;
    let exit_rx = spawn_exit_waiter(pid);
    let mut state = ChildWaitState::Running;

    loop {
        let now = Instant::now();
        drive_wait_state(
            &mut child,
            options,
            start,
            now,
            &mut state,
            &mut soft_interrupt,
            &mut hard_kill,
        );

        match recv_exit_with_deadline(&exit_rx, next_wait_slice(start, options, state, now))? {
            Some(status) => {
                return Ok(ChildWaitOutcome {
                    status,
                    termination: state.termination_reason(),
                });
            }
            None => continue,
        }
    }
}

#[cfg(not(unix))]
fn wait_for_child_polling<FSoft, FHard>(
    mut child: Child,
    options: ChildWaitOptions<'_>,
    mut soft_interrupt: FSoft,
    mut hard_kill: FHard,
) -> std::io::Result<ChildWaitOutcome>
where
    FSoft: FnMut(&mut Child),
    FHard: FnMut(&mut Child),
{
    let start = Instant::now();
    let mut state = ChildWaitState::Running;

    loop {
        let now = Instant::now();
        drive_wait_state(
            &mut child,
            options,
            start,
            now,
            &mut state,
            &mut soft_interrupt,
            &mut hard_kill,
        );

        if let Some(status) = child.try_wait()? {
            return Ok(ChildWaitOutcome {
                status,
                termination: state.termination_reason(),
            });
        }

        std::thread::park_timeout(next_wait_slice(start, options, state, now));
    }
}

fn drive_wait_state<FSoft, FHard>(
    child: &mut Child,
    options: ChildWaitOptions<'_>,
    start: Instant,
    now: Instant,
    state: &mut ChildWaitState,
    soft_interrupt: &mut FSoft,
    hard_kill: &mut FHard,
) where
    FSoft: FnMut(&mut Child),
    FHard: FnMut(&mut Child),
{
    if *state == ChildWaitState::Running {
        if options
            .timeout
            .is_some_and(|limit| now.duration_since(start) > limit)
        {
            soft_interrupt(child);
            *state = ChildWaitState::Terminating {
                reason: ChildTerminationReason::Timeout,
                started_at: now,
                hard_kill_sent: false,
            };
        } else if options
            .cancellation
            .is_some_and(|flag| flag.load(Ordering::SeqCst))
        {
            soft_interrupt(child);
            *state = ChildWaitState::Terminating {
                reason: ChildTerminationReason::Cancelled,
                started_at: now,
                hard_kill_sent: false,
            };
        }
    }

    if let ChildWaitState::Terminating {
        reason,
        started_at,
        hard_kill_sent: false,
    } = *state
        && now.duration_since(started_at) > options.interrupt_grace
    {
        hard_kill(child);
        *state = ChildWaitState::Terminating {
            reason,
            started_at,
            hard_kill_sent: true,
        };
    }
}

fn next_wait_slice(
    start: Instant,
    options: ChildWaitOptions<'_>,
    state: ChildWaitState,
    now: Instant,
) -> Duration {
    let mut next = options.poll_interval.max(Duration::from_millis(1));

    if let Some(limit) = options.timeout
        && state == ChildWaitState::Running
    {
        let deadline = start + limit;
        next = next.min(
            deadline
                .saturating_duration_since(now)
                .max(Duration::from_millis(1)),
        );
    }

    if let Some(interrupted_at) = state.interrupt_started_at()
        && !state.hard_kill_sent()
    {
        let deadline = interrupted_at + options.interrupt_grace;
        next = next.min(
            deadline
                .saturating_duration_since(now)
                .max(Duration::from_millis(1)),
        );
    }

    next.max(Duration::from_millis(1))
}

#[cfg(unix)]
fn spawn_exit_waiter(pid: i32) -> Receiver<std::io::Result<ExitStatus>> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let mut raw_status = 0;
        // SAFETY: waiting on a known child pid is a standard POSIX operation.
        let result = match unsafe { libc::waitpid(pid, &mut raw_status, 0) } {
            waited if waited == pid => Ok(ExitStatus::from_raw(raw_status)),
            -1 => Err(std::io::Error::last_os_error()),
            waited => Err(std::io::Error::other(format!(
                "waitpid returned unexpected pid {} for {}",
                waited, pid
            ))),
        };
        let _ = tx.send(result);
    });
    rx
}

#[cfg(unix)]
fn recv_exit_with_deadline(
    rx: &Receiver<std::io::Result<ExitStatus>>,
    wait_for: Duration,
) -> std::io::Result<Option<ExitStatus>> {
    match rx.recv_timeout(wait_for) {
        Ok(result) => result.map(Some),
        Err(mpsc::RecvTimeoutError::Timeout) => Ok(None),
        Err(mpsc::RecvTimeoutError::Disconnected) => {
            Err(std::io::Error::other("child exit waiter disconnected"))
        }
    }
}
