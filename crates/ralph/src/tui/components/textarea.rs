//! Multi-line rich text area component for the Ralph TUI.
//!
//! Responsibilities:
//! - Provide a reusable, focus-aware multi-line editor implementing the foundation `Component` trait.
//! - Support cursor movement, insertion/deletion, and scroll behavior via `tui-textarea`.
//! - Define consistent commit/cancel behavior (Enter commits, Esc cancels) and newline insertion (Alt+Enter).
//! - Safely handle `UiEvent::Paste` by inserting multi-line text without panicking.
//! - Support placeholder rendering when empty and not focused.
//!
//! Not handled here:
//! - List/map field conversion policy (comma-separated vs newline-separated); callers decide.
//! - Advanced editing features (selection, undo/redo policy exposure, syntax highlighting).
//!
//! Invariants/assumptions:
//! - Enter (no modifiers) is "Submit", not newline; use Alt+Enter for newline.
//! - This component only mutates its state when focused and enabled.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::tui::{
    App,
    foundation::{Component, ComponentId, FocusId, FocusManager, RenderCtx, UiEvent},
    textarea_input::MultiLineInput,
};

/// Messages produced by `TextAreaComponent`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TextAreaMessage {
    /// Value changed (new value).
    Changed(String),
    /// User pressed Enter to submit.
    Submit(String),
    /// User pressed Esc to cancel.
    Cancel,
}

/// Focus-aware multi-line text area component.
pub(crate) struct TextAreaComponent {
    id: ComponentId,
    focus_local: u16,
    enabled: bool,

    input: MultiLineInput,
    placeholder: Option<String>,
    last_area: Option<Rect>,
}

impl TextAreaComponent {
    /// Create a new textarea component.
    pub(crate) fn new(id: ComponentId, focus_local: u16, value: impl Into<String>) -> Self {
        Self {
            id,
            focus_local,
            enabled: true,
            input: MultiLineInput::new(value, false),
            placeholder: None,
            last_area: None,
        }
    }

    /// Configure this textarea as a list field (newline-separated items).
    pub(crate) fn with_list_field(mut self, is_list_field: bool) -> Self {
        let v = self.input.value();
        self.input = MultiLineInput::new(v, is_list_field);
        self
    }

    /// Set whether the component is enabled.
    pub(crate) fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Set placeholder text shown when textarea is empty.
    pub(crate) fn set_placeholder(&mut self, placeholder: impl Into<String>) {
        self.placeholder = Some(placeholder.into());
    }

    /// Get the focus ID for this component.
    pub(crate) fn focus_id(&self) -> FocusId {
        FocusId::new(self.id, self.focus_local)
    }

    /// Get the current text value.
    pub(crate) fn value(&self) -> String {
        self.input.value()
    }

    /// Check if this component is currently focused.
    fn is_focused(&self, focus: &FocusManager) -> bool {
        focus.is_focused(self.focus_id())
    }

    /// Insert pasted text, normalizing line endings to newlines.
    fn insert_paste(&mut self, text: &str) -> bool {
        let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
        let mut first = true;
        for line in normalized.split('\n') {
            if !first {
                self.input.textarea_mut().insert_newline();
            }
            first = false;
            if !line.is_empty() {
                self.input.textarea_mut().insert_str(line);
            }
        }
        // Treat as changed if any non-empty or any newline existed.
        !normalized.is_empty()
    }
}

impl Component for TextAreaComponent {
    type Message = TextAreaMessage;

    fn render(&mut self, f: &mut Frame<'_>, area: Rect, _app: &App, ctx: &mut RenderCtx<'_>) {
        self.last_area = Some(area);

        let block = Block::default().borders(Borders::ALL);
        let inner = block.inner(area);
        f.render_widget(block, area);

        ctx.register_focus(self.focus_id(), area, self.enabled);

        let is_empty = self.input.value().is_empty();
        let placeholder = self.placeholder.as_deref().unwrap_or("");

        if is_empty && !placeholder.is_empty() {
            let p = Paragraph::new(Line::from(Span::styled(
                placeholder.to_string(),
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::ITALIC),
            )));
            f.render_widget(p, inner);
        } else {
            f.render_widget(self.input.widget(), inner);
        }
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

        // Click-to-focus.
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
                code: KeyCode::Enter,
                modifiers,
                ..
            }) => {
                if modifiers.contains(KeyModifiers::ALT) {
                    // Alt+Enter inserts newline.
                    let changed = self
                        .input
                        .input(KeyEvent::new(KeyCode::Enter, KeyModifiers::ALT));
                    if changed {
                        return Some(TextAreaMessage::Changed(self.input.value()));
                    }
                    return None;
                }
                Some(TextAreaMessage::Submit(self.input.value()))
            }
            UiEvent::Key(KeyEvent {
                code: KeyCode::Esc, ..
            }) => Some(TextAreaMessage::Cancel),
            UiEvent::Key(k) => {
                let before = self.input.value();
                let _handled_or_changed = self.input.input(*k);
                let after = self.input.value();
                if after != before {
                    Some(TextAreaMessage::Changed(after))
                } else {
                    None
                }
            }
            UiEvent::Paste(text) => {
                let before = self.input.value();
                let changed = self.insert_paste(text);
                let after = self.input.value();
                if changed && after != before {
                    Some(TextAreaMessage::Changed(after))
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_component() {
        let c = TextAreaComponent::new(ComponentId::new("test", 0), 0, "hello\nworld");
        assert_eq!(c.value(), "hello\nworld");
        assert!(c.enabled);
    }

    #[test]
    fn test_with_list_field() {
        let c =
            TextAreaComponent::new(ComponentId::new("test", 0), 0, "a, b, c").with_list_field(true);
        // List field converts comma-separated to newline-separated
        assert_eq!(c.value(), "a\nb\nc");
    }

    #[test]
    fn test_set_placeholder() {
        let mut c = TextAreaComponent::new(ComponentId::new("test", 0), 0, "");
        c.set_placeholder("Enter text...");
        assert_eq!(c.placeholder, Some("Enter text...".to_string()));
    }

    #[test]
    fn test_focus_id() {
        let c = TextAreaComponent::new(ComponentId::new("test", 0), 3, "");
        let fid = c.focus_id();
        assert_eq!(fid.component.kind, "test");
        assert_eq!(fid.local, 3);
    }

    #[test]
    fn test_insert_paste_normalizes_crlf() {
        let mut c = TextAreaComponent::new(ComponentId::new("test", 0), 0, "");
        let changed = c.insert_paste("line1\r\nline2\rline3");
        assert!(changed);
        assert_eq!(c.value(), "line1\nline2\nline3");
    }

    #[test]
    fn test_insert_paste_empty() {
        let mut c = TextAreaComponent::new(ComponentId::new("test", 0), 0, "existing");
        let changed = c.insert_paste("");
        assert!(!changed);
        assert_eq!(c.value(), "existing");
    }

    #[test]
    fn test_insert_paste_single_line() {
        let mut c = TextAreaComponent::new(ComponentId::new("test", 0), 0, "");
        let changed = c.insert_paste("just one line");
        assert!(changed);
        assert_eq!(c.value(), "just one line");
    }
}
