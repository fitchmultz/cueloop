//! Comment detection for the watch command.
//!
//! Responsibilities:
//! - Build regex patterns for detecting TODO/FIXME/HACK/XXX comments.
//! - Detect comments in source files.
//! - Determine comment type from line content.
//! - Extract context for detected comments.
//!
//! Not handled here:
//! - File watching (see `event_loop.rs`).
//! - Task creation from comments (see `tasks.rs`).
//!
//! Invariants/assumptions:
//! - Comment regex is case-insensitive.
//! - Comment content is extracted from capture groups.
//! - Context includes filename, line number, and truncated content.

use crate::commands::watch::types::{CommentType, DetectedComment};
use anyhow::{Context, Result};
use regex::Regex;
use std::path::Path;

/// Build regex for detecting comments based on comment types.
pub fn build_comment_regex(comment_types: &[CommentType]) -> Result<Regex> {
    let mut patterns = Vec::new();

    let has_all = comment_types.contains(&CommentType::All);

    if has_all || comment_types.contains(&CommentType::Todo) {
        patterns.push(r"TODO\s*[:;-]?\s*(.+)$");
    }
    if has_all || comment_types.contains(&CommentType::Fixme) {
        patterns.push(r"FIXME\s*[:;-]?\s*(.+)$");
    }
    if has_all || comment_types.contains(&CommentType::Hack) {
        patterns.push(r"HACK\s*[:;-]?\s*(.+)$");
    }
    if has_all || comment_types.contains(&CommentType::Xxx) {
        patterns.push(r"XXX\s*[:;-]?\s*(.+)$");
    }

    if patterns.is_empty() {
        patterns.push(r"(?:TODO|FIXME|HACK|XXX)\s*[:;-]?\s*(.+)$");
    }

    let combined = patterns.join("|");
    let regex = Regex::new(&format!(r"(?i)({})", combined))
        .context("Failed to compile comment detection regex")?;

    Ok(regex)
}

/// Detect comments in a file.
pub fn detect_comments(file_path: &Path, regex: &Regex) -> Result<Vec<DetectedComment>> {
    let content = std::fs::read_to_string(file_path)
        .with_context(|| format!("Failed to read file: {}", file_path.display()))?;

    let mut comments = Vec::new();

    for (line_num, line) in content.lines().enumerate() {
        if let Some(captures) = regex.captures(line) {
            // Extract the comment content
            let content = captures
                .get(1)
                .or_else(|| captures.get(2))
                .or_else(|| captures.get(3))
                .or_else(|| captures.get(4))
                .map(|m| m.as_str().trim().to_string())
                .unwrap_or_default();

            if content.is_empty() {
                continue;
            }

            // Determine comment type from the match
            let comment_type = determine_comment_type(line);

            // Get context (surrounding lines)
            let context = extract_context(&content, line_num + 1, file_path);

            comments.push(DetectedComment {
                file_path: file_path.to_path_buf(),
                line_number: line_num + 1,
                comment_type,
                content,
                context,
            });
        }
    }

    Ok(comments)
}

/// Determine the comment type from a line.
pub fn determine_comment_type(line: &str) -> CommentType {
    let upper = line.to_uppercase();
    if upper.contains("TODO") {
        CommentType::Todo
    } else if upper.contains("FIXME") {
        CommentType::Fixme
    } else if upper.contains("HACK") {
        CommentType::Hack
    } else if upper.contains("XXX") {
        CommentType::Xxx
    } else {
        CommentType::All
    }
}

/// Extract context for a comment.
pub fn extract_context(content: &str, line_number: usize, file_path: &Path) -> String {
    format!(
        "{}:{} - {}",
        file_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown"),
        line_number,
        content.chars().take(100).collect::<String>()
    )
}

#[cfg(test)]
mod tests {
    //! Unit tests for comment detection functionality.
    //!
    //! Responsibilities:
    //! - Test regex compilation for all comment type combinations
    //! - Test comment detection in source files with various formats
    //! - Test comment type determination and priority ordering
    //! - Test context extraction and formatting
    //! - Test edge cases: empty content, missing files, truncation
    //!
    //! Not handled here:
    //! - File watching integration (see event_loop.rs)
    //! - Task creation from comments (see tasks.rs)
    //! - Debouncing logic (see debounce.rs)

    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn determine_comment_type_detection() {
        assert_eq!(
            determine_comment_type("// TODO: fix this"),
            CommentType::Todo
        );
        assert_eq!(
            determine_comment_type("// FIXME: broken"),
            CommentType::Fixme
        );
        assert_eq!(
            determine_comment_type("// HACK: workaround"),
            CommentType::Hack
        );
        assert_eq!(
            determine_comment_type("// XXX: review needed"),
            CommentType::Xxx
        );
    }

    #[test]
    fn extract_context_format() {
        let ctx = extract_context("test content", 42, Path::new("/path/to/file.rs"));
        assert!(ctx.contains("file.rs"));
        assert!(ctx.contains("42"));
        assert!(ctx.contains("test content"));
    }

    #[test]
    fn detect_comments_finds_todos() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "// TODO: fix this bug").unwrap();
        writeln!(temp_file, "fn main() {{}}").unwrap();
        writeln!(temp_file, "// FIXME: handle error").unwrap();
        temp_file.flush().unwrap();

        let regex = build_comment_regex(&[CommentType::All]).unwrap();
        let comments = detect_comments(temp_file.path(), &regex).unwrap();

        assert_eq!(comments.len(), 2);
        // Content includes the marker for multi-type regex
        assert!(comments[0].content.contains("fix this bug"));
        assert_eq!(comments[0].comment_type, CommentType::Todo);
        assert!(comments[1].content.contains("handle error"));
        assert_eq!(comments[1].comment_type, CommentType::Fixme);
    }

    #[test]
    fn detect_comments_returns_error_for_missing_file() {
        let regex = build_comment_regex(&[CommentType::Todo]).unwrap();
        let result = detect_comments(Path::new("/nonexistent/file.rs"), &regex);
        assert!(result.is_err());
    }

    #[test]
    fn build_comment_regex_empty_types_defaults_to_all() {
        // Empty comment types should default to matching all types
        let regex = build_comment_regex(&[]).unwrap();
        assert!(regex.is_match("// TODO: fix this"));
        assert!(regex.is_match("// FIXME: fix this"));
        assert!(regex.is_match("// HACK: workaround"));
        assert!(regex.is_match("// XXX: review"));
    }

    #[test]
    fn build_comment_regex_single_type_only() {
        let regex = build_comment_regex(&[CommentType::Todo]).unwrap();
        assert!(regex.is_match("// TODO: fix this"));
        assert!(!regex.is_match("// FIXME: fix this"));
        assert!(!regex.is_match("// HACK: workaround"));
        assert!(!regex.is_match("// XXX: review"));
    }

    #[test]
    fn build_comment_regex_multiple_types() {
        let regex = build_comment_regex(&[CommentType::Todo, CommentType::Fixme]).unwrap();
        assert!(regex.is_match("// TODO: fix this"));
        assert!(regex.is_match("// FIXME: fix this"));
        assert!(!regex.is_match("// HACK: workaround"));
    }

    #[test]
    fn build_comment_regex_case_insensitive() {
        let regex = build_comment_regex(&[CommentType::Todo]).unwrap();
        assert!(regex.is_match("// todo: fix this"));
        assert!(regex.is_match("// Todo: fix this"));
        assert!(regex.is_match("// TODO: fix this"));
        assert!(regex.is_match("// ToDo: fix this"));
    }

    #[test]
    fn build_comment_regex_various_separators() {
        let regex = build_comment_regex(&[CommentType::All]).unwrap();
        // Colon separator
        assert!(regex.is_match("// TODO: fix this"));
        // Semicolon separator
        assert!(regex.is_match("// TODO; fix this"));
        // Dash separator
        assert!(regex.is_match("// TODO- fix this"));
        // No separator (just whitespace)
        assert!(regex.is_match("// TODO fix this"));
    }

    #[test]
    fn detect_comments_extracts_content_correctly() {
        // Test content extraction through the public API (not implementation details)
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "// TODO: implement feature X").unwrap();
        temp_file.flush().unwrap();

        let regex = build_comment_regex(&[CommentType::Todo]).unwrap();
        let comments = detect_comments(temp_file.path(), &regex).unwrap();

        assert_eq!(comments.len(), 1);
        // Content format varies by regex structure; verify the meaningful part is present
        assert!(comments[0].content.contains("implement feature X"));
    }

    #[test]
    fn detect_comments_handles_all_comment_types() {
        // Test that all comment markers (TODO, FIXME, HACK, XXX) are detected
        // when using CommentType::All
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "// FIXME: fix this").unwrap();
        writeln!(temp_file, "// TODO: do that").unwrap();
        writeln!(temp_file, "// HACK: workaround").unwrap();
        writeln!(temp_file, "// XXX: review needed").unwrap();
        temp_file.flush().unwrap();

        let regex = build_comment_regex(&[CommentType::All]).unwrap();
        let comments = detect_comments(temp_file.path(), &regex).unwrap();

        assert_eq!(comments.len(), 4, "All comment types should be detected");
        assert_eq!(comments[0].comment_type, CommentType::Fixme);
        assert_eq!(comments[1].comment_type, CommentType::Todo);
        assert_eq!(comments[2].comment_type, CommentType::Hack);
        assert_eq!(comments[3].comment_type, CommentType::Xxx);
    }

    #[test]
    fn determine_comment_type_prefers_first_match() {
        // If a line contains multiple markers, first match wins
        // Note: This tests current behavior - the first type found in order: TODO, FIXME, HACK, XXX
        assert_eq!(
            determine_comment_type("// TODO and FIXME here"),
            CommentType::Todo
        );
        assert_eq!(
            determine_comment_type("// FIXME and HACK here"),
            CommentType::Fixme
        );
        assert_eq!(
            determine_comment_type("// HACK and XXX here"),
            CommentType::Hack
        );
    }

    #[test]
    fn determine_comment_type_in_code_context() {
        // Comments within code
        assert_eq!(
            determine_comment_type("let x = 1; // TODO: optimize"),
            CommentType::Todo
        );
        assert_eq!(
            determine_comment_type("/* FIXME: broken */"),
            CommentType::Fixme
        );
        assert_eq!(
            determine_comment_type("/// HACK: workaround for bug"),
            CommentType::Hack
        );
    }

    #[test]
    fn determine_comment_type_returns_all_when_no_match() {
        assert_eq!(
            determine_comment_type("// Some regular comment"),
            CommentType::All
        );
        assert_eq!(
            determine_comment_type("Just code without comments"),
            CommentType::All
        );
        assert_eq!(determine_comment_type(""), CommentType::All);
    }

    #[test]
    fn extract_context_truncates_long_content() {
        let long_content = "a".repeat(200);
        let ctx = extract_context(&long_content, 1, Path::new("/test/file.rs"));

        // Should be truncated to 100 chars
        let content_part = ctx.split(" - ").last().unwrap();
        assert_eq!(content_part.len(), 100);
    }

    #[test]
    fn extract_context_handles_special_chars_in_filename() {
        let ctx = extract_context("content", 42, Path::new("/path/to/file-name_v2.rs"));
        assert!(ctx.contains("file-name_v2.rs"));
    }

    #[test]
    fn extract_context_line_number_one() {
        let ctx = extract_context("first line", 1, Path::new("/test/file.rs"));
        assert!(ctx.contains(":1 -"));
    }

    #[test]
    fn detect_comments_handles_multiline_content() {
        let mut temp_file = NamedTempFile::new().unwrap();
        // A comment with content that looks like multiple lines
        writeln!(temp_file, "// TODO: line 1").unwrap();
        writeln!(temp_file, "// TODO: line 2").unwrap();
        writeln!(temp_file, "// TODO: line 3").unwrap();
        temp_file.flush().unwrap();

        let regex = build_comment_regex(&[CommentType::All]).unwrap();
        let comments = detect_comments(temp_file.path(), &regex).unwrap();

        assert_eq!(comments.len(), 3);
        assert_eq!(comments[0].line_number, 1);
        assert_eq!(comments[1].line_number, 2);
        assert_eq!(comments[2].line_number, 3);
    }

    #[test]
    fn detect_comments_preserves_file_path() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "// TODO: test").unwrap();
        temp_file.flush().unwrap();

        let regex = build_comment_regex(&[CommentType::Todo]).unwrap();
        let comments = detect_comments(temp_file.path(), &regex).unwrap();

        assert_eq!(comments[0].file_path, temp_file.path());
    }

    #[test]
    fn detect_comments_with_fixme_type_only() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "// TODO: ignored").unwrap();
        writeln!(temp_file, "// FIXME: found").unwrap();
        writeln!(temp_file, "// HACK: ignored").unwrap();
        temp_file.flush().unwrap();

        let regex = build_comment_regex(&[CommentType::Fixme]).unwrap();
        let comments = detect_comments(temp_file.path(), &regex).unwrap();

        assert_eq!(comments.len(), 1);
        assert_eq!(comments[0].comment_type, CommentType::Fixme);
        // Content format varies based on regex structure
        assert!(comments[0].content.contains("found"));
    }

    #[test]
    fn detect_comments_preserves_content_case() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "// TODO: Fix The BUG in ModuleA").unwrap();
        temp_file.flush().unwrap();

        let regex = build_comment_regex(&[CommentType::Todo]).unwrap();
        let comments = detect_comments(temp_file.path(), &regex).unwrap();

        assert_eq!(comments.len(), 1);
        assert!(comments[0].content.contains("Fix The BUG in ModuleA"));
    }

    #[test]
    fn extract_context_with_unknown_file() {
        // Path with no file_name (rare case)
        let ctx = extract_context("content", 5, Path::new(""));
        assert!(ctx.contains("unknown"));
        assert!(ctx.contains(":5 -"));
    }

    #[test]
    fn build_comment_regex_all_type_matches_everything() {
        let regex = build_comment_regex(&[CommentType::All]).unwrap();
        assert!(regex.is_match("TODO: something"));
        assert!(regex.is_match("FIXME: something"));
        assert!(regex.is_match("HACK: something"));
        assert!(regex.is_match("XXX: something"));
    }

    #[test]
    fn determine_comment_type_xxx_variations() {
        assert_eq!(determine_comment_type("// XXX: review"), CommentType::Xxx);
        assert_eq!(determine_comment_type("// xxx: review"), CommentType::Xxx);
        assert_eq!(determine_comment_type("// XxX: review"), CommentType::Xxx);
    }
}
