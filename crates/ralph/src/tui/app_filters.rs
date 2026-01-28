//! Filter-related helper functions for the TUI.
//!
//! Responsibilities:
//! - Provide pure functions for filter token normalization and key generation.
//!
//! Not handled here:
//! - Filter state management (handled in app.rs).
//! - Filter caching logic (handled in app.rs).

/// Normalize a filter token for consistent cache hits.
pub fn normalize_filter_token(value: &str) -> String {
    value.trim().to_lowercase()
}

/// Parse comma or newline-separated list from input string.
#[allow(dead_code)]
pub fn parse_list(input: &str) -> Vec<String> {
    input
        .split([',', '\n'])
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
        .collect()
}

/// Parse comma or whitespace-separated tags from input string.
#[allow(dead_code)]
pub fn parse_tags(input: &str) -> Vec<String> {
    input
        .split(|c: char| c == ',' || c.is_whitespace())
        .map(|tag| tag.trim().to_string())
        .filter(|tag| !tag.is_empty())
        .collect()
}
