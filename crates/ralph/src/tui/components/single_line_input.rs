//! Single-line rich text input component for the Ralph TUI.
//!
//! Responsibilities:
//! - Provide a reusable, focus-aware single-line text editor implementing the foundation `Component` trait.
//! - Support cursor movement (Left/Right/Home/End), deletion (Backspace/Delete), and word deletion (Ctrl+W, Ctrl+Backspace).
//! - Support placeholder rendering when empty.
//! - Safely handle paste-like multi-character insertion via `UiEvent::Paste`.
//!
//! Not handled here:
//! - Global focus traversal (Tab/Shift+Tab) policy; parent containers decide when to call `focus_next/prev`.
//! - Input validation, masking (password), history, or completion; those are higher-level concerns.
//! - Grapheme-cluster editing; this component operates on Unicode scalar values (chars), matching existing `TextInput` semantics.
//!
//! Invariants/assumptions:
//! - Editing cursor is a character index in `0..=value.chars().count()`.
//! - Newlines are not permitted; pasted text is normalized to a single line by removing `\r`/\n`.
//! - The component only mutates its state when it is focused and enabled.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};
use tui_input::Input;
use unicode_width::UnicodeWidthChar;

use crate::tui::{
    App,
    foundation::{Component, ComponentId, FocusId, FocusManager, RenderCtx, UiEvent},
    input::{TextInput, apply_text_input_key},
};

/// Messages produced by `SingleLineInputComponent`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SingleLineInputMessage {
    /// Value changed (new value).
    Changed(String),
    /// User pressed Enter to submit.
    Submit(String),
    /// User pressed Esc to cancel.
    Cancel,
}

/// Focus-aware single-line editor.
pub(crate) struct SingleLineInputComponent {
    id: ComponentId,
    focus_local: u16,
    enabled: bool,

    // Canonical single-line state container per OpenTUI recommendation.
    // We adapt its value/cursor to our established editing semantics (TextInput) for keybindings.
    input: Input,

    placeholder: Option<String>,
    last_area: Option<Rect>,
}

impl SingleLineInputComponent {
    /// Create a new single-line input component.
    pub(crate) fn new(id: ComponentId, focus_local: u16, value: impl Into<String>) -> Self {
        let value = value.into();
        Self {
            id,
            focus_local,
            enabled: true,
            input: Input::new(value),
            placeholder: None,
            last_area: None,
        }
    }

    /// Set whether the component is enabled (can receive focus and input).
    pub(crate) fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Set placeholder text shown when input is empty.
    pub(crate) fn set_placeholder(&mut self, placeholder: impl Into<String>) {
        self.placeholder = Some(placeholder.into());
    }

    /// Get the current input value.
    pub(crate) fn value(&self) -> &str {
        self.input.value()
    }

    /// Get the focus ID for this component.
    pub(crate) fn focus_id(&self) -> FocusId {
        FocusId::new(self.id, self.focus_local)
    }

    /// Check if this component is currently focused.
    fn is_focused(&self, focus: &FocusManager) -> bool {
        focus.is_focused(self.focus_id())
    }

    /// Apply a key event to the input, returning a message if value changed.
    fn apply_key(&mut self, key: &KeyEvent) -> Option<SingleLineInputMessage> {
        let before = self.input.value().to_string();

        // Adapt to existing semantics in `tui/input.rs` for consistent behavior.
        let mut model = TextInput::from_parts(self.input.value().to_string(), self.input.cursor());
        let _edit = apply_text_input_key(&mut model, key);

        // Re-sync canonical state using builder pattern.
        let cursor = model.cursor();
        self.input = Input::new(model.into_value()).with_cursor(cursor);

        let after = self.input.value().to_string();
        if after != before {
            Some(SingleLineInputMessage::Changed(after))
        } else {
            None
        }
    }

    /// Apply paste text, normalizing newlines to single line.
    fn apply_paste(&mut self, text: &str) -> Option<SingleLineInputMessage> {
        let before = self.input.value().to_string();

        // Normalize to a single line: remove CR/LF.
        let normalized: String = text.chars().filter(|c| *c != '\n' && *c != '\r').collect();
        if normalized.is_empty() {
            return None;
        }

        // Insert at current cursor by adapting through TextInput.
        let mut model = TextInput::from_parts(self.input.value().to_string(), self.input.cursor());
        for ch in normalized.chars() {
            model.insert_char(ch);
        }

        let cursor = model.cursor();
        self.input = Input::new(model.into_value()).with_cursor(cursor);

        let after = self.input.value().to_string();
        if after != before {
            Some(SingleLineInputMessage::Changed(after))
        } else {
            None
        }
    }

    /// Render the input line with inline cursor marker.
    ///
    /// Returns (display_string, cursor_display_column).
    fn render_line_and_cursor(&self, width: u16, focused: bool) -> (String, u16) {
        // Render with an inline cursor marker (do not rely on terminal cursor).
        // This avoids any need for global cursor positioning and matches textarea-style cursor highlighting.
        let marker = if focused { '▏' } else { ' ' };

        let model = TextInput::from_parts(self.input.value().to_string(), self.input.cursor());
        let full = model.with_cursor_marker(marker);

        // Simple horizontal viewport: ensure the marker is visible in `width`.
        if width == 0 {
            return (String::new(), 0);
        }
        let max_cols = width as usize;

        // Compute display column of each char and find cursor marker column.
        let mut cols = 0usize;
        let mut cursor_col = 0usize;
        let mut rendered: Vec<(char, usize)> = Vec::new();
        for ch in full.chars() {
            let w = UnicodeWidthChar::width(ch).unwrap_or(0).max(1);
            if ch == marker {
                cursor_col = cols;
            }
            rendered.push((ch, w));
            cols = cols.saturating_add(w);
        }

        let start_col = cursor_col.saturating_sub(max_cols.saturating_sub(1));
        let mut out = String::new();
        let mut col = 0usize;
        let mut out_cursor_x = 0u16;

        for (ch, w) in rendered {
            let next = col.saturating_add(w);
            if next <= start_col {
                col = next;
                continue;
            }
            if (out.chars().count() as u16) >= width {
                break;
            }
            if col <= cursor_col && cursor_col < next {
                out_cursor_x = (out.chars().count() as u16).min(width.saturating_sub(1));
            }
            out.push(ch);
            col = next;
            if out.chars().count() >= max_cols {
                break;
            }
        }

        (out, out_cursor_x)
    }
}

impl Component for SingleLineInputComponent {
    type Message = SingleLineInputMessage;

    fn render(&mut self, f: &mut Frame<'_>, area: Rect, _app: &App, ctx: &mut RenderCtx<'_>) {
        self.last_area = Some(area);

        let block_style = Style::default().fg(Color::DarkGray);
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(block_style);

        let inner = block.inner(area);
        f.render_widget(block, area);

        // Register focus for the full area (including borders) to keep click/focus UX forgiving.
        ctx.register_focus(self.focus_id(), area, self.enabled);

        // Placeholder is rendered only when empty; cursor marker visibility is handled by focus state in events.
        let is_empty = self.input.value().is_empty();
        let placeholder = self.placeholder.as_deref().unwrap_or("");

        let is_focused = ctx.focus.nodes().iter().any(|n| n.id == self.focus_id());

        let (line, _cursor_x) = if is_empty && !placeholder.is_empty() {
            // Render placeholder with no cursor marker.
            (placeholder.to_string(), 0)
        } else {
            // Render using inline cursor marker; width is inner.width.
            self.render_line_and_cursor(inner.width, is_focused)
        };

        let text_style = if is_empty && !placeholder.is_empty() {
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::ITALIC)
        } else {
            Style::default().fg(Color::White)
        };

        let p = Paragraph::new(Line::from(Span::styled(line, text_style)));
        f.render_widget(p, inner);
    }

    fn handle_event(
        &mut self,
        event: &UiEvent,
        _app: &App,
        focus: &mut FocusManager,
    ) -> Option<Self::Message> {
        if !self.enabled {
            return None;
        }

        // Mouse click-to-focus (local, based on last render rect).
        if event.is_left_click()
            && let (Some((x, y)), Some(area)) = (event.mouse_position(), self.last_area)
        {
            let inside = x >= area.x
                && x < area.x.saturating_add(area.width)
                && y >= area.y
                && y < area.y.saturating_add(area.height);
            if inside {
                focus.focus(self.focus_id());
            }
        }

        if !self.is_focused(focus) {
            return None;
        }

        match event {
            UiEvent::Key(KeyEvent {
                code, modifiers, ..
            }) => {
                // Commit/cancel are consistent across input + textarea.
                if *code == KeyCode::Enter
                    && !modifiers
                        .intersects(KeyModifiers::ALT | KeyModifiers::SHIFT | KeyModifiers::CONTROL)
                {
                    return Some(SingleLineInputMessage::Submit(
                        self.input.value().to_string(),
                    ));
                }
                if *code == KeyCode::Esc {
                    return Some(SingleLineInputMessage::Cancel);
                }

                // Extract the KeyEvent from UiEvent for apply_key
                if let UiEvent::Key(key_event) = event {
                    self.apply_key(key_event)
                } else {
                    None
                }
            }
            UiEvent::Paste(text) => self.apply_paste(text),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_component() {
        let c = SingleLineInputComponent::new(ComponentId::new("test", 0), 0, "hello");
        assert_eq!(c.value(), "hello");
        assert!(c.enabled);
    }

    #[test]
    fn test_set_placeholder() {
        let mut c = SingleLineInputComponent::new(ComponentId::new("test", 0), 0, "");
        c.set_placeholder("Type here...");
        assert_eq!(c.placeholder, Some("Type here...".to_string()));
    }

    #[test]
    fn test_apply_paste_normalizes_newlines() {
        let mut c = SingleLineInputComponent::new(ComponentId::new("test", 0), 0, "ab");
        // Cursor is already at end (2) after creating with "ab"

        // Paste with newlines - should be normalized
        let msg = c.apply_paste("cd\nef\rgh");
        assert!(msg.is_some());
        assert_eq!(c.value(), "abcdefgh");
    }

    #[test]
    fn test_apply_paste_empty() {
        let mut c = SingleLineInputComponent::new(ComponentId::new("test", 0), 0, "hello");
        let msg = c.apply_paste("");
        assert!(msg.is_none());
        assert_eq!(c.value(), "hello");
    }

    #[test]
    fn test_apply_paste_only_newlines() {
        let mut c = SingleLineInputComponent::new(ComponentId::new("test", 0), 0, "hello");
        let msg = c.apply_paste("\n\r\n");
        assert!(msg.is_none());
        assert_eq!(c.value(), "hello");
    }

    #[test]
    fn test_render_line_with_cursor() {
        let c = SingleLineInputComponent::new(ComponentId::new("test", 0), 0, "hello");
        let (line, _) = c.render_line_and_cursor(20, true);
        // Should contain cursor marker
        assert!(line.contains('▏'));
    }

    #[test]
    fn test_focus_id() {
        let c = SingleLineInputComponent::new(ComponentId::new("test", 0), 5, "");
        let fid = c.focus_id();
        assert_eq!(fid.component.kind, "test");
        assert_eq!(fid.local, 5);
    }
}
