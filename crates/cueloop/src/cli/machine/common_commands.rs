//! Canonical machine-command string helpers shared by machine surfaces.
//!
//! Purpose:
//! - Keep user-facing recovery/continuation command strings in one focused module.
//!
//! Responsibilities:
//! - Return stable machine command strings used in machine contracts, CLI output, and app guidance.
//! - Preserve exact placeholder text for commands that require operator-supplied values.
//!
//! Not handled here:
//! - Command execution.
//! - Clap argument parsing or routing.
//! - App-side classification of commands into native UI actions.
//!
//! Usage:
//! - Re-exported by `common.rs` for machine command handlers.
//!
//! Invariants/assumptions:
//! - Strings are part of the machine-facing recovery contract; update tests and app expectations together.

pub(crate) fn machine_queue_validate_command() -> &'static str {
    "cueloop machine queue validate"
}

pub(crate) fn machine_queue_graph_command() -> &'static str {
    "cueloop machine queue graph"
}

pub(crate) fn machine_queue_repair_command(dry_run: bool) -> &'static str {
    if dry_run {
        "cueloop machine queue repair --dry-run"
    } else {
        "cueloop machine queue repair"
    }
}

pub(crate) fn machine_queue_undo_dry_run_command() -> &'static str {
    "cueloop machine queue undo --dry-run"
}

pub(crate) fn machine_queue_undo_restore_command() -> &'static str {
    "cueloop machine queue undo --id <SNAPSHOT_ID>"
}

pub(crate) fn machine_task_mutate_command(dry_run: bool) -> &'static str {
    if dry_run {
        "cueloop machine task mutate --dry-run --input <PATH>"
    } else {
        "cueloop machine task mutate --input <PATH>"
    }
}

pub(crate) fn machine_task_build_command() -> &'static str {
    "cueloop machine task build --input <PATH>"
}

pub(crate) fn machine_task_decompose_write_preview_command(checkpoint_id: &str) -> String {
    format!(
        "cueloop machine task decompose --write --from-preview {}",
        shell_quote(checkpoint_id)
    )
}

fn shell_quote(value: &str) -> String {
    if value.bytes().all(|byte| {
        byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'/' | b':')
    }) {
        return value.to_string();
    }
    format!("'{}'", value.replace('\'', "'\\''"))
}

pub(crate) fn machine_run_one_resume_command() -> &'static str {
    "cueloop machine run one --resume"
}

pub(crate) fn machine_run_stop_command(dry_run: bool) -> &'static str {
    if dry_run {
        "cueloop machine run stop --dry-run"
    } else {
        "cueloop machine run stop"
    }
}

pub(crate) fn machine_run_parallel_status_command() -> &'static str {
    "cueloop machine run parallel-status"
}

pub(crate) fn machine_run_loop_command(parallel: bool, force: bool) -> &'static str {
    match (parallel, force) {
        (true, false) => "cueloop machine run loop --resume --max-tasks 0 --parallel <N>",
        (true, true) => "cueloop machine run loop --resume --max-tasks 0 --force --parallel <N>",
        (false, false) => "cueloop machine run loop --resume --max-tasks 0",
        (false, true) => "cueloop machine run loop --resume --max-tasks 0 --force",
    }
}

pub(crate) fn machine_doctor_report_command() -> &'static str {
    "cueloop machine doctor report"
}
