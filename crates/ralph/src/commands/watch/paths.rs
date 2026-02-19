//! Path filtering and pattern matching for the watch command.
//!
//! Responsibilities:
//! - Filter file paths from watch events based on patterns and ignore rules.
//! - Match filenames against glob patterns using globset.
//!
//! Not handled here:
//! - File watching or event handling (see `event_loop.rs`).
//! - Comment detection (see `comments.rs`).
//!
//! Invariants/assumptions:
//! - Directories are always skipped.
//! - Ignore patterns take precedence over include patterns.
//! - Common ignore directories (target/, node_modules/, .git/, etc.) are hardcoded.

use crate::commands::watch::types::WatchOptions;
use notify::Event;
use std::path::{Path, PathBuf};

/// Get relevant file paths from a watch event.
pub fn get_relevant_paths(event: &Event, opts: &WatchOptions) -> Option<Vec<PathBuf>> {
    let paths: Vec<PathBuf> = event
        .paths
        .iter()
        .filter(|p| should_process_file(p, &opts.patterns, &opts.ignore_patterns))
        .cloned()
        .collect();

    if paths.is_empty() { None } else { Some(paths) }
}

/// Check if a file should be processed based on patterns and ignore rules.
pub fn should_process_file(path: &Path, patterns: &[String], ignore_patterns: &[String]) -> bool {
    // Skip directories
    if path.is_dir() {
        return false;
    }

    // Check if file matches any pattern
    let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

    // Check ignore patterns first
    for ignore in ignore_patterns {
        if matches_pattern(file_name, ignore) {
            return false;
        }
    }

    // Check if in common ignore directories
    let path_str = path.to_string_lossy();
    let ignore_dirs = [
        "/target/",
        "/node_modules/",
        "/.git/",
        "/vendor/",
        "/.ralph/",
    ];
    for dir in &ignore_dirs {
        if path_str.contains(dir) {
            return false;
        }
    }

    // Check if file matches any pattern
    patterns.iter().any(|p| matches_pattern(file_name, p))
}

/// Match a filename against a glob pattern using globset.
///
/// Supports standard glob syntax:
/// - `*` matches any sequence of characters (except `/`)
/// - `?` matches any single character
/// - `[abc]` matches any character in the set
/// - `[a-z]` matches any character in the range
pub fn matches_pattern(name: &str, pattern: &str) -> bool {
    globset::Glob::new(pattern)
        .map(|g| g.compile_matcher().is_match(name))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    //! Unit tests for path filtering and pattern matching.
    //!
    //! Responsibilities:
    //! - Test glob pattern matching for filenames
    //! - Test file filtering based on patterns and ignore rules
    //! - Test event path extraction and filtering
    //!
    //! Not handled here:
    //! - File system operations (just path string logic)
    //! - Comment detection (see comments.rs)
    //! - Full watch loop integration (see event_loop.rs)

    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn matches_pattern_basic() {
        assert!(matches_pattern("test.rs", "*.rs"));
        assert!(matches_pattern("test.rs", "test.*"));
        assert!(!matches_pattern("test.py", "*.rs"));
    }

    #[test]
    fn matches_pattern_question() {
        assert!(matches_pattern("test.rs", "t??t.rs"));
        assert!(!matches_pattern("test.rs", "t?t.rs"));
    }

    #[test]
    fn matches_pattern_regex_metacharacters() {
        // Character class patterns - these would break with the old regex-based implementation
        // Note: *.[rs] matches files ending in .r or .s (single char), not .rs
        assert!(matches_pattern("test.r", "*.[rs]"));
        assert!(matches_pattern("test.s", "*.[rs]"));
        assert!(!matches_pattern("test.rs", "*.[rs]"));
        assert!(!matches_pattern("test.py", "*.[rs]"));

        // Plus sign in filename - + is literal in glob, not a regex quantifier
        assert!(matches_pattern("file+1.txt", "file+*.txt"));
        assert!(matches_pattern("file+123.txt", "file+*.txt"));

        // Parentheses in filename - () are literal in glob, not regex groups
        assert!(matches_pattern("test(1).rs", "test(*).rs"));
        assert!(matches_pattern("test(backup).rs", "test(*).rs"));

        // Dollar signs in filename - $ is literal in glob, not regex anchor
        assert!(matches_pattern("test.$$$", "test.*"));
        assert!(matches_pattern("file.$$$.txt", "file.*.txt"));

        // Caret in filename - ^ is literal in glob, not regex anchor
        assert!(matches_pattern("file^name.txt", "file^name.txt"));
        assert!(matches_pattern("file^name.txt", "file*.txt"));
    }

    #[test]
    fn matches_pattern_character_classes() {
        // Range patterns
        assert!(matches_pattern("file1.txt", "file[0-9].txt"));
        assert!(matches_pattern("file5.txt", "file[0-9].txt"));
        assert!(matches_pattern("file9.txt", "file[0-9].txt"));
        assert!(!matches_pattern("filea.txt", "file[0-9].txt"));

        // Multiple character classes
        assert!(matches_pattern("test_a.rs", "test_[a-z].rs"));
        assert!(matches_pattern("test_z.rs", "test_[a-z].rs"));
        assert!(!matches_pattern("test_1.rs", "test_[a-z].rs"));
    }

    #[test]
    fn matches_pattern_edge_cases() {
        // Empty pattern should only match empty string
        assert!(matches_pattern("", ""));
        assert!(!matches_pattern("test.rs", ""));

        // Invalid glob patterns should return false (not panic)
        // Unclosed character class is invalid in globset
        assert!(!matches_pattern("test.rs", "*.[rs"));

        // Just wildcards
        assert!(matches_pattern("anything", "*"));
        assert!(matches_pattern("a", "?"));
        assert!(!matches_pattern("ab", "?"));
    }

    // =========================================================================
    // Tests for should_process_file
    // =========================================================================

    #[test]
    fn should_process_file_skips_directories() {
        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path().join("testdir");
        fs::create_dir(&dir_path).unwrap();

        // Even with matching pattern, directory should be skipped
        assert!(!should_process_file(&dir_path, &["*".to_string()], &[]));
    }

    #[test]
    fn should_process_file_requires_pattern_match() {
        let path = Path::new("file.txt");
        // Does not match *.rs pattern
        assert!(!should_process_file(path, &["*.rs".to_string()], &[]));
        // Does match *.txt pattern
        assert!(should_process_file(path, &["*.txt".to_string()], &[]));
    }

    #[test]
    fn should_process_file_ignores_take_precedence() {
        let path = Path::new("test.rs");
        // File matches include pattern *.rs
        assert!(should_process_file(path, &["*.rs".to_string()], &[]));
        // But should be ignored when ignore pattern test.* is present
        assert!(!should_process_file(
            path,
            &["*.rs".to_string()],
            &["test.*".to_string()]
        ));
    }

    #[test]
    fn should_process_file_skips_target_dir() {
        let path = Path::new("/project/target/debug/main.rs");
        // Even though it's a .rs file, target/ is hardcoded to be ignored
        assert!(!should_process_file(path, &["*.rs".to_string()], &[]));
    }

    #[test]
    fn should_process_file_skips_node_modules() {
        let path = Path::new("/project/node_modules/package/index.js");
        assert!(!should_process_file(path, &["*.js".to_string()], &[]));
    }

    #[test]
    fn should_process_file_skips_git_dir() {
        let path = Path::new("/project/.git/hooks/pre-commit");
        assert!(!should_process_file(path, &["*".to_string()], &[]));
    }

    #[test]
    fn should_process_file_skips_vendor_dir() {
        let path = Path::new("/project/vendor/lib/helper.rb");
        assert!(!should_process_file(path, &["*.rb".to_string()], &[]));
    }

    #[test]
    fn should_process_file_skips_ralph_dir() {
        let path = Path::new("/project/.ralph/cache/data.json");
        assert!(!should_process_file(path, &["*.json".to_string()], &[]));
    }

    // =========================================================================
    // Tests for get_relevant_paths
    // =========================================================================

    #[test]
    fn get_relevant_paths_filters_event_paths() {
        let event = Event {
            kind: notify::EventKind::Modify(notify::event::ModifyKind::Data(
                notify::event::DataChange::Content,
            )),
            paths: vec![
                PathBuf::from("test.rs"),  // Should match *.rs
                PathBuf::from("other.py"), // Should not match
            ],
            attrs: Default::default(),
        };
        let opts = WatchOptions {
            patterns: vec!["*.rs".to_string()],
            ignore_patterns: vec![],
            debounce_ms: 100,
            auto_queue: false,
            notify: false,
            comment_types: vec![],
            paths: vec![],
            force: false,
            close_removed: false,
        };

        let result = get_relevant_paths(&event, &opts);
        assert!(result.is_some());
        let paths = result.unwrap();
        assert_eq!(paths.len(), 1);
        assert!(paths[0].ends_with("test.rs"));
    }

    #[test]
    fn get_relevant_paths_returns_none_when_no_matches() {
        let event = Event {
            kind: notify::EventKind::Modify(notify::event::ModifyKind::Data(
                notify::event::DataChange::Content,
            )),
            paths: vec![PathBuf::from("file.txt"), PathBuf::from("file.py")],
            attrs: Default::default(),
        };
        let opts = WatchOptions {
            patterns: vec!["*.rs".to_string()],
            ignore_patterns: vec![],
            debounce_ms: 100,
            auto_queue: false,
            notify: false,
            comment_types: vec![],
            paths: vec![],
            force: false,
            close_removed: false,
        };

        assert!(get_relevant_paths(&event, &opts).is_none());
    }

    #[test]
    fn get_relevant_paths_respects_ignore_patterns() {
        let event = Event {
            kind: notify::EventKind::Modify(notify::event::ModifyKind::Data(
                notify::event::DataChange::Content,
            )),
            paths: vec![
                PathBuf::from("include.rs"), // Should match
                PathBuf::from("exclude.rs"), // Should be ignored
            ],
            attrs: Default::default(),
        };
        let opts = WatchOptions {
            patterns: vec!["*.rs".to_string()],
            ignore_patterns: vec!["exclude.*".to_string()],
            debounce_ms: 100,
            auto_queue: false,
            notify: false,
            comment_types: vec![],
            paths: vec![],
            force: false,
            close_removed: false,
        };

        let result = get_relevant_paths(&event, &opts);
        assert!(result.is_some());
        let paths = result.unwrap();
        assert_eq!(paths.len(), 1);
        assert!(paths[0].ends_with("include.rs"));
    }
}
