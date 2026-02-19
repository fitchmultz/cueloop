//! Watch command implementation for file monitoring and task detection.
//!
//! Responsibilities:
//! - Set up file system watcher using the notify crate.
//! - Orchestrate debouncing, comment detection, and task creation.
//! - Provide the public API (`run_watch`) used by the CLI.
//!
//! Not handled here:
//! - CLI argument parsing (see `crate::cli::watch`).
//! - Low-level comment detection (see `comments.rs`).
//! - Task creation details (see `tasks.rs`).
//!
//! Invariants/assumptions:
//! - The module structure uses tight visibility; only `run_watch`, `CommentType`,
//!   and `WatchOptions` are public.
//! - Each submodule has a single cohesive responsibility.

use crate::commands::watch::comments::build_comment_regex;
use crate::commands::watch::event_loop::run_watch_loop;
use crate::commands::watch::processor::process_pending_files;
use crate::commands::watch::state::WatchState;
use crate::config::Resolved;
use anyhow::{Context, Result};
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

mod comments;
mod debounce;
mod event_loop;
mod paths;
mod processor;
mod state;
mod tasks;
mod types;

// Re-export public API for CLI
pub use types::{CommentType, WatchOptions};

/// Run the watch command with the given options.
pub fn run_watch(resolved: &Resolved, opts: WatchOptions) -> Result<()> {
    log::info!("Starting watch mode on {} path(s)...", opts.paths.len());
    log::info!("Patterns: {:?}", opts.patterns);
    log::info!("Debounce: {}ms", opts.debounce_ms);
    log::info!("Auto-queue: {}", opts.auto_queue);

    // Set up channel for file events
    let (tx, rx) = channel::<notify::Result<Event>>();

    // Create watcher
    let mut watcher = RecommendedWatcher::new(
        move |res: notify::Result<Event>| {
            let _ = tx.send(res);
        },
        Config::default(),
    )
    .context("Failed to create file watcher")?;

    // Watch specified paths
    for path in &opts.paths {
        let mode = if path.is_file() {
            RecursiveMode::NonRecursive
        } else {
            RecursiveMode::Recursive
        };
        watcher
            .watch(path, mode)
            .with_context(|| format!("Failed to watch path: {}", path.display()))?;
    }

    log::info!("Watch mode active. Press Ctrl+C to stop.");

    // Set up watch state with debouncing
    let state = Arc::new(Mutex::new(WatchState::new(opts.debounce_ms)));
    let state_for_signal = state.clone();

    // Set up Ctrl+C handler
    let running = Arc::new(Mutex::new(true));
    let running_for_signal = running.clone();

    ctrlc::set_handler(move || {
        log::info!("Received interrupt signal, shutting down...");
        match running_for_signal.lock() {
            Ok(mut r) => *r = false,
            Err(e) => {
                log::error!("Watch 'running' mutex poisoned in signal handler: {}", e);
                // Cannot recover; exit will happen via main loop detection
            }
        }
        // Trigger processing of any pending files
        match state_for_signal.lock() {
            Ok(mut s) => s.last_event = Instant::now() - Duration::from_secs(1),
            Err(e) => {
                log::error!("Watch 'state' mutex poisoned in signal handler: {}", e);
            }
        }
    })
    .context("Failed to set Ctrl+C handler")?;

    // Main event loop
    let comment_regex = build_comment_regex(&opts.comment_types)?;
    let mut last_processed: HashMap<PathBuf, Instant> = HashMap::new();

    run_watch_loop(
        &rx,
        &running,
        &state,
        resolved,
        &comment_regex,
        &opts,
        &mut last_processed,
    )?;

    // Process any remaining files before exit
    process_pending_files(resolved, &state, &comment_regex, &opts, &mut last_processed)?;

    log::info!("Watch mode stopped.");
    Ok(())
}

#[cfg(test)]
mod tests {
    //! Integration tests for watch command functionality.
    //!
    //! Responsibilities:
    //! - Test actual watch command behavior (run_watch, path validation)
    //! - Test regex building through public API
    //! - Test that all comment type variants work correctly
    //!
    //! Not handled here:
    //! - Individual module logic (tested in submodule unit tests)
    //! - Struct derive behavior (trusted to compiler)

    use super::*;
    use crate::commands::watch::types::CommentType;
    use crate::contracts::{Config, QueueFile};
    use tempfile::TempDir;

    fn create_test_resolved(temp_dir: &TempDir) -> Resolved {
        let queue_path = temp_dir.path().join("queue.json");
        let done_path = temp_dir.path().join("done.json");

        // Create empty queue file
        let queue = QueueFile::default();
        let queue_json = serde_json::to_string_pretty(&queue).unwrap();
        std::fs::write(&queue_path, queue_json).unwrap();

        Resolved {
            config: Config::default(),
            repo_root: temp_dir.path().to_path_buf(),
            queue_path,
            done_path,
            id_prefix: "RQ".to_string(),
            id_width: 4,
            global_config_path: None,
            project_config_path: None,
        }
    }

    #[test]
    fn build_comment_regex_integration() {
        // Test that build_comment_regex works through the public API
        let regex = build_comment_regex(&[CommentType::Todo]).unwrap();

        assert!(regex.is_match("// TODO: test"));
        assert!(regex.is_match("// todo: test"));
        assert!(!regex.is_match("// FIXME: test"));
    }

    #[test]
    fn watch_state_creation() {
        let state = WatchState::new(100);
        assert!(state.pending_files.is_empty());
        assert_eq!(state.debounce_duration, Duration::from_millis(100));
    }

    #[test]
    fn run_watch_rejects_nonexistent_paths() {
        let temp_dir = TempDir::new().unwrap();
        let resolved = create_test_resolved(&temp_dir);

        let nonexistent = temp_dir.path().join("nonexistent.rs");

        let opts = WatchOptions {
            patterns: vec!["*.rs".to_string()],
            debounce_ms: 50,
            auto_queue: false,
            notify: false,
            ignore_patterns: vec![],
            comment_types: vec![CommentType::Todo],
            paths: vec![nonexistent.clone()],
            force: false,
            close_removed: false,
        };

        // run_watch should fail for nonexistent paths
        let result = run_watch(&resolved, opts);
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(err_msg.contains("Failed to watch path") || err_msg.contains("No such file"));
    }

    #[test]
    fn all_comment_type_variants_compile_regex() {
        // Verify all comment type variants produce valid regexes
        let all_types = vec![
            CommentType::Todo,
            CommentType::Fixme,
            CommentType::Hack,
            CommentType::Xxx,
            CommentType::All,
        ];

        for ct in all_types {
            let regex = build_comment_regex(&[ct]).unwrap();
            // Should compile successfully with non-empty pattern
            assert!(!regex.as_str().is_empty());
        }
    }
}
