//! Initialization workflow orchestration.
//!
//! Purpose:
//! - Implement the end-to-end `ralph init` workflow.
//!
//! Responsibilities:
//! - Create `.ralph` state, acquire the queue lock, and run the optional wizard.
//! - Write queue, done, config, and optional README files.
//! - Update `.gitignore`, warn on pending migrations, and build the final report.
//!
//! Scope:
//! - Workflow orchestration only; individual writers and README helpers live in sibling modules.
//!
//! Usage:
//! - Re-exported as `crate::commands::init::run_init`.
//!
//! Invariants/assumptions:
//! - New projects always initialize `.jsonc` queue/done/config files.
//! - Gitignore updates remain best-effort and idempotent.

use crate::config;
use crate::queue;
use anyhow::{Context, Result};
use std::fs;

use super::migration_check::check_pending_migrations;
use super::{InitOptions, InitReport, gitignore, readme, wizard, writers};

pub fn run_init(resolved: &config::Resolved, opts: InitOptions) -> Result<InitReport> {
    let ralph_dir = resolved.repo_root.join(".ralph");
    fs::create_dir_all(&ralph_dir).with_context(|| format!("create {}", ralph_dir.display()))?;

    let _queue_lock = queue::acquire_queue_lock(&resolved.repo_root, "init", opts.force_lock)?;

    let wizard_answers = if opts.interactive {
        Some(wizard::run_wizard(&resolved.repo_root)?)
    } else {
        None
    };

    let queue_path = resolved
        .repo_root
        .join(crate::constants::queue::DEFAULT_QUEUE_FILE);
    let done_path = resolved
        .repo_root
        .join(crate::constants::queue::DEFAULT_DONE_FILE);
    let config_path = resolved
        .repo_root
        .join(crate::constants::queue::DEFAULT_CONFIG_FILE);

    let queue_status = writers::write_queue(
        &queue_path,
        opts.force,
        &resolved.id_prefix,
        resolved.id_width,
        wizard_answers.as_ref(),
    )?;
    let done_status = writers::write_done(
        &done_path,
        opts.force,
        &resolved.id_prefix,
        resolved.id_width,
    )?;
    let config_status = writers::write_config(&config_path, opts.force, wizard_answers.as_ref())?;

    if let Some(answers) = wizard_answers.as_ref()
        && answers.queue_tracking_mode == wizard::QueueTrackingMode::LocalIgnored
        && let Err(e) = gitignore::ensure_local_queue_gitignore_entries(&resolved.repo_root)
    {
        log::warn!(
            "Failed to update .gitignore for local queue mode: {}. You may need to manually add '.ralph/queue.jsonc' and '.ralph/done.jsonc' to your .gitignore.",
            e
        );
    }

    let mut readme_status = None;
    if crate::prompts::prompts_reference_readme(&resolved.repo_root)? {
        let readme_path = resolved.repo_root.join(".ralph/README.md");
        let (status, version) = readme::write_readme(&readme_path, opts.force, opts.update_readme)?;
        readme_status = Some((status, version));
    }

    if let Err(e) = gitignore::ensure_ralph_gitignore_entries(&resolved.repo_root) {
        log::warn!(
            "Failed to update .gitignore: {}. You may need to manually add '.ralph/workspaces/', '.ralph/logs/', and '.ralph/trust.jsonc' to your .gitignore.",
            e
        );
    }

    check_pending_migrations(resolved)?;

    if opts.interactive {
        wizard::print_completion_message(wizard_answers.as_ref(), &resolved.queue_path);
    }

    Ok(InitReport {
        queue_status,
        done_status,
        config_status,
        readme_status,
        queue_path,
        done_path,
        config_path,
    })
}
