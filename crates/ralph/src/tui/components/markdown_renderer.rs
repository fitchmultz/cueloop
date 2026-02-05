//! Markdown rendering for Ralph TUI with optional Tree-sitter Rust highlighting.
//!
//! Responsibilities:
//! - Parse a minimal Markdown subset (headings, lists, emphasis, inline code, fenced code blocks).
//! - Produce line-oriented `ratatui::text::{Line, Span}` output suitable for scrollable views.
//! - Highlight fenced Rust code blocks via Tree-sitter when available.
//! - Provide a `Component` wrapper for future/overlay usage consistent with the foundation layer.
//!
//! Not handled here:
//! - Full CommonMark support (tables, HTML blocks, images, footnotes, etc.).
//! - Horizontal scrolling; long lines must wrap to width.
//! - Multi-language highlighting (only Rust fences are highlighted).
//!
//! Invariants/assumptions:
//! - Rendering must be deterministic and panic-free on malformed Markdown.
//! - Output is pre-wrapped to the provided width so scroll offsets remain line-based.

use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use super::syntax_highlighter::{CodeLanguage, SyntaxHighlighter};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MarkdownRenderConfig {
    pub(crate) width: usize,
    pub(crate) enable_syntax_highlighting: bool,
}

impl MarkdownRenderConfig {
    pub(crate) fn new(width: usize) -> Self {
        Self {
            width: width.max(1),
            enable_syntax_highlighting: true,
        }
    }
}

pub(crate) struct MarkdownRenderer;

impl MarkdownRenderer {
    pub(crate) fn render(markdown: &str, cfg: MarkdownRenderConfig) -> Vec<Line<'static>> {
        let mut opts = Options::empty();
        opts.insert(Options::ENABLE_STRIKETHROUGH);
        opts.insert(Options::ENABLE_TASKLISTS);

        let parser = Parser::new_ext(markdown, opts);

        let mut out: Vec<Line<'static>> = Vec::new();
        let mut spans: Vec<Span<'static>> = Vec::new();

        let mut style_stack: Vec<Style> = vec![Style::default()];
        let mut in_heading: Option<u8> = None;

        let mut list_stack: Vec<ListCtx> = Vec::new();

        let mut in_code_block: Option<CodeFence> = None;

        for ev in parser {
            match ev {
                Event::Start(tag) => match tag {
                    Tag::Paragraph => {}
                    Tag::Heading { level, .. } => {
                        in_heading = Some(level as u8);
                    }
                    Tag::Emphasis => push_style(&mut style_stack, Modifier::ITALIC),
                    Tag::Strong => push_style(&mut style_stack, Modifier::BOLD),
                    Tag::List(start) => {
                        list_stack.push(ListCtx::new(start));
                    }
                    Tag::Item => {
                        spans.clear();
                    }
                    Tag::CodeBlock(kind) => {
                        in_code_block = Some(CodeFence::new(kind));
                    }
                    _ => {}
                },
                Event::End(end) => match end {
                    TagEnd::Paragraph => {
                        flush_wrapped_paragraph(&mut out, &mut spans, cfg.width);
                    }
                    TagEnd::Heading(_) => {
                        let level = in_heading.take().unwrap_or(1);
                        let heading_style = heading_style(level);
                        for s in spans.iter_mut() {
                            s.style = heading_style.patch(s.style);
                        }
                        flush_wrapped_paragraph(&mut out, &mut spans, cfg.width);
                        out.push(Line::from(""));
                    }
                    TagEnd::Emphasis => pop_style(&mut style_stack),
                    TagEnd::Strong => pop_style(&mut style_stack),
                    TagEnd::Item => {
                        let prefix = list_prefix(&mut list_stack);
                        flush_wrapped_list_item(&mut out, &mut spans, cfg.width, &prefix);
                    }
                    TagEnd::List(_) => {
                        list_stack.pop();
                        out.push(Line::from(""));
                    }
                    TagEnd::CodeBlock => {
                        if let Some(fence) = in_code_block.take() {
                            render_code_block(&mut out, fence, cfg);
                            out.push(Line::from(""));
                        }
                    }
                    _ => {}
                },
                Event::Text(t) => {
                    if let Some(fence) = in_code_block.as_mut() {
                        fence.code.push_str(&t);
                    } else {
                        push_text(&mut spans, t.to_string(), *style_stack.last().unwrap());
                    }
                }
                Event::Code(t) => {
                    if let Some(fence) = in_code_block.as_mut() {
                        fence.code.push_str(&t);
                    } else {
                        let code_style = inline_code_style();
                        push_text(&mut spans, t.to_string(), code_style);
                    }
                }
                Event::SoftBreak => {
                    if in_code_block.is_some() {
                        if let Some(fence) = in_code_block.as_mut() {
                            fence.code.push('\n');
                        }
                    } else {
                        push_text(&mut spans, " ".to_string(), *style_stack.last().unwrap());
                    }
                }
                Event::HardBreak => {
                    if in_code_block.is_some() {
                        if let Some(fence) = in_code_block.as_mut() {
                            fence.code.push('\n');
                        }
                    } else {
                        flush_wrapped_paragraph(&mut out, &mut spans, cfg.width);
                    }
                }
                _ => {}
            }
        }

        // Final flush safety.
        flush_wrapped_paragraph(&mut out, &mut spans, cfg.width);

        // Trim trailing blanks.
        while matches!(out.last(), Some(l) if l.spans.is_empty() || (l.spans.len() == 1 && l.spans[0].content.is_empty()))
        {
            out.pop();
        }

        out
    }
}

#[derive(Debug)]
struct ListCtx {
    ordered: bool,
    next_index: usize,
}
impl ListCtx {
    fn new(start: Option<u64>) -> Self {
        match start {
            None => Self {
                ordered: false,
                next_index: 1,
            },
            Some(s) => Self {
                ordered: true,
                next_index: s as usize,
            },
        }
    }
}

fn list_prefix(stack: &mut [ListCtx]) -> String {
    let Some(ctx) = stack.last_mut() else {
        return "• ".to_string();
    };
    if ctx.ordered {
        let p = format!("{}. ", ctx.next_index);
        ctx.next_index += 1;
        p
    } else {
        "• ".to_string()
    }
}

#[derive(Debug)]
struct CodeFence {
    lang: Option<String>,
    code: String,
}
impl CodeFence {
    fn new(kind: CodeBlockKind<'_>) -> Self {
        let lang = match kind {
            CodeBlockKind::Indented => None,
            CodeBlockKind::Fenced(info) => {
                let s = info.trim();
                if s.is_empty() {
                    None
                } else {
                    Some(s.to_string())
                }
            }
        };
        Self {
            lang,
            code: String::new(),
        }
    }
}

fn heading_style(level: u8) -> Style {
    match level {
        1 => Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
        2 => Style::default()
            .fg(Color::LightCyan)
            .add_modifier(Modifier::BOLD),
        _ => Style::default().add_modifier(Modifier::BOLD),
    }
}

fn inline_code_style() -> Style {
    Style::default()
        .fg(Color::Yellow)
        .bg(Color::Black)
        .add_modifier(Modifier::BOLD)
}

fn code_block_base_style() -> Style {
    Style::default().fg(Color::White).bg(Color::Black)
}

fn push_style(stack: &mut Vec<Style>, add: Modifier) {
    let cur = *stack.last().unwrap_or(&Style::default());
    stack.push(cur.add_modifier(add));
}

fn pop_style(stack: &mut Vec<Style>) {
    if stack.len() > 1 {
        stack.pop();
    }
}

fn push_text(spans: &mut Vec<Span<'static>>, text: String, style: Style) {
    if text.is_empty() {
        return;
    }
    spans.push(Span::styled(text, style));
}

fn flush_wrapped_paragraph(
    out: &mut Vec<Line<'static>>,
    spans: &mut Vec<Span<'static>>,
    width: usize,
) {
    if spans.is_empty() {
        return;
    }
    let wrapped = wrap_spans(spans, width, "", "");
    out.extend(wrapped);
    spans.clear();
}

fn flush_wrapped_list_item(
    out: &mut Vec<Line<'static>>,
    spans: &mut Vec<Span<'static>>,
    width: usize,
    prefix: &str,
) {
    let indent = " ".repeat(prefix.chars().count());
    let wrapped = wrap_spans(spans, width, prefix, &indent);
    out.extend(wrapped);
    spans.clear();
}

fn render_code_block(out: &mut Vec<Line<'static>>, fence: CodeFence, cfg: MarkdownRenderConfig) {
    let base = code_block_base_style();
    let gutter = Span::styled("│ ", Style::default().fg(Color::DarkGray).bg(Color::Black));

    let lang = fence.lang.as_deref().map(normalize_lang);

    let mut lines = if cfg.enable_syntax_highlighting && matches!(lang.as_deref(), Some("rust")) {
        // Lazy-init, degrade to plain if unavailable.
        thread_local! {
            static RUST: std::cell::RefCell<Option<SyntaxHighlighter>> = const { std::cell::RefCell::new(None) };
        }

        RUST.with(|cell| {
            let mut slot = cell.borrow_mut();
            if slot.is_none() {
                *slot = SyntaxHighlighter::try_new(CodeLanguage::Rust);
            }
            if let Some(h) = slot.as_mut() {
                h.highlight_rust(&fence.code, base)
            } else {
                fence
                    .code
                    .split('\n')
                    .map(|s| Line::from(Span::styled(s.to_string(), base)))
                    .collect()
            }
        })
    } else {
        fence
            .code
            .split('\n')
            .map(|s| Line::from(Span::styled(s.to_string(), base)))
            .collect()
    };

    // Wrap code lines to width (line-based scrolling invariant).
    let width = cfg.width.max(1);
    let mut wrapped_lines: Vec<Line<'static>> = Vec::new();
    for line in lines.drain(..) {
        let wrapped = wrap_spans(&line.spans, width.saturating_sub(2), "", "");
        wrapped_lines.extend(wrapped);
    }

    for mut line in wrapped_lines {
        line.spans.insert(0, gutter.clone());
        out.push(line);
    }
}

fn normalize_lang(info: &str) -> String {
    info.split(|c: char| c.is_whitespace() || c == ',' || c == ';')
        .next()
        .unwrap_or("")
        .trim()
        .to_lowercase()
}

fn wrap_spans(
    spans: &[Span<'static>],
    width: usize,
    first_prefix: &str,
    next_prefix: &str,
) -> Vec<Line<'static>> {
    let width = width.max(1);

    let first_prefix_w = first_prefix.chars().count();
    let next_prefix_w = next_prefix.chars().count();

    let mut tokens = tokenize_spans(spans);

    let mut out: Vec<Line<'static>> = Vec::new();
    let mut cur: Vec<Span<'static>> = Vec::new();
    let mut cur_w = 0usize;

    let mut prefix = first_prefix.to_string();
    let mut prefix_w = first_prefix_w;

    // Helper to start a new line with prefix.
    let start_line = |cur: &mut Vec<Span<'static>>, cur_w: &mut usize, p: &str, pw: usize| {
        cur.clear();
        *cur_w = 0;
        if !p.is_empty() {
            cur.push(Span::raw(p.to_string()));
            *cur_w = pw;
        }
    };

    start_line(&mut cur, &mut cur_w, &prefix, prefix_w);

    while let Some(tok) = tokens.first().cloned() {
        tokens.remove(0);

        let tok_w = tok.text.chars().count();

        // Drop leading whitespace after a wrap.
        if cur_w == prefix_w && tok.text.chars().all(|c| c.is_whitespace()) {
            continue;
        }

        if cur_w + tok_w <= width {
            cur.push(Span::styled(tok.text, tok.style));
            cur_w += tok_w;
            continue;
        }

        // If token doesn't fit, flush current line.
        if cur_w > prefix_w {
            out.push(Line::from(cur.clone()));
            prefix = next_prefix.to_string();
            prefix_w = next_prefix_w;
            start_line(&mut cur, &mut cur_w, &prefix, prefix_w);
            // Re-process the token on the next line.
            tokens.insert(0, tok);
            continue;
        }

        // Hard split a too-wide token.
        let mut remaining = tok.text;
        while !remaining.is_empty() {
            let room = width.saturating_sub(cur_w).max(1);
            let (head, tail) = split_at_char(remaining.as_str(), room);
            cur.push(Span::styled(head.to_string(), tok.style));
            out.push(Line::from(cur.clone()));

            prefix = next_prefix.to_string();
            prefix_w = next_prefix_w;
            start_line(&mut cur, &mut cur_w, &prefix, prefix_w);

            remaining = tail.to_string();
        }
    }

    if cur_w > 0 {
        out.push(Line::from(cur));
    }

    out
}

#[derive(Debug, Clone)]
struct Tok {
    text: String,
    style: Style,
}

fn tokenize_spans(spans: &[Span<'static>]) -> Vec<Tok> {
    let mut out = Vec::new();
    for sp in spans {
        let s = sp.content.as_ref();
        for part in split_whitespace_runs(s) {
            out.push(Tok {
                text: part.to_string(),
                style: sp.style,
            });
        }
    }
    out
}

fn split_whitespace_runs(s: &str) -> Vec<&str> {
    if s.is_empty() {
        return Vec::new();
    }
    let mut parts = Vec::new();
    let mut start = 0usize;
    let mut last_is_ws = s.chars().next().unwrap().is_whitespace();
    for (i, ch) in s.char_indices() {
        let is_ws = ch.is_whitespace();
        if is_ws != last_is_ws {
            parts.push(&s[start..i]);
            start = i;
            last_is_ws = is_ws;
        }
    }
    parts.push(&s[start..]);
    parts
}

fn split_at_char(s: &str, n: usize) -> (&str, &str) {
    if n == 0 {
        return ("", s);
    }
    for (count, (i, _)) in s.char_indices().enumerate() {
        if count == n {
            return (&s[..i], &s[i..]);
        }
    }
    (s, "")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_headings_lists_inline_code_and_fences() {
        let md = r#"
# Title

Some *emphasis* and `inline`.

- item one
- item two

```rust
fn main() { println!("hi"); }
```
"#;

        let cfg = MarkdownRenderConfig {
            width: 40,
            enable_syntax_highlighting: true,
        };
        let lines = MarkdownRenderer::render(md, cfg);
        assert!(!lines.is_empty());

        // Must include gutter for code block.
        let has_gutter = lines
            .iter()
            .any(|l| l.spans.iter().any(|s| s.content.as_ref().contains('│')));
        assert!(has_gutter);
    }

    #[test]
    fn does_not_panic_on_malformed_markdown() {
        let md = "```rust\nfn main() {}\n";
        let cfg = MarkdownRenderConfig {
            width: 20,
            enable_syntax_highlighting: true,
        };
        let _ = MarkdownRenderer::render(md, cfg);
    }

    #[test]
    fn can_disable_syntax_highlighting_for_degradation() {
        let md = "```rust\nfn main() {}\n```";
        let cfg = MarkdownRenderConfig {
            width: 30,
            enable_syntax_highlighting: false,
        };
        let lines = MarkdownRenderer::render(md, cfg);
        assert!(!lines.is_empty());

        // Should still have gutter even without highlighting
        let has_gutter = lines
            .iter()
            .any(|l| l.spans.iter().any(|s| s.content.as_ref().contains('│')));
        assert!(has_gutter);
    }

    #[test]
    fn renders_basic_markdown_elements() {
        let md = "# Heading\n\nSome **bold** and *italic* text.";
        let cfg = MarkdownRenderConfig::new(80);
        let lines = MarkdownRenderer::render(md, cfg);

        assert!(!lines.is_empty());
        // Should have heading and content
        assert!(lines.len() >= 2);
    }

    #[test]
    fn renders_ordered_lists() {
        let md = "1. First\n2. Second\n3. Third";
        let cfg = MarkdownRenderConfig::new(40);
        let lines = MarkdownRenderer::render(md, cfg);

        assert!(!lines.is_empty());
        // Should have 3 list items
        assert!(lines.len() >= 3);
    }

    #[test]
    fn handles_inline_code() {
        let md = "Use `cargo build` to compile.";
        let cfg = MarkdownRenderConfig::new(80);
        let lines = MarkdownRenderer::render(md, cfg);

        assert!(!lines.is_empty());
        // Line should have content
        assert!(!lines[0].spans.is_empty());
    }

    #[test]
    fn wraps_long_lines() {
        let md = "This is a very long line that should be wrapped to multiple lines when rendered with a narrow width.";
        let cfg = MarkdownRenderConfig::new(20);
        let lines = MarkdownRenderer::render(md, cfg);

        // Should produce multiple lines due to wrapping
        assert!(lines.len() > 1);
    }

    #[test]
    fn handles_empty_markdown() {
        let cfg = MarkdownRenderConfig::new(80);
        let lines = MarkdownRenderer::render("", cfg);
        // Empty markdown should produce empty output
        assert!(lines.is_empty());
    }
}
