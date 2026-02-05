//! Unified diff viewer component for the Ralph TUI.
//!
//! Responsibilities:
//! - Compute a unified, line-oriented diff between an "old" and "new" text input using `similar`.
//! - Render diff lines with required prefixes (`-`, `+`, ` `) and stable color semantics:
//!   deletions red, insertions green, unchanged gray.
//! - Integrate with `ScrollableContainerComponent` for scrolling, focus, and mouse wheel behavior.
//! - Gracefully handle empty inputs, very large diffs, and narrow terminal widths without panicking.
//!
//! Not handled here:
//! - Inline/word-level highlighting within a changed line (this is a line-based viewer by design).
//! - File headers, timestamps, or patch metadata (callers can render those separately if needed).
//! - Horizontal scrolling (long lines are wrapped to the viewport width).
//!
//! Invariants/assumptions:
//! - Inputs are treated as text; line endings are normalized to `\n` before diffing.
//! - Diff rendering is deterministic and panic-free for any UTF-8 input.
//! - Scrolling is vertical only and delegated to `ScrollableContainerComponent`.

use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders},
};
use similar::{ChangeTag, TextDiff};

use crate::tui::{
    App,
    foundation::{Component, ComponentId, FocusManager, RenderCtx, UiEvent},
};

use super::scroll_container::{ScrollableContainerComponent, ScrollableContainerMessage};

/// Messages produced by `DiffViewerComponent`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DiffViewerMessage {
    /// Scroll position changed.
    Scrolled { offset: usize },
    /// No externally relevant action.
    Noop,
}

/// Internal representation of a single diff line.
#[derive(Debug, Clone, PartialEq, Eq)]
struct DiffOpLine {
    tag: ChangeTag,
    text: String, // single logical line, no trailing '\n'
}

/// Customizable styles for the diff viewer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DiffViewerStyle {
    /// Style for deleted lines.
    pub(crate) delete: Style,
    /// Style for inserted lines.
    pub(crate) insert: Style,
    /// Style for unchanged lines.
    pub(crate) equal: Style,
    /// Style for the prefix character (-, +, space).
    pub(crate) prefix: Style,
}

impl Default for DiffViewerStyle {
    fn default() -> Self {
        Self {
            delete: Style::default().fg(Color::Red),
            insert: Style::default().fg(Color::Green),
            equal: Style::default().fg(Color::DarkGray),
            prefix: Style::default().add_modifier(Modifier::BOLD),
        }
    }
}

/// Unified diff viewer component that renders old→new text differences.
pub(crate) struct DiffViewerComponent {
    scroll: ScrollableContainerComponent,

    old_text: String,
    new_text: String,

    style: DiffViewerStyle,

    // Diff cache (independent of width).
    diff_lines: Vec<DiffOpLine>,
    diff_dirty: bool,

    // Wrap cache key.
    last_content_width: u16,
    wrapped_dirty: bool,
}

impl DiffViewerComponent {
    /// Create a new diff viewer component.
    pub(crate) fn new() -> Self {
        let mut scroll = ScrollableContainerComponent::new(ComponentId::new("diff_viewer", 0), 0);
        scroll.set_sticky(false);
        scroll.set_title("Diff");

        Self {
            scroll,
            old_text: String::new(),
            new_text: String::new(),
            style: DiffViewerStyle::default(),
            diff_lines: Vec::new(),
            diff_dirty: true,
            last_content_width: 0,
            wrapped_dirty: true,
        }
    }

    /// Set the title shown in the border.
    pub(crate) fn set_title(&mut self, title: impl Into<String>) {
        self.scroll.set_title(title);
    }

    /// Set custom styles for the diff viewer.
    pub(crate) fn set_style(&mut self, style: DiffViewerStyle) {
        self.style = style;
        self.wrapped_dirty = true;
    }

    /// Set the old and new text inputs to diff.
    pub(crate) fn set_inputs(&mut self, old_text: impl Into<String>, new_text: impl Into<String>) {
        self.old_text = old_text.into();
        self.new_text = new_text.into();
        self.diff_dirty = true;
        self.wrapped_dirty = true;
    }

    /// Normalize line endings to `\n` for consistent diffing.
    fn normalize_newlines(s: &str) -> String {
        s.replace("\r\n", "\n").replace('\r', "\n")
    }

    /// Compute the diff lines from old and new text.
    fn compute_diff_lines(old_text: &str, new_text: &str) -> Vec<DiffOpLine> {
        let old_norm = Self::normalize_newlines(old_text);
        let new_norm = Self::normalize_newlines(new_text);

        // Fast-path: exact equality (including both empty) => no changes.
        if old_norm == new_norm {
            return Vec::new();
        }

        let diff = TextDiff::from_lines(&old_norm, &new_norm);

        let mut out: Vec<DiffOpLine> = Vec::new();
        for ch in diff.iter_all_changes() {
            let tag = ch.tag();
            let mut v = ch.value();

            // `from_lines` yields values that typically include trailing '\n'.
            if v.ends_with('\n') {
                v = &v[..v.len() - 1];
            }

            out.push(DiffOpLine {
                tag,
                text: v.to_string(),
            });
        }
        out
    }

    /// Get the prefix character for a change tag.
    fn prefix_for(tag: ChangeTag) -> char {
        match tag {
            ChangeTag::Delete => '-',
            ChangeTag::Insert => '+',
            ChangeTag::Equal => ' ',
        }
    }

    /// Get the style for a change tag.
    fn style_for(&self, tag: ChangeTag) -> Style {
        match tag {
            ChangeTag::Delete => self.style.delete,
            ChangeTag::Insert => self.style.insert,
            ChangeTag::Equal => self.style.equal,
        }
    }

    /// Calculate the content width available for text given an area.
    fn content_width_for_area(area: Rect) -> u16 {
        // Mirror ScrollableContainer's geometry: borders + optional scrollbar column.
        let block = Block::default().borders(Borders::ALL);
        let inner = block.inner(area);

        let inner_content_width = if inner.width >= 2 {
            inner.width.saturating_sub(1) // reserve scrollbar
        } else {
            inner.width
        };

        // Our rendered line format is: "<prefix><space><content>"
        // So content area loses 2 columns if possible.
        inner_content_width.saturating_sub(2)
    }

    /// Wrap a string to fit within the given width.
    fn wrap_to_width(s: &str, width: usize) -> Vec<String> {
        let width = width.max(1);
        if s.is_empty() {
            return vec![String::new()];
        }

        let mut out = Vec::new();
        let mut cur = String::new();
        let mut cur_len = 0usize;

        for ch in s.chars() {
            if cur_len >= width {
                out.push(cur);
                cur = String::new();
                cur_len = 0;
            }
            cur.push(ch);
            cur_len += 1;
        }
        out.push(cur);
        out
    }

    /// Render the diff lines wrapped to the given content width.
    fn render_wrapped_lines(&self, content_width: u16) -> Vec<Line<'static>> {
        if self.diff_lines.is_empty() {
            return vec![Line::from(Span::styled(
                "(no changes)",
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::ITALIC),
            ))];
        }

        let w = content_width.max(1) as usize;
        let mut out: Vec<Line<'static>> = Vec::new();

        for op in &self.diff_lines {
            let prefix = Self::prefix_for(op.tag);
            let base_style = self.style_for(op.tag);
            let prefix_style = base_style.patch(self.style.prefix);

            let wrapped = Self::wrap_to_width(&op.text, w);

            for (i, part) in wrapped.into_iter().enumerate() {
                let (pfx, pfx_style) = if i == 0 {
                    (prefix, prefix_style)
                } else {
                    (' ', base_style) // continuation line: no +/- marker
                };

                // Two spans keeps prefix styling separate and predictable.
                let line = Line::from(vec![
                    Span::styled(format!("{pfx} "), pfx_style),
                    Span::styled(part, base_style),
                ]);
                out.push(line);
            }
        }

        out
    }

    /// Ensure cached diff and wrapped lines are up to date.
    fn ensure_cached(&mut self, content_width: u16) {
        if self.diff_dirty {
            self.diff_lines = Self::compute_diff_lines(&self.old_text, &self.new_text);
            self.diff_dirty = false;
            self.wrapped_dirty = true;
        }

        if self.wrapped_dirty || self.last_content_width != content_width {
            self.last_content_width = content_width;
            let rendered = self.render_wrapped_lines(content_width);
            self.scroll.set_lines(rendered);
            self.wrapped_dirty = false;
        }
    }
}

impl Component for DiffViewerComponent {
    type Message = DiffViewerMessage;

    fn render(&mut self, f: &mut Frame<'_>, area: Rect, app: &App, ctx: &mut RenderCtx<'_>) {
        // Compute wrapping width from the same geometry assumptions as the scroll container.
        let content_width = Self::content_width_for_area(area);
        self.ensure_cached(content_width);

        // Delegate all rendering + focus registration to the scroll container.
        self.scroll.render(f, area, app, ctx);
    }

    fn handle_event(
        &mut self,
        event: &UiEvent,
        app: &App,
        focus: &mut FocusManager,
    ) -> Option<Self::Message> {
        match self.scroll.handle_event(event, app, focus) {
            None => None,
            Some(ScrollableContainerMessage::Scrolled { offset, .. }) => {
                Some(DiffViewerMessage::Scrolled { offset })
            }
            Some(ScrollableContainerMessage::StickyChanged { .. }) => Some(DiffViewerMessage::Noop),
            Some(ScrollableContainerMessage::Noop) => Some(DiffViewerMessage::Noop),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::Color;

    #[test]
    fn computes_empty_when_equal_including_empty() {
        let out = DiffViewerComponent::compute_diff_lines("", "");
        assert!(out.is_empty());

        let out2 = DiffViewerComponent::compute_diff_lines("a\nb\n", "a\nb\n");
        assert!(out2.is_empty());
    }

    #[test]
    fn computes_insert_delete_and_equal_tags() {
        let out = DiffViewerComponent::compute_diff_lines("a\nb\n", "a\nc\n");
        assert!(!out.is_empty());
        assert!(out.iter().any(|l| l.tag == ChangeTag::Delete));
        assert!(out.iter().any(|l| l.tag == ChangeTag::Insert));
        assert!(out.iter().any(|l| l.tag == ChangeTag::Equal));
    }

    #[test]
    fn wraps_long_lines_without_panicking_on_narrow_widths() {
        let mut c = DiffViewerComponent::new();
        c.set_inputs("", "this-is-a-very-long-line");
        c.diff_lines = vec![DiffOpLine {
            tag: ChangeTag::Insert,
            text: "this-is-a-very-long-line".to_string(),
        }];

        let rendered = c.render_wrapped_lines(1);
        assert!(!rendered.is_empty());
    }

    #[test]
    fn renders_prefix_and_colors() {
        let mut c = DiffViewerComponent::new();
        c.diff_lines = vec![
            DiffOpLine {
                tag: ChangeTag::Delete,
                text: "gone".into(),
            },
            DiffOpLine {
                tag: ChangeTag::Insert,
                text: "new".into(),
            },
            DiffOpLine {
                tag: ChangeTag::Equal,
                text: "same".into(),
            },
        ];

        let rendered = c.render_wrapped_lines(80);
        assert!(rendered[0].spans[0].content.starts_with("- "));
        assert!(rendered[1].spans[0].content.starts_with("+ "));
        assert!(rendered[2].spans[0].content.starts_with("  "));

        assert_eq!(rendered[0].spans[1].style.fg, Some(Color::Red));
        assert_eq!(rendered[1].spans[1].style.fg, Some(Color::Green));
        assert_eq!(rendered[2].spans[1].style.fg, Some(Color::DarkGray));
    }

    #[test]
    fn prefix_chars_match_tags() {
        assert_eq!(DiffViewerComponent::prefix_for(ChangeTag::Delete), '-');
        assert_eq!(DiffViewerComponent::prefix_for(ChangeTag::Insert), '+');
        assert_eq!(DiffViewerComponent::prefix_for(ChangeTag::Equal), ' ');
    }

    #[test]
    fn normalize_newlines_handles_all_types() {
        assert_eq!(
            DiffViewerComponent::normalize_newlines("a\r\nb\r\nc"),
            "a\nb\nc"
        );
        assert_eq!(
            DiffViewerComponent::normalize_newlines("a\rb\rc"),
            "a\nb\nc"
        );
        assert_eq!(
            DiffViewerComponent::normalize_newlines("a\nb\nc"),
            "a\nb\nc"
        );
    }

    #[test]
    fn wrap_to_width_splits_correctly() {
        let result = DiffViewerComponent::wrap_to_width("hello world", 5);
        assert_eq!(result.len(), 3);
        assert_eq!(result[0], "hello");
        assert_eq!(result[1], " worl");
        assert_eq!(result[2], "d");
    }

    #[test]
    fn wrap_to_width_handles_empty() {
        let result = DiffViewerComponent::wrap_to_width("", 10);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], "");
    }

    #[test]
    fn content_width_calculation_reserves_space() {
        // A 20x10 area:
        // - Borders take 2 cols (left+right) = 18 inner
        // - Scrollbar takes 1 col = 17 inner_content
        // - Prefix+space takes 2 cols = 15 content
        let area = Rect::new(0, 0, 20, 10);
        let width = DiffViewerComponent::content_width_for_area(area);
        assert_eq!(width, 15);
    }

    #[test]
    fn ensures_cached_updates_on_input_change() {
        let mut c = DiffViewerComponent::new();
        c.set_inputs("old", "new");
        assert!(c.diff_dirty);

        c.ensure_cached(80);
        assert!(!c.diff_dirty);
        assert!(!c.wrapped_dirty);
    }
}
