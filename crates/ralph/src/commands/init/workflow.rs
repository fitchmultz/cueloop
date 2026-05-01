//! Initialization workflow orchestration.
//!
//! Purpose:
//! - Implement the end-to-end `ralph init` workflow.
//!
//! Responsibilities:
//! - Create active runtime state, acquire the queue lock, and run the optional wizard.
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
    let runtime_dir = config::project_runtime_dir(&resolved.repo_root);
    fs::create_dir_all(&runtime_dir)
        .with_context(|| format!("create {}", runtime_dir.display()))?;

    let _queue_lock = queue::acquire_queue_lock(&resolved.repo_root, "init", opts.force_lock)?;

    let wizard_answers = if opts.interactive {
        Some(wizard::run_wizard(&resolved.repo_root)?)
    } else {
        None
    };

    let queue_path = runtime_dir.join("queue.jsonc");
    let done_path = runtime_dir.join("done.jsonc");
    let config_path = resolved
        .project_config_path
        .clone()
        .unwrap_or_else(|| config::project_config_path(&resolved.repo_root));

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
            "Failed to update .gitignore for local queue mode: {}. You may need to manually add '{}' and '{}' to your .gitignore.",
            e,
            queue_path
                .strip_prefix(&resolved.repo_root)
                .unwrap_or(&queue_path)
                .display(),
            done_path
                .strip_prefix(&resolved.repo_root)
                .unwrap_or(&done_path)
                .display()
        );
    }

    let mut readme_status = None;
    if crate::prompts::prompts_reference_readme(&resolved.repo_root)? {
        let readme_path = runtime_dir.join("README.md");
        let (status, version) = readme::write_readme(&readme_path, opts.force)?;
        readme_status = Some((status, version));
    }

    if let Err(e) = gitignore::ensure_cueloop_gitignore_entries(&resolved.repo_root) {
        log::warn!(
            "Failed to update .gitignore: {}. You may need to manually add active runtime logs, workspaces, and trust entries to your .gitignore.",
            e
        );
    }

    check_pending_migrations(resolved)?;

    if opts.interactive {
        wizard::print_completion_message(wizard_answers.as_ref(), &queue_path);
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
