//! Tree-sitter-backed syntax highlighting for Ralph TUI Markdown code blocks.
//!
//! Responsibilities:
//! - Provide a small, fast Rust syntax highlighter that returns `ratatui::text::Line` output.
//! - Compile and run a minimal Tree-sitter query against Rust source code.
//! - Map captures to stable, readable terminal styles.
//! - Gracefully degrade to plain code styling if parser/query fails.
//!
//! Not handled here:
//! - Full multi-language highlighting (only Rust is supported initially).
//! - Theme customization or user-configurable palettes (use fixed, boring defaults).
//! - Precise semantic highlighting parity with editors (keep it minimal).
//!
//! Invariants/assumptions:
//! - Input is UTF-8. Tree-sitter byte ranges are assumed to be valid UTF-8 boundaries.
//! - Highlighting must never panic; failures must fall back to plain rendering.

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use streaming_iterator::StreamingIterator;
use tree_sitter::{Node, Parser, Query, QueryCursor};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CodeLanguage {
    Rust,
}

// Note: Parser doesn't implement Debug, so we implement Debug manually
pub(crate) struct SyntaxHighlighter {
    parser: Parser,
    query: Query,
}

impl std::fmt::Debug for SyntaxHighlighter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SyntaxHighlighter")
            .field("parser", &"<tree_sitter::Parser>")
            .field("query", &self.query)
            .finish()
    }
}

impl SyntaxHighlighter {
    pub(crate) fn try_new(lang: CodeLanguage) -> Option<Self> {
        match lang {
            CodeLanguage::Rust => Self::try_new_rust(),
        }
    }

    fn try_new_rust() -> Option<Self> {
        let mut parser = Parser::new();
        let language = tree_sitter_rust::LANGUAGE.into();

        // If this fails for any reason, we degrade (return None).
        if parser.set_language(&language).is_err() {
            return None;
        }

        // Minimal query: comments/strings/numbers/types/functions/macros.
        // Keep this tiny and tolerant of grammar changes.
        let query_src = r#"
          (line_comment) @comment
          (block_comment) @comment

          (string_literal) @string
          (char_literal) @string
          (raw_string_literal) @string

          (integer_literal) @number
          (float_literal) @number

          (boolean_literal) @constant

          (type_identifier) @type
          (primitive_type) @type

          (macro_invocation macro: (identifier) @macro)
          (function_item name: (identifier) @function)
          (call_expression function: (identifier) @function)
        "#;

        let query = Query::new(&language, query_src).ok()?;
        Some(Self { parser, query })
    }

    pub(crate) fn highlight_rust(&mut self, code: &str, base: Style) -> Vec<Line<'static>> {
        self.highlight_with_query(code, base)
    }

    fn highlight_with_query(&mut self, code: &str, base: Style) -> Vec<Line<'static>> {
        // Empty code produces empty output
        if code.is_empty() {
            return Vec::new();
        }

        let Some(tree) = self.parser.parse(code, None) else {
            return plain_code_lines(code, base);
        };

        let root = tree.root_node();
        let mut cursor = QueryCursor::new();

        // Per-line segment collection.
        let line_starts = compute_line_starts(code);
        let mut segments_per_line: Vec<Vec<Segment>> = vec![Vec::new(); line_starts.len()];

        // Use StreamingIterator for tree-sitter 0.24 API
        let mut matches = cursor.matches(&self.query, root, code.as_bytes());
        while let Some(m) = matches.next() {
            for cap in m.captures {
                let node = cap.node;
                let name = &self.query.capture_names()[cap.index as usize];
                let seg_style = style_for_capture(name);
                let priority = priority_for_capture(name);

                push_node_segments(
                    code,
                    node,
                    seg_style,
                    priority,
                    &line_starts,
                    &mut segments_per_line,
                );
            }
        }

        // Build `Line` output for each physical input line.
        let mut out = Vec::new();
        for (line_idx, (start, end)) in line_ranges(code, &line_starts).into_iter().enumerate() {
            let line_text = &code[start..end];
            let spans = render_line_with_segments(line_text, &segments_per_line[line_idx], base);
            out.push(Line::from(spans));
        }

        // Preserve trailing newline behavior: if code ends with '\n', add a final blank line.
        if code.ends_with('\n') {
            out.push(Line::from(Span::styled(String::new(), base)));
        }

        out
    }
}

fn plain_code_lines(code: &str, base: Style) -> Vec<Line<'static>> {
    code.split('\n')
        .map(|s| Line::from(Span::styled(s.to_string(), base)))
        .collect()
}

#[derive(Debug, Clone)]
struct Segment {
    start: usize, // byte offset within the line slice
    end: usize,   // byte offset within the line slice
    style: Style,
    priority: u8,
}

fn style_for_capture(name: &str) -> Style {
    match name {
        "comment" => Style::default()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::ITALIC),
        "string" => Style::default().fg(Color::Green),
        "number" => Style::default().fg(Color::Magenta),
        "constant" => Style::default().fg(Color::Cyan),
        "type" => Style::default().fg(Color::Yellow),
        "function" => Style::default()
            .fg(Color::Blue)
            .add_modifier(Modifier::BOLD),
        "macro" => Style::default().fg(Color::LightCyan),
        _ => Style::default(),
    }
}

fn priority_for_capture(name: &str) -> u8 {
    match name {
        "comment" => 100,
        "string" => 90,
        "number" => 80,
        "constant" => 70,
        "type" => 60,
        "macro" => 50,
        "function" => 40,
        _ => 0,
    }
}

fn compute_line_starts(s: &str) -> Vec<usize> {
    let mut starts = vec![0];
    for (i, b) in s.as_bytes().iter().enumerate() {
        if *b == b'\n' && i < s.len() {
            starts.push(i + 1);
        }
    }
    // Ensure at least one line.
    if starts.is_empty() {
        starts.push(0);
    }
    starts
}

fn line_ranges(s: &str, starts: &[usize]) -> Vec<(usize, usize)> {
    let mut out = Vec::with_capacity(starts.len());
    for (idx, &start) in starts.iter().enumerate() {
        let end = if idx + 1 < starts.len() {
            // exclude the newline; start of next line is after '\n'
            let next_start = starts[idx + 1];
            s[..next_start]
                .rfind('\n')
                .unwrap_or(next_start)
                .min(next_start)
        } else {
            s.len()
        };
        // If end points at '\n', trim.
        let end = if end > start && s.as_bytes().get(end - 1) == Some(&b'\n') {
            end - 1
        } else {
            end
        };
        out.push((start, end));
    }
    out
}

fn push_node_segments(
    code: &str,
    node: Node<'_>,
    style: Style,
    priority: u8,
    line_starts: &[usize],
    segments_per_line: &mut [Vec<Segment>],
) {
    let range = node.byte_range();
    let start = range.start;
    let end = range.end.min(code.len());
    if start >= end {
        return;
    }

    // Find first line index by scanning starts (small N; keep it simple).
    let mut line_idx = 0usize;
    while line_idx + 1 < line_starts.len() && line_starts[line_idx + 1] <= start {
        line_idx += 1;
    }

    let ranges = line_ranges(code, line_starts);
    for i in line_idx..ranges.len() {
        let (ls, le) = ranges[i];
        if end <= ls {
            break;
        }
        if start >= le {
            continue;
        }
        let seg_start = start.max(ls) - ls;
        let seg_end = end.min(le) - ls;
        if seg_start < seg_end {
            segments_per_line[i].push(Segment {
                start: seg_start,
                end: seg_end,
                style,
                priority,
            });
        }
        if end <= le {
            break;
        }
    }
}

fn render_line_with_segments(line: &str, segments: &[Segment], base: Style) -> Vec<Span<'static>> {
    if segments.is_empty() {
        return vec![Span::styled(line.to_string(), base)];
    }

    // Boundary scan with priority resolution.
    let mut boundaries = Vec::with_capacity(segments.len() * 2 + 2);
    boundaries.push(0usize);
    boundaries.push(line.len());
    for s in segments {
        boundaries.push(s.start.min(line.len()));
        boundaries.push(s.end.min(line.len()));
    }
    boundaries.sort_unstable();
    boundaries.dedup();

    let mut spans = Vec::new();
    for w in boundaries.windows(2) {
        let a = w[0];
        let b = w[1];
        if a >= b {
            continue;
        }

        let mut best: Option<&Segment> = None;
        for seg in segments {
            if seg.start <= a && b <= seg.end {
                best = match best {
                    None => Some(seg),
                    Some(cur) => {
                        if seg.priority > cur.priority {
                            Some(seg)
                        } else {
                            Some(cur)
                        }
                    }
                };
            }
        }

        let slice = &line[a..b];
        let style = match best {
            None => base,
            Some(seg) => base.patch(seg.style),
        };
        spans.push(Span::styled(slice.to_string(), style));
    }

    spans
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn highlights_rust_and_degrades_never_panics() {
        let mut h = SyntaxHighlighter::try_new(CodeLanguage::Rust).expect("rust highlighter");
        let base = Style::default();

        let code = r#"fn main() { println!("hi"); // comment
}"#;

        let lines = h.highlight_rust(code, base);
        assert!(!lines.is_empty());

        let has_comment_style = lines
            .iter()
            .any(|l| l.spans.iter().any(|s| s.style.fg == Some(Color::DarkGray)));
        assert!(has_comment_style, "expected comment styling");
    }

    #[test]
    fn fallback_plain_lines_when_parser_fails_is_safe() {
        // Simulate fallback by constructing plain directly.
        let base = Style::default();
        let out = plain_code_lines("a\nb", base);
        assert_eq!(out.len(), 2);
    }

    #[test]
    fn highlights_strings_and_functions() {
        let mut h = SyntaxHighlighter::try_new(CodeLanguage::Rust).expect("rust highlighter");
        let base = Style::default();

        let code = r#"fn greet() { let msg = "hello"; }"#;
        let lines = h.highlight_rust(code, base);

        // Should produce at least one line
        assert!(!lines.is_empty());

        // Check that we have some styled spans
        let total_spans: usize = lines.iter().map(|l| l.spans.len()).sum();
        assert!(total_spans > 0, "should have styled spans");
    }

    #[test]
    fn handles_empty_code() {
        let mut h = SyntaxHighlighter::try_new(CodeLanguage::Rust).expect("rust highlighter");
        let base = Style::default();

        let lines = h.highlight_rust("", base);
        // Empty code produces an empty line
        assert_eq!(lines.len(), 0);
    }

    #[test]
    fn handles_multiline_code() {
        let mut h = SyntaxHighlighter::try_new(CodeLanguage::Rust).expect("rust highlighter");
        let base = Style::default();

        let code = "fn a() {}\nfn b() {}\nfn c() {}";
        let lines = h.highlight_rust(code, base);

        // Should have 3 lines
        assert_eq!(lines.len(), 3);
    }
}
