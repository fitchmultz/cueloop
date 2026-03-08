//! Event loop for the watch command.
//!
//! Responsibilities:
//! - Run the watch event loop until stopped or channel disconnects.
//! - Handle file events and coordinate processing.
//! - Manage Ctrl+C signal handling state.
//!
//! Not handled here:
//! - File watching setup (see `mod.rs`).
//! - Comment detection (see `comments.rs`).
//! - Task creation (see `tasks.rs`).
//!
//! Invariants/assumptions:
//! - The loop exits cleanly on channel disconnect or when `running` is false.
//! - Mutex poison errors are handled gracefully (logged, loop continues or exits).
//! - Timeout-based processing checks for pending files on each iteration.

use crate::commands::watch::debounce::can_reprocess_at;
use crate::commands::watch::paths::get_relevant_paths;
use crate::commands::watch::processor::process_pending_files;
use crate::commands::watch::state::WatchState;
use crate::commands::watch::types::WatchOptions;
use crate::config::Resolved;
use anyhow::Result;
use notify::Event;
use regex::Regex;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc::RecvTimeoutError;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

fn handle_watch_event(
    event: &Event,
    state: &Arc<Mutex<WatchState>>,
    resolved: &Resolved,
    comment_regex: &Regex,
    opts: &WatchOptions,
    last_processed: &mut HashMap<PathBuf, Instant>,
    now: Instant,
) -> Result<()> {
    if let Some(paths) = get_relevant_paths(event, opts) {
        let debounce = Duration::from_millis(opts.debounce_ms);
        let mut should_process = false;
        match state.lock() {
            Ok(mut guard) => {
                for path in paths {
                    if can_reprocess_at(&path, last_processed, debounce, now)
                        && guard.add_file_at(path.clone(), now)
                    {
                        should_process = true;
                    }
                }
            }
            Err(e) => {
                log::error!("Watch 'state' mutex poisoned, skipping event: {}", e);
                return Ok(());
            }
        }
        if should_process {
            process_pending_files(resolved, state, comment_regex, opts, last_processed)?;
        }
    }
    Ok(())
}

fn pending_files_ready_at(state: &WatchState, now: Instant) -> bool {
    !state.pending_files.is_empty()
        && now.duration_since(state.last_event) >= state.debounce_duration
}

fn handle_timeout_tick(
    state: &Arc<Mutex<WatchState>>,
    resolved: &Resolved,
    comment_regex: &Regex,
    opts: &WatchOptions,
    last_processed: &mut HashMap<PathBuf, Instant>,
    now: Instant,
) -> Result<()> {
    let should_process = match state.lock() {
        Ok(guard) => pending_files_ready_at(&guard, now),
        Err(e) => {
            log::error!("Watch 'state' mutex poisoned during timeout check: {}", e);
            false
        }
    };
    if should_process {
        process_pending_files(resolved, state, comment_regex, opts, last_processed)?;
    }
    Ok(())
}

/// Run the watch event loop until stopped or channel disconnects.
///
/// This is extracted as a separate function for testability.
pub fn run_watch_loop(
    rx: &std::sync::mpsc::Receiver<notify::Result<Event>>,
    running: &Arc<Mutex<bool>>,
    state: &Arc<Mutex<WatchState>>,
    resolved: &Resolved,
    comment_regex: &Regex,
    opts: &WatchOptions,
    last_processed: &mut HashMap<PathBuf, Instant>,
) -> Result<()> {
    loop {
        // Check running state with poison handling
        let should_continue = match running.lock() {
            Ok(guard) => *guard,
            Err(e) => {
                log::error!("Watch 'running' mutex poisoned, exiting: {}", e);
                break;
            }
        };
        if !should_continue {
            break;
        }
        // Check for events with timeout
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(Ok(event)) => {
                handle_watch_event(
                    &event,
                    state,
                    resolved,
                    comment_regex,
                    opts,
                    last_processed,
                    Instant::now(),
                )?;
            }
            Ok(Err(e)) => {
                log::warn!("Watch error: {}", e);
            }
            Err(RecvTimeoutError::Disconnected) => {
                log::info!("Watch channel disconnected, shutting down...");
                break;
            }
            Err(RecvTimeoutError::Timeout) => {
                handle_timeout_tick(
                    state,
                    resolved,
                    comment_regex,
                    opts,
                    last_processed,
                    Instant::now(),
                )?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::watch::comments::build_comment_regex;
    use crate::commands::watch::types::CommentType;
    use crate::contracts::{Config, QueueFile};
    use std::path::PathBuf;
    use std::sync::mpsc::channel;
    use tempfile::{NamedTempFile, TempDir};

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
    fn watch_loop_exits_on_channel_disconnect() {
        let temp_dir = TempDir::new().unwrap();
        let resolved = create_test_resolved(&temp_dir);

        let (tx, rx) = channel::<notify::Result<Event>>();
        let running = Arc::new(Mutex::new(true));
        let state = Arc::new(Mutex::new(WatchState::new(100)));

        let opts = WatchOptions {
            patterns: vec!["*.rs".to_string()],
            debounce_ms: 100,
            auto_queue: false,
            notify: false,
            ignore_patterns: vec![],
            comment_types: vec![CommentType::Todo],
            paths: vec![PathBuf::from(".")],
            force: false,
            close_removed: false,
        };

        let comment_regex = build_comment_regex(&opts.comment_types).unwrap();
        let mut last_processed: HashMap<PathBuf, Instant> = HashMap::new();

        // Drop the sender to simulate channel disconnect
        drop(tx);

        run_watch_loop(
            &rx,
            &running,
            &state,
            &resolved,
            &comment_regex,
            &opts,
            &mut last_processed,
        )
        .expect("watch loop should exit cleanly on channel disconnect");
    }

    #[test]
    fn watch_loop_exits_on_running_mutex_poison() {
        let temp_dir = TempDir::new().unwrap();
        let resolved = create_test_resolved(&temp_dir);

        let (tx, rx) = channel::<notify::Result<Event>>();
        let running = Arc::new(Mutex::new(true));
        let state = Arc::new(Mutex::new(WatchState::new(100)));

        let opts = WatchOptions {
            patterns: vec!["*.rs".to_string()],
            debounce_ms: 100,
            auto_queue: false,
            notify: false,
            ignore_patterns: vec![],
            comment_types: vec![CommentType::Todo],
            paths: vec![PathBuf::from(".")],
            force: false,
            close_removed: false,
        };

        let comment_regex = build_comment_regex(&opts.comment_types).unwrap();
        let mut last_processed: HashMap<PathBuf, Instant> = HashMap::new();

        // Clone for the poisoning thread
        let running_clone = running.clone();

        // Spawn a thread that will panic while holding the running mutex
        let poison_handle = std::thread::spawn(move || {
            let _guard = running_clone.lock().unwrap();
            panic!("Intentional panic to poison running mutex");
        });

        // Wait for the panic
        let _ = poison_handle.join();

        drop(tx);
        run_watch_loop(
            &rx,
            &running,
            &state,
            &resolved,
            &comment_regex,
            &opts,
            &mut last_processed,
        )
        .expect("watch loop should exit cleanly on running mutex poison");
    }

    // =====================================================================
    // Additional event loop tests
    // =====================================================================

    #[test]
    fn watch_loop_processes_file_event() {
        use notify::EventKind;
        use notify::event::{DataChange, ModifyKind};
        use std::io::Write;

        let temp_dir = TempDir::new().unwrap();
        let resolved = create_test_resolved(&temp_dir);

        // Create a file with a TODO
        let mut temp_file = NamedTempFile::new_in(temp_dir.path()).unwrap();
        writeln!(temp_file, "// TODO: test").unwrap();
        temp_file.flush().unwrap();

        let (tx, rx) = channel::<notify::Result<Event>>();
        let state = Arc::new(Mutex::new(WatchState::new(50)));

        let opts = WatchOptions {
            patterns: vec!["*.rs".to_string()],
            debounce_ms: 50,
            auto_queue: false,
            notify: false,
            ignore_patterns: vec![],
            comment_types: vec![CommentType::Todo],
            paths: vec![PathBuf::from(".")],
            force: false,
            close_removed: false,
        };

        let comment_regex = build_comment_regex(&opts.comment_types).unwrap();
        let mut last_processed: HashMap<PathBuf, Instant> = HashMap::new();
        let start = Instant::now();

        // Send a file event
        let event = Event {
            kind: EventKind::Modify(ModifyKind::Data(DataChange::Content)),
            paths: vec![temp_file.path().to_path_buf()],
            attrs: Default::default(),
        };
        tx.send(Ok(event.clone())).unwrap();
        let received = rx
            .recv_timeout(Duration::from_secs(1))
            .expect("receive watch event")
            .expect("watch event result");
        handle_watch_event(
            &received,
            &state,
            &resolved,
            &comment_regex,
            &opts,
            &mut last_processed,
            start,
        )
        .expect("handle watch event");
        handle_timeout_tick(
            &state,
            &resolved,
            &comment_regex,
            &opts,
            &mut last_processed,
            start + Duration::from_millis(60),
        )
        .expect("process timeout tick");
        drop(tx);
    }

    #[test]
    fn watch_loop_handles_state_mutex_poison_during_event() {
        use notify::EventKind;
        use notify::event::{DataChange, ModifyKind};

        let temp_dir = TempDir::new().unwrap();
        let resolved = create_test_resolved(&temp_dir);

        let (tx, rx) = channel::<notify::Result<Event>>();
        let state = Arc::new(Mutex::new(WatchState::new(100)));

        let opts = WatchOptions {
            patterns: vec!["*.rs".to_string()],
            debounce_ms: 100,
            auto_queue: false,
            notify: false,
            ignore_patterns: vec![],
            comment_types: vec![CommentType::Todo],
            paths: vec![PathBuf::from(".")],
            force: false,
            close_removed: false,
        };

        let comment_regex = build_comment_regex(&opts.comment_types).unwrap();
        let mut last_processed: HashMap<PathBuf, Instant> = HashMap::new();

        // Poison the state mutex
        let state_clone = state.clone();
        let poison_handle = std::thread::spawn(move || {
            let _guard = state_clone.lock().unwrap();
            panic!("Poison state mutex");
        });
        let _ = poison_handle.join();

        // Send an event - loop should handle poison gracefully
        let event = Event {
            kind: EventKind::Modify(ModifyKind::Data(DataChange::Content)),
            paths: vec![PathBuf::from("/test/file.rs")],
            attrs: Default::default(),
        };
        tx.send(Ok(event)).unwrap();
        let received = rx
            .recv_timeout(Duration::from_secs(1))
            .expect("receive watch event")
            .expect("watch event result");
        handle_watch_event(
            &received,
            &state,
            &resolved,
            &comment_regex,
            &opts,
            &mut last_processed,
            Instant::now(),
        )
        .expect("state mutex poison should be handled gracefully");

        drop(tx);
    }

    #[test]
    fn watch_loop_handles_watch_error() {
        let (tx, rx) = channel::<notify::Result<Event>>();

        // Send an error result
        let error = notify::Error::generic("Test watch error");
        tx.send(Err(error)).unwrap();
        let received = rx.recv_timeout(Duration::from_secs(1));
        assert!(matches!(received, Ok(Err(_))));
        drop(tx);
    }

    #[test]
    fn watch_loop_exits_when_running_set_to_false() {
        let temp_dir = TempDir::new().unwrap();
        let resolved = create_test_resolved(&temp_dir);

        let (_tx, rx) = channel::<notify::Result<Event>>();
        let running = Arc::new(Mutex::new(true));
        let state = Arc::new(Mutex::new(WatchState::new(100)));

        let opts = WatchOptions {
            patterns: vec!["*.rs".to_string()],
            debounce_ms: 100,
            auto_queue: false,
            notify: false,
            ignore_patterns: vec![],
            comment_types: vec![CommentType::Todo],
            paths: vec![PathBuf::from(".")],
            force: false,
            close_removed: false,
        };

        let comment_regex = build_comment_regex(&opts.comment_types).unwrap();
        let mut last_processed: HashMap<PathBuf, Instant> = HashMap::new();

        *running.lock().unwrap() = false;
        run_watch_loop(
            &rx,
            &running,
            &state,
            &resolved,
            &comment_regex,
            &opts,
            &mut last_processed,
        )
        .expect("watch loop should exit immediately when running=false");
    }

    #[test]
    fn timeout_tick_processes_pending_files_after_debounce() {
        use std::io::Write;

        let temp_dir = TempDir::new().unwrap();
        let resolved = create_test_resolved(&temp_dir);
        let mut temp_file = NamedTempFile::new_in(temp_dir.path()).unwrap();
        writeln!(temp_file, "// TODO: timeout path").unwrap();
        temp_file.flush().unwrap();

        let state = Arc::new(Mutex::new(WatchState::new(50)));
        let opts = WatchOptions {
            patterns: vec!["*.rs".to_string()],
            debounce_ms: 50,
            auto_queue: false,
            notify: false,
            ignore_patterns: vec![],
            comment_types: vec![CommentType::Todo],
            paths: vec![PathBuf::from(".")],
            force: false,
            close_removed: false,
        };
        let comment_regex = build_comment_regex(&opts.comment_types).unwrap();
        let mut last_processed: HashMap<PathBuf, Instant> = HashMap::new();
        let start = Instant::now();

        state
            .lock()
            .unwrap()
            .add_file_at(temp_file.path().to_path_buf(), start);

        handle_timeout_tick(
            &state,
            &resolved,
            &comment_regex,
            &opts,
            &mut last_processed,
            start + Duration::from_millis(60),
        )
        .expect("timeout tick should process pending files");

        let guard = state.lock().unwrap();
        assert!(guard.pending_files.is_empty());
    }

    #[test]
    fn event_loop_helpers_leave_timeout_queue_idle_without_pending_files() {
        let temp_dir = TempDir::new().unwrap();
        let resolved = create_test_resolved(&temp_dir);
        let state = Arc::new(Mutex::new(WatchState::new(50)));
        let opts = WatchOptions {
            patterns: vec!["*.rs".to_string()],
            debounce_ms: 50,
            auto_queue: false,
            notify: false,
            ignore_patterns: vec![],
            comment_types: vec![CommentType::Todo],
            paths: vec![PathBuf::from(".")],
            force: false,
            close_removed: false,
        };
        let comment_regex = build_comment_regex(&opts.comment_types).unwrap();
        let mut last_processed: HashMap<PathBuf, Instant> = HashMap::new();

        handle_timeout_tick(
            &state,
            &resolved,
            &comment_regex,
            &opts,
            &mut last_processed,
            Instant::now() + Duration::from_secs(1),
        )
        .expect("timeout tick without pending files should be a no-op");
        assert!(state.lock().unwrap().pending_files.is_empty());
    }
}
