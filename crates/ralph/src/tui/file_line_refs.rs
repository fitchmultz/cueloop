//! File:line reference parsing utilities for the TUI.
//!
//! Responsibilities:
//! - Extract `path:line` references from arbitrary user text (notes/evidence).
//! - Provide stable formatting for clipboard output.
//!
//! Does NOT handle:
//! - Validating that paths exist on disk.
//! - Opening editors or interacting with the clipboard.
//!
//! Invariants/assumptions:
//! - Line numbers are base-10 positive integers.
//! - Paths are treated as opaque strings; normalization is minimal (trim punctuation).

use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FileLineRef {
    pub(crate) path: String,
    pub(crate) line: u32,
}

pub(crate) fn extract_file_line_refs<'a, I>(inputs: I) -> Vec<FileLineRef>
where
    I: IntoIterator<Item = &'a str>,
{
    let mut out = Vec::new();

    for input in inputs {
        let bytes = input.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] != b':' {
                i += 1;
                continue;
            }

            // Parse digits after ':'
            let mut j = i + 1;
            let start_digits = j;
            while j < bytes.len() && bytes[j].is_ascii_digit() {
                j += 1;
            }
            if start_digits == j {
                i += 1;
                continue;
            }

            let line: u32 = match input[start_digits..j].parse() {
                Ok(n) if n > 0 => n,
                _ => {
                    i = j;
                    continue;
                }
            };

            // Walk backwards to find start of path token.
            let mut k = i;
            while k > 0 {
                let c = bytes[k - 1] as char;
                let allowed =
                    c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '/' | '.' | '\\');
                if !allowed {
                    break;
                }
                k -= 1;
            }

            let raw_path =
                input[k..i].trim_matches(|c: char| matches!(c, '(' | '[' | '{' | '"' | '\'' | '`'));
            let path = raw_path.trim_end_matches(|c: char| {
                matches!(
                    c,
                    ',' | '.' | ';' | ':' | ')' | ']' | '}' | '"' | '\'' | '`'
                )
            });

            if !path.is_empty() {
                out.push(FileLineRef {
                    path: path.to_string(),
                    line,
                });
            }

            i = j;
        }
    }

    out
}

pub(crate) fn format_refs_for_clipboard(refs: &[FileLineRef]) -> String {
    let mut seen: HashSet<(String, u32)> = HashSet::new();
    let mut lines = Vec::new();

    for r in refs {
        let key = (r.path.clone(), r.line);
        if seen.insert(key) {
            lines.push(format!("{}:{}", r.path, r.line));
        }
    }

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_basic_refs_and_strips_punctuation() {
        let refs = extract_file_line_refs([
            "See src/main.rs:42, and `./crates/ralph/src/tui/app.rs:855`.",
            "(docs/tui-task-management.md:10)",
        ]);
        assert_eq!(
            refs,
            vec![
                FileLineRef {
                    path: "src/main.rs".to_string(),
                    line: 42
                },
                FileLineRef {
                    path: "./crates/ralph/src/tui/app.rs".to_string(),
                    line: 855
                },
                FileLineRef {
                    path: "docs/tui-task-management.md".to_string(),
                    line: 10
                },
            ]
        );
        assert_eq!(
            format_refs_for_clipboard(&refs),
            "src/main.rs:42\n./crates/ralph/src/tui/app.rs:855\ndocs/tui-task-management.md:10"
        );
    }

    #[test]
    fn ignores_colons_without_digits() {
        let refs = extract_file_line_refs(["http://example.com:abc", "foo:bar", "x:y"]);
        assert!(refs.is_empty());
    }

    #[test]
    fn dedupes_on_formatting() {
        let refs = vec![
            FileLineRef {
                path: "a.rs".to_string(),
                line: 1,
            },
            FileLineRef {
                path: "a.rs".to_string(),
                line: 1,
            },
            FileLineRef {
                path: "a.rs".to_string(),
                line: 2,
            },
        ];
        assert_eq!(format_refs_for_clipboard(&refs), "a.rs:1\na.rs:2");
    }

    #[test]
    fn handles_windows_paths() {
        let refs = extract_file_line_refs(["src\\main.rs:42"]);
        assert_eq!(
            refs,
            vec![FileLineRef {
                path: "src\\main.rs".to_string(),
                line: 42
            }]
        );
    }

    #[test]
    fn handles_multiple_refs_in_single_line() {
        let refs = extract_file_line_refs(["See src/a.rs:10 and src/b.rs:20"]);
        assert_eq!(
            refs,
            vec![
                FileLineRef {
                    path: "src/a.rs".to_string(),
                    line: 10
                },
                FileLineRef {
                    path: "src/b.rs".to_string(),
                    line: 20
                },
            ]
        );
    }

    #[test]
    fn ignores_line_zero() {
        let refs = extract_file_line_refs(["src/main.rs:0"]);
        assert!(refs.is_empty());
    }
}
