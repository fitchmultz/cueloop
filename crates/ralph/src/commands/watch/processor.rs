//! File processing for the watch command.
//!
//! Responsibilities:
//! - Process pending files and detect comments.
//! - Coordinate between debounce logic, comment detection, and task handling.
//!
//! Not handled here:
//! - File watching or event handling (see `event_loop.rs`).
//! - Low-level comment detection (see `comments.rs`).
//! - Task creation (see `tasks.rs`).
//!
//! Invariants/assumptions:
//! - Files are skipped if recently processed (within debounce window).
//! - Errors reading individual files are logged but don't stop processing.
//! - Old entries in `last_processed` are cleaned up periodically.

use crate::commands::watch::comments::detect_comments;
use crate::commands::watch::debounce::{can_reprocess, cleanup_old_entries};
use crate::commands::watch::state::WatchState;
use crate::commands::watch::tasks::handle_detected_comments;
use crate::commands::watch::types::{DetectedComment, WatchOptions};
use crate::config::Resolved;
use anyhow::Result;
use regex::Regex;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Process pending files and detect comments.
pub fn process_pending_files(
    resolved: &Resolved,
    state: &Arc<Mutex<WatchState>>,
    comment_regex: &Regex,
    opts: &WatchOptions,
    last_processed: &mut HashMap<PathBuf, Instant>,
) -> Result<()> {
    let files: Vec<PathBuf> = match state.lock() {
        Ok(mut guard) => guard.take_pending(),
        Err(e) => {
            log::error!("Watch 'state' mutex poisoned, cannot process files: {}", e);
            return Ok(());
        }
    };

    if files.is_empty() {
        return Ok(());
    }

    let debounce = Duration::from_millis(opts.debounce_ms);
    let mut all_comments: Vec<DetectedComment> = Vec::new();

    for file_path in files {
        // Skip if file was recently processed (within debounce window)
        if !can_reprocess(&file_path, last_processed, debounce) {
            continue;
        }

        match detect_comments(&file_path, comment_regex) {
            Ok(comments) => {
                if !comments.is_empty() {
                    log::debug!(
                        "Detected {} comments in {}",
                        comments.len(),
                        file_path.display()
                    );
                    all_comments.extend(comments);
                }
                // Record when this file was processed
                last_processed.insert(file_path, Instant::now());
            }
            Err(e) => {
                log::warn!("Failed to process file {}: {}", file_path.display(), e);
            }
        }
    }

    // Periodically clean up old entries to prevent unbounded growth
    cleanup_old_entries(last_processed, debounce);

    if !all_comments.is_empty() {
        handle_detected_comments(resolved, &all_comments, opts)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::watch::comments::build_comment_regex;
    use crate::commands::watch::types::CommentType;
    use crate::contracts::{Config, QueueFile};
    use std::io::Write;
    use std::path::PathBuf;
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
    fn process_pending_files_handles_state_mutex_poison() {
        let temp_dir = TempDir::new().unwrap();
        let resolved = create_test_resolved(&temp_dir);
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
        let state_clone = state.clone();

        // Spawn a thread that will panic while holding the state mutex
        let poison_handle = std::thread::spawn(move || {
            let _guard = state_clone.lock().unwrap();
            panic!("Intentional panic to poison state mutex");
        });

        // Wait for the panic
        let _ = poison_handle.join();

        // Now the state mutex is poisoned - verify process_pending_files handles it gracefully
        let result = process_pending_files(
            &resolved,
            &state,
            &comment_regex,
            &opts,
            &mut last_processed,
        );

        // Should return Ok, not panic
        assert!(
            result.is_ok(),
            "process_pending_files should handle state mutex poison gracefully"
        );
    }

    #[test]
    fn process_pending_files_happy_path() {
        let temp_dir = TempDir::new().unwrap();
        let resolved = create_test_resolved(&temp_dir);
        let state = Arc::new(Mutex::new(WatchState::new(100)));

        // Create a temp file with a TODO comment
        let mut temp_file = NamedTempFile::new_in(temp_dir.path()).unwrap();
        writeln!(temp_file, "// TODO: test task").unwrap();
        temp_file.flush().unwrap();

        // Add the file to pending
        let file_path = temp_file.path().to_path_buf();
        state.lock().unwrap().add_file(file_path.clone());

        let opts = WatchOptions {
            patterns: vec!["*.rs".to_string()],
            debounce_ms: 100,
            auto_queue: false, // Don't actually queue, just test processing
            notify: false,
            ignore_patterns: vec![],
            comment_types: vec![CommentType::Todo],
            paths: vec![PathBuf::from(".")],
            force: false,
            close_removed: false,
        };

        let comment_regex = build_comment_regex(&opts.comment_types).unwrap();
        let mut last_processed: HashMap<PathBuf, Instant> = HashMap::new();

        // Process the pending file
        let result = process_pending_files(
            &resolved,
            &state,
            &comment_regex,
            &opts,
            &mut last_processed,
        );

        assert!(result.is_ok());

        // Verify the file was recorded in last_processed
        assert!(last_processed.contains_key(&file_path));

        // Verify state is empty (files were taken)
        assert!(state.lock().unwrap().pending_files.is_empty());
    }

    #[test]
    fn process_pending_files_skips_recently_processed() {
        let temp_dir = TempDir::new().unwrap();
        let resolved = create_test_resolved(&temp_dir);
        let state = Arc::new(Mutex::new(WatchState::new(100)));

        // Create a temp file with a TODO comment
        let mut temp_file = NamedTempFile::new_in(temp_dir.path()).unwrap();
        writeln!(temp_file, "// TODO: test task").unwrap();
        temp_file.flush().unwrap();

        let file_path = temp_file.path().to_path_buf();

        // Pre-populate last_processed with very recent timestamp
        let mut last_processed: HashMap<PathBuf, Instant> = HashMap::new();
        let original_timestamp = Instant::now();
        last_processed.insert(file_path.clone(), original_timestamp);

        // Add file to pending
        state.lock().unwrap().add_file(file_path.clone());

        let opts = WatchOptions {
            patterns: vec!["*.rs".to_string()],
            debounce_ms: 10000, // Very long debounce - should prevent re-processing
            auto_queue: false,
            notify: false,
            ignore_patterns: vec![],
            comment_types: vec![CommentType::Todo],
            paths: vec![PathBuf::from(".")],
            force: false,
            close_removed: false,
        };

        let comment_regex = build_comment_regex(&opts.comment_types).unwrap();

        // Process - should skip due to recent processing
        let result = process_pending_files(
            &resolved,
            &state,
            &comment_regex,
            &opts,
            &mut last_processed,
        );

        assert!(result.is_ok());
        // File should still be in last_processed with original timestamp (not updated)
        assert!(last_processed.contains_key(&file_path));
        assert_eq!(
            last_processed.get(&file_path).unwrap(),
            &original_timestamp,
            "Timestamp should not be updated when file is skipped due to debounce"
        );
        // Queue should remain empty since file was skipped
        let queue = crate::queue::load_queue(&resolved.queue_path).unwrap();
        assert!(
            queue.tasks.is_empty(),
            "No tasks should be created for skipped file"
        );
    }

    #[test]
    fn process_pending_files_handles_empty_queue() {
        let temp_dir = TempDir::new().unwrap();
        let resolved = create_test_resolved(&temp_dir);
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

        // Process with no pending files
        let result = process_pending_files(
            &resolved,
            &state,
            &comment_regex,
            &opts,
            &mut last_processed,
        );

        assert!(result.is_ok());
        assert!(last_processed.is_empty());
    }

    #[test]
    fn process_pending_files_handles_multiple_files() {
        let temp_dir = TempDir::new().unwrap();
        let resolved = create_test_resolved(&temp_dir);
        let state = Arc::new(Mutex::new(WatchState::new(100)));

        // Create multiple temp files with TODO comments
        let mut temp_file1 = NamedTempFile::new_in(temp_dir.path()).unwrap();
        writeln!(temp_file1, "// TODO: task 1").unwrap();
        temp_file1.flush().unwrap();
        let path1 = temp_file1.path().to_path_buf();

        let mut temp_file2 = NamedTempFile::new_in(temp_dir.path()).unwrap();
        writeln!(temp_file2, "// FIXME: task 2").unwrap();
        temp_file2.flush().unwrap();
        let path2 = temp_file2.path().to_path_buf();

        let mut temp_file3 = NamedTempFile::new_in(temp_dir.path()).unwrap();
        writeln!(temp_file3, "// HACK: task 3").unwrap();
        temp_file3.flush().unwrap();
        let path3 = temp_file3.path().to_path_buf();

        // Add all files to pending
        {
            let mut state_guard = state.lock().unwrap();
            state_guard.add_file(path1.clone());
            state_guard.add_file(path2.clone());
            state_guard.add_file(path3.clone());
        }

        // Use CommentType::All to catch all comment types
        let opts = WatchOptions {
            patterns: vec!["*.rs".to_string()],
            debounce_ms: 100,
            auto_queue: false,
            notify: false,
            ignore_patterns: vec![],
            comment_types: vec![CommentType::All],
            paths: vec![PathBuf::from(".")],
            force: false,
            close_removed: false,
        };

        let comment_regex = build_comment_regex(&opts.comment_types).unwrap();
        let mut last_processed: HashMap<PathBuf, Instant> = HashMap::new();

        // Process all pending files
        let result = process_pending_files(
            &resolved,
            &state,
            &comment_regex,
            &opts,
            &mut last_processed,
        );

        assert!(result.is_ok());

        // All files should be recorded in last_processed
        assert!(last_processed.contains_key(&path1));
        assert!(last_processed.contains_key(&path2));
        assert!(last_processed.contains_key(&path3));

        // State should be empty
        assert!(state.lock().unwrap().pending_files.is_empty());
    }

    #[test]
    fn process_pending_files_handles_file_read_error() {
        let temp_dir = TempDir::new().unwrap();
        let resolved = create_test_resolved(&temp_dir);
        let state = Arc::new(Mutex::new(WatchState::new(100)));

        // Add a non-existent file to pending
        let nonexistent_path = temp_dir.path().join("nonexistent.rs");
        state.lock().unwrap().add_file(nonexistent_path.clone());

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

        // Process - should handle the error gracefully
        let result = process_pending_files(
            &resolved,
            &state,
            &comment_regex,
            &opts,
            &mut last_processed,
        );

        // Should succeed even though file read failed
        assert!(result.is_ok());
        // Non-existent file should not be in last_processed
        assert!(!last_processed.contains_key(&nonexistent_path));
    }

    #[test]
    fn process_pending_files_respects_auto_queue_option() {
        let temp_dir = TempDir::new().unwrap();
        let resolved = create_test_resolved(&temp_dir);
        let state = Arc::new(Mutex::new(WatchState::new(100)));

        // Create a temp file with a TODO comment
        let mut temp_file = NamedTempFile::new_in(temp_dir.path()).unwrap();
        writeln!(temp_file, "// TODO: test task").unwrap();
        temp_file.flush().unwrap();

        let file_path = temp_file.path().to_path_buf();
        state.lock().unwrap().add_file(file_path.clone());

        // Test with auto_queue = true
        let opts = WatchOptions {
            patterns: vec!["*.rs".to_string()],
            debounce_ms: 100,
            auto_queue: true, // Enable auto-queue
            notify: false,
            ignore_patterns: vec![],
            comment_types: vec![CommentType::Todo],
            paths: vec![PathBuf::from(".")],
            force: false,
            close_removed: false,
        };

        let comment_regex = build_comment_regex(&opts.comment_types).unwrap();
        let mut last_processed: HashMap<PathBuf, Instant> = HashMap::new();

        let result = process_pending_files(
            &resolved,
            &state,
            &comment_regex,
            &opts,
            &mut last_processed,
        );

        assert!(result.is_ok());

        // Verify queue was modified (task should have been added)
        let updated_queue = crate::queue::load_queue(&resolved.queue_path).unwrap();
        // Should have created a task for the TODO comment
        assert!(!updated_queue.tasks.is_empty());
    }

    #[test]
    fn process_pending_files_with_no_matching_comments() {
        let temp_dir = TempDir::new().unwrap();
        let resolved = create_test_resolved(&temp_dir);
        let state = Arc::new(Mutex::new(WatchState::new(100)));

        // Create a temp file with NO TODO comments
        let mut temp_file = NamedTempFile::new_in(temp_dir.path()).unwrap();
        writeln!(temp_file, "// This is a normal comment").unwrap();
        writeln!(temp_file, "fn main() {{}}").unwrap();
        temp_file.flush().unwrap();

        let file_path = temp_file.path().to_path_buf();
        state.lock().unwrap().add_file(file_path.clone());

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

        // Process the file
        let result = process_pending_files(
            &resolved,
            &state,
            &comment_regex,
            &opts,
            &mut last_processed,
        );

        assert!(result.is_ok());

        // File should be recorded as processed
        assert!(last_processed.contains_key(&file_path));

        // Queue should be empty (no tasks created)
        let queue = crate::queue::load_queue(&resolved.queue_path).unwrap();
        assert!(queue.tasks.is_empty());
    }

    #[test]
    fn process_pending_files_with_mixed_comment_types() {
        let temp_dir = TempDir::new().unwrap();
        let resolved = create_test_resolved(&temp_dir);
        let state = Arc::new(Mutex::new(WatchState::new(100)));

        // Create a temp file with mixed comment types
        let mut temp_file = NamedTempFile::new_in(temp_dir.path()).unwrap();
        writeln!(temp_file, "// TODO: todo task").unwrap();
        writeln!(temp_file, "// FIXME: fixme task").unwrap();
        writeln!(temp_file, "// Normal comment").unwrap();
        writeln!(temp_file, "// HACK: hack task").unwrap();
        temp_file.flush().unwrap();

        let file_path = temp_file.path().to_path_buf();
        state.lock().unwrap().add_file(file_path.clone());

        // Only watch for TODO comments
        let opts = WatchOptions {
            patterns: vec!["*.rs".to_string()],
            debounce_ms: 100,
            auto_queue: true,
            notify: false,
            ignore_patterns: vec![],
            comment_types: vec![CommentType::Todo],
            paths: vec![PathBuf::from(".")],
            force: false,
            close_removed: false,
        };

        let comment_regex = build_comment_regex(&opts.comment_types).unwrap();
        let mut last_processed: HashMap<PathBuf, Instant> = HashMap::new();

        let result = process_pending_files(
            &resolved,
            &state,
            &comment_regex,
            &opts,
            &mut last_processed,
        );

        assert!(result.is_ok());

        // Only TODO task should be queued
        let queue = crate::queue::load_queue(&resolved.queue_path).unwrap();
        assert_eq!(queue.tasks.len(), 1);
        assert!(queue.tasks[0].title.contains("TODO"));
    }
}
