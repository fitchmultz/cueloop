//! State management for the watch command.
//!
//! Responsibilities:
//! - Track pending files that need processing.
//! - Manage debounce timing to batch rapid file changes.
//!
//! Not handled here:
//! - File watching or event handling (see `event_loop.rs`).
//! - Comment detection (see `comments.rs`).
//! - Task creation (see `tasks.rs`).
//!
//! Invariants/assumptions:
//! - `WatchState` is protected by a mutex in the watch loop.
//! - `debounce_duration` is derived from `WatchOptions.debounce_ms`.
//! - `last_event` is updated when files are added or taken.

use std::collections::HashSet;
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// Internal state for the file watcher.
pub struct WatchState {
    pub pending_files: HashSet<PathBuf>,
    pub last_event: Instant,
    pub debounce_duration: Duration,
}

impl WatchState {
    pub fn new(debounce_ms: u64) -> Self {
        Self {
            pending_files: HashSet::new(),
            last_event: Instant::now(),
            debounce_duration: Duration::from_millis(debounce_ms),
        }
    }

    /// Add a file to pending set. Returns true if debounce window has elapsed.
    pub fn add_file(&mut self, path: PathBuf) -> bool {
        self.pending_files.insert(path);
        let now = Instant::now();
        if now.duration_since(self.last_event) >= self.debounce_duration {
            self.last_event = now;
            true
        } else {
            false
        }
    }

    /// Take all pending files and reset the event timer.
    pub fn take_pending(&mut self) -> Vec<PathBuf> {
        let files: Vec<PathBuf> = self.pending_files.drain().collect();
        self.last_event = Instant::now();
        files
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests for watch state management.
    //!
    //! Responsibilities:
    //! - Test WatchState initialization with various debounce durations
    //! - Test file tracking (add, deduplicate, take pending)
    //! - Test debounce timing behavior
    //!
    //! Not handled here:
    //! - File watching events (see event_loop.rs)
    //! - Comment detection (see comments.rs)
    //! - Mutex handling (tested in integration)

    use super::*;

    #[test]
    fn watch_state_new_initializes_empty() {
        let state = WatchState::new(100);
        assert!(state.pending_files.is_empty());
        assert_eq!(state.debounce_duration, Duration::from_millis(100));
    }

    #[test]
    fn add_file_inserts_new_path() {
        let mut state = WatchState::new(100);
        let path = PathBuf::from("/test/file.rs");

        state.add_file(path.clone());

        assert!(state.pending_files.contains(&path));
        assert_eq!(state.pending_files.len(), 1);
    }

    #[test]
    fn add_file_deduplicates_paths() {
        let mut state = WatchState::new(100);
        let path = PathBuf::from("/test/file.rs");

        state.add_file(path.clone());
        state.add_file(path.clone()); // Duplicate

        assert_eq!(state.pending_files.len(), 1);
    }

    #[test]
    fn add_file_returns_true_when_debounce_elapsed() {
        // This test verifies the return value when debounce window has passed
        // We need to wait for debounce to elapse
        let mut state = WatchState::new(0); // 0ms debounce for instant elapsed
        let path = PathBuf::from("/test/file.rs");

        // Should return true because debounce duration is 0
        let should_process = state.add_file(path);

        assert!(should_process);
    }

    #[test]
    fn add_file_returns_false_within_debounce() {
        let mut state = WatchState::new(10000); // Very long debounce
        let path1 = PathBuf::from("/test/file1.rs");
        let path2 = PathBuf::from("/test/file2.rs");

        // First file should return true (debounce window starts)
        let _ = state.add_file(path1);

        // Second file within debounce window should return false
        let should_process = state.add_file(path2);

        assert!(!should_process);
    }

    #[test]
    fn take_pending_removes_all_files() {
        let mut state = WatchState::new(100);
        state.add_file(PathBuf::from("/test/file1.rs"));
        state.add_file(PathBuf::from("/test/file2.rs"));

        let pending = state.take_pending();

        assert_eq!(pending.len(), 2);
        assert!(state.pending_files.is_empty());
    }

    #[test]
    fn take_pending_returns_empty_when_no_files() {
        let mut state = WatchState::new(100);

        let pending = state.take_pending();

        assert!(pending.is_empty());
    }

    #[test]
    fn take_pending_resets_last_event() {
        let mut state = WatchState::new(100);
        state.add_file(PathBuf::from("/test/file.rs"));
        let before = state.last_event;

        state.take_pending();

        // last_event should be updated (not equal to before)
        assert_ne!(
            state.last_event, before,
            "last_event should be updated when taking pending files"
        );
    }

    #[test]
    fn take_pending_preserves_all_files() {
        // HashSet doesn't guarantee order, but we should test that
        // all files are returned (not checking specific order)
        let mut state = WatchState::new(100);
        state.add_file(PathBuf::from("/test/a.rs"));
        state.add_file(PathBuf::from("/test/b.rs"));
        state.add_file(PathBuf::from("/test/c.rs"));

        let pending = state.take_pending();

        assert_eq!(pending.len(), 3);
        // Verify all expected files are present
        assert!(pending.iter().any(|p| p.ends_with("a.rs")));
        assert!(pending.iter().any(|p| p.ends_with("b.rs")));
        assert!(pending.iter().any(|p| p.ends_with("c.rs")));
    }

    #[test]
    fn add_file_multiple_paths_tracked() {
        let mut state = WatchState::new(100);
        let path1 = PathBuf::from("/test/file1.rs");
        let path2 = PathBuf::from("/test/file2.rs");
        let path3 = PathBuf::from("/test/file3.rs");

        state.add_file(path1.clone());
        state.add_file(path2.clone());
        state.add_file(path3.clone());

        assert_eq!(state.pending_files.len(), 3);
        assert!(state.pending_files.contains(&path1));
        assert!(state.pending_files.contains(&path2));
        assert!(state.pending_files.contains(&path3));
    }

    #[test]
    fn add_file_updates_last_event_when_debounce_elapsed() {
        let mut state = WatchState::new(0); // 0ms means debounce always elapsed
        let before = state.last_event;

        state.add_file(PathBuf::from("/test/file.rs"));

        // last_event should be updated because debounce elapsed (0ms)
        assert_ne!(
            state.last_event, before,
            "last_event should be updated when debounce has elapsed"
        );
    }

    #[test]
    fn take_pending_empty_then_add_new_files() {
        let mut state = WatchState::new(100);

        // Take pending when empty
        let pending = state.take_pending();
        assert!(pending.is_empty());

        // Add new files after taking
        state.add_file(PathBuf::from("/test/file.rs"));
        assert_eq!(state.pending_files.len(), 1);
    }

    #[test]
    fn debounce_duration_zero_always_elapsed() {
        let mut state = WatchState::new(0);
        let path1 = PathBuf::from("/test/file1.rs");
        let path2 = PathBuf::from("/test/file2.rs");

        // Both should return true with 0ms debounce
        assert!(state.add_file(path1));
        assert!(state.add_file(path2));
    }
}
