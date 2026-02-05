//! Line number gutter component for the Ralph TUI.
//!
//! Responsibilities:
//! - Render right-aligned line numbers with a configurable gutter width.
//! - Optionally render per-line highlight indicators (background + optional symbol).
//! - Optionally render per-line diagnostics (error/warning symbols).
//! - Degrade gracefully for narrow widths (never panic; show best-effort indicators).
//! - Operate alongside arbitrary content (the gutter is rendered separately from content).
//!
//! Not handled here:
//! - Owning or rendering the content itself (callers render content in a separate area).
//! - Input handling for "click line number to select" (can be added later if needed).
//! - Horizontal scrolling or text wrapping policies (belongs to the content renderer).
//!
//! Invariants/assumptions:
//! - Line numbers exposed to this component are 1-based (`1` is the first visible line).
//! - All rendering is immediate-mode and must be safe for any terminal size.

use std::collections::HashMap;

use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
};

use crate::tui::{
    App,
    foundation::{Component, ComponentId, FocusManager, RenderCtx, UiEvent},
};

/// Diagnostic severity for line indicators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DiagnosticSeverity {
    /// Error-level diagnostic.
    Error,
    /// Warning-level diagnostic.
    Warning,
}

/// Highlight configuration for a specific line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LineHighlight {
    /// Background color for the highlight.
    pub(crate) bg: Color,
    /// Optional symbol to display (first char used if too wide).
    pub(crate) symbol: Option<String>,
}

/// Configuration for the line number gutter appearance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LineNumberGutterConfig {
    /// Width of the gutter in columns.
    pub(crate) width: u16,
    /// Default style for line numbers.
    pub(crate) line_number_style: Style,

    /// Style for highlighted/active line numbers.
    pub(crate) highlight_number_style: Style,
    /// Default background color for highlighted lines.
    pub(crate) highlight_default_bg: Color,
    /// Fallback symbol when highlight has no symbol.
    pub(crate) highlight_symbol_fallback: String,

    /// Whether diagnostics are enabled.
    pub(crate) diagnostics_enabled: bool,
    /// Symbol for error diagnostics.
    pub(crate) error_symbol: String,
    /// Symbol for warning diagnostics.
    pub(crate) warning_symbol: String,
    /// Style for error diagnostics.
    pub(crate) error_style: Style,
    /// Style for warning diagnostics.
    pub(crate) warning_style: Style,
}

impl Default for LineNumberGutterConfig {
    fn default() -> Self {
        Self {
            width: 6,
            line_number_style: Style::default().fg(Color::DarkGray),

            highlight_number_style: Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
            highlight_default_bg: Color::Blue,
            highlight_symbol_fallback: "▶".to_string(),

            diagnostics_enabled: true,
            error_symbol: "⨯".to_string(),
            warning_symbol: "!".to_string(),
            error_style: Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            warning_style: Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        }
    }
}

/// Line number gutter component that renders line numbers with optional highlights and diagnostics.
pub(crate) struct LineNumberGutterComponent {
    id: ComponentId,
    enabled: bool,

    cfg: LineNumberGutterConfig,

    start_line: u32, // 1-based
    line_count: u32, // how many logical lines exist
    active_line: Option<u32>,

    highlights: HashMap<u32, LineHighlight>, // line -> highlight
    diagnostics: HashMap<u32, DiagnosticSeverity>, // line -> diag
}

impl LineNumberGutterComponent {
    /// Create a new line number gutter component with the given configuration.
    pub(crate) fn new(cfg: LineNumberGutterConfig) -> Self {
        Self {
            id: ComponentId::new("line_number_gutter", 0),
            enabled: false, // gutter is non-interactive by default
            cfg,
            start_line: 1,
            line_count: 0,
            active_line: None,
            highlights: HashMap::new(),
            diagnostics: HashMap::new(),
        }
    }

    /// Enable or disable the gutter for event handling.
    pub(crate) fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Set the visible line range.
    ///
    /// `start_line` is 1-based. `line_count` is the total number of lines in the document.
    pub(crate) fn set_range(&mut self, start_line: u32, line_count: u32) {
        self.start_line = start_line.max(1);
        self.line_count = line_count;
    }

    /// Set the currently active/focused line (if any).
    pub(crate) fn set_active_line(&mut self, line: Option<u32>) {
        self.active_line = line;
    }

    /// Set per-line highlights.
    pub(crate) fn set_highlights(&mut self, highlights: HashMap<u32, LineHighlight>) {
        self.highlights = highlights;
    }

    /// Set per-line diagnostics.
    pub(crate) fn set_diagnostics(&mut self, diagnostics: HashMap<u32, DiagnosticSeverity>) {
        self.diagnostics = diagnostics;
    }

    /// Calculate the visible width given the area and config.
    fn visible_width(area: Rect, cfg_width: u16) -> u16 {
        area.width.min(cfg_width)
    }

    /// Format a number right-aligned within the given width.
    /// If the number is too wide, shows the rightmost digits.
    fn right_align_number(num: u32, width: usize) -> String {
        let s = num.to_string();
        if width == 0 {
            return String::new();
        }
        let char_count = s.chars().count();
        if char_count >= width {
            // If too narrow, show the rightmost digits.
            return s
                .chars()
                .rev()
                .take(width)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect();
        }
        format!("{:>width$}", s, width = width)
    }

    /// Get the first character of a string, or space if empty.
    fn first_char_symbol(s: &str) -> char {
        s.chars().next().unwrap_or(' ')
    }

    /// Get the diagnostic symbol and style for a line, if any.
    fn diag_symbol_and_style(&self, line: u32) -> Option<(char, Style)> {
        if !self.cfg.diagnostics_enabled {
            return None;
        }
        match self.diagnostics.get(&line).copied() {
            None => None,
            Some(DiagnosticSeverity::Error) => Some((
                Self::first_char_symbol(&self.cfg.error_symbol),
                self.cfg.error_style,
            )),
            Some(DiagnosticSeverity::Warning) => Some((
                Self::first_char_symbol(&self.cfg.warning_symbol),
                self.cfg.warning_style,
            )),
        }
    }

    /// Get the highlight symbol and background for a line, if any.
    fn highlight_symbol_and_bg(&self, line: u32) -> Option<(char, Color)> {
        let hl = self.highlights.get(&line)?;
        let symbol = hl
            .symbol
            .as_deref()
            .unwrap_or(&self.cfg.highlight_symbol_fallback);
        Some((Self::first_char_symbol(symbol), hl.bg))
    }

    /// Get the appropriate style for a line number.
    fn line_number_style_for(&self, line: u32) -> Style {
        if self.active_line == Some(line) {
            self.cfg.highlight_number_style
        } else {
            self.cfg.line_number_style
        }
    }

    /// Render a single row of the gutter.
    fn render_row(&self, f: &mut Frame<'_>, area: Rect, row: u16, line: u32) {
        let w = area.width as i16;
        if w <= 0 {
            return;
        }

        let y = area.y.saturating_add(row);
        if y >= area.y.saturating_add(area.height) {
            return;
        }

        // Layout policy:
        // - Column 0: diagnostics if enabled and width>=1
        // - Column 1: highlight symbol if width>=2
        // - Remaining columns: right-aligned number
        let mut bg: Option<Color> = None;
        let mut diag: Option<(char, Style)> = None;
        let mut hl_sym: Option<char> = None;

        if let Some((sym, diag_style)) = self.diag_symbol_and_style(line) {
            diag = Some((sym, diag_style));
        }
        if let Some((sym, hl_bg)) = self.highlight_symbol_and_bg(line) {
            hl_sym = Some(sym);
            bg = Some(hl_bg);
        }

        let buf = f.buffer_mut();

        // Fill background across the full row if highlighted.
        if let Some(bg) = bg {
            for dx in 0..area.width {
                let x = area.x.saturating_add(dx);
                let cell = &mut buf[(x, y)];
                cell.set_style(Style::default().bg(bg));
            }
        }

        // Col 0: diag
        if area.width >= 1 {
            let (ch, st) = diag.unwrap_or((' ', Style::default()));
            let mut st = st;
            if let Some(bg) = bg {
                st = st.bg(bg);
            }
            buf[(area.x, y)].set_symbol(&ch.to_string()).set_style(st);
        }

        // Col 1: highlight symbol
        if area.width >= 2 {
            let ch = hl_sym.unwrap_or(' ');
            let mut st = Style::default();
            if let Some(bg) = bg {
                st = st.bg(bg).fg(Color::White).add_modifier(Modifier::BOLD);
            } else {
                st = st.fg(Color::DarkGray);
            }
            buf[(area.x.saturating_add(1), y)]
                .set_symbol(&ch.to_string())
                .set_style(st);
        }

        // Remaining: number (or blank if out of range)
        let number_cols = area.width.saturating_sub(2) as usize;
        let text = if line >= 1 && line <= self.line_count && number_cols > 0 {
            Self::right_align_number(line, number_cols)
        } else {
            " ".repeat(number_cols)
        };

        let mut st = self.line_number_style_for(line);
        if let Some(bg) = bg {
            st = st.bg(bg);
        }

        for (i, ch) in text.chars().enumerate() {
            let x = area.x.saturating_add(2).saturating_add(i as u16);
            if x >= area.x.saturating_add(area.width) {
                break;
            }
            buf[(x, y)].set_symbol(&ch.to_string()).set_style(st);
        }
    }
}

impl Component for LineNumberGutterComponent {
    type Message = ();

    fn render(&mut self, f: &mut Frame<'_>, area: Rect, _app: &App, _ctx: &mut RenderCtx<'_>) {
        let width = Self::visible_width(area, self.cfg.width);
        let area = Rect {
            x: area.x,
            y: area.y,
            width,
            height: area.height,
        };

        if area.width == 0 || area.height == 0 {
            return;
        }

        for row in 0..area.height {
            let line = self.start_line.saturating_add(row as u32);
            self.render_row(f, area, row, line);
        }
    }

    fn handle_event(
        &mut self,
        _event: &UiEvent,
        _app: &App,
        _focus: &mut FocusManager,
    ) -> Option<Self::Message> {
        // Intentionally non-interactive by default.
        if self.enabled { Some(()) } else { None }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn right_align_truncates_for_narrow_width() {
        assert_eq!(
            LineNumberGutterComponent::right_align_number(12345, 3),
            "345"
        );
        assert_eq!(LineNumberGutterComponent::right_align_number(7, 3), "  7");
        assert_eq!(LineNumberGutterComponent::right_align_number(7, 0), "");
    }

    #[test]
    fn right_align_handles_very_large_numbers() {
        assert_eq!(
            LineNumberGutterComponent::right_align_number(123456789, 5),
            "56789"
        );
    }

    #[test]
    fn first_char_symbol_behavior() {
        assert_eq!(LineNumberGutterComponent::first_char_symbol("▶"), '▶');
        assert_eq!(LineNumberGutterComponent::first_char_symbol(""), ' ');
        assert_eq!(LineNumberGutterComponent::first_char_symbol("abc"), 'a');
    }

    #[test]
    fn highlight_and_diag_helpers_are_safe() {
        let mut c = LineNumberGutterComponent::new(LineNumberGutterConfig::default());
        c.set_range(1, 10);

        let mut hl = HashMap::new();
        hl.insert(
            3,
            LineHighlight {
                bg: Color::Blue,
                symbol: Some("▶".to_string()),
            },
        );
        c.set_highlights(hl);

        let mut diag = HashMap::new();
        diag.insert(3, DiagnosticSeverity::Error);
        c.set_diagnostics(diag);

        assert!(c.highlight_symbol_and_bg(3).is_some());
        assert!(c.diag_symbol_and_style(3).is_some());
        assert!(c.diag_symbol_and_style(999).is_none());
        assert!(c.diag_symbol_and_style(1).is_none());
    }

    #[test]
    fn highlight_symbol_fallback_used() {
        let cfg = LineNumberGutterConfig::default();
        let mut c = LineNumberGutterComponent::new(cfg.clone());
        c.set_range(1, 10);

        let mut hl = HashMap::new();
        hl.insert(
            5,
            LineHighlight {
                bg: Color::Blue,
                symbol: None, // No custom symbol
            },
        );
        c.set_highlights(hl);

        let result = c.highlight_symbol_and_bg(5);
        assert!(result.is_some());
        let (sym, _) = result.unwrap();
        assert_eq!(
            sym,
            LineNumberGutterComponent::first_char_symbol(&cfg.highlight_symbol_fallback)
        );
    }

    #[test]
    fn line_number_style_for_active_line() {
        let cfg = LineNumberGutterConfig::default();
        let mut c = LineNumberGutterComponent::new(cfg.clone());
        c.set_range(1, 10);
        c.set_active_line(Some(5));

        let normal_style = c.line_number_style_for(3);
        let active_style = c.line_number_style_for(5);

        assert_eq!(normal_style, cfg.line_number_style);
        assert_eq!(active_style, cfg.highlight_number_style);
    }

    #[test]
    fn set_range_enforces_minimum_start() {
        let mut c = LineNumberGutterComponent::new(LineNumberGutterConfig::default());
        c.set_range(0, 10); // Try to set start to 0
        assert_eq!(c.start_line, 1); // Should be clamped to 1
    }

    #[test]
    fn diagnostics_disabled_returns_none() {
        let cfg = LineNumberGutterConfig {
            diagnostics_enabled: false,
            ..Default::default()
        };
        let mut c = LineNumberGutterComponent::new(cfg);
        c.set_range(1, 10);

        let mut diag = HashMap::new();
        diag.insert(3, DiagnosticSeverity::Error);
        c.set_diagnostics(diag);

        assert!(c.diag_symbol_and_style(3).is_none());
    }

    #[test]
    fn visible_width_respects_config_and_area() {
        let area = Rect::new(0, 0, 10, 5);
        assert_eq!(LineNumberGutterComponent::visible_width(area, 6), 6);

        let narrow_area = Rect::new(0, 0, 3, 5);
        assert_eq!(LineNumberGutterComponent::visible_width(narrow_area, 6), 3);
    }
}
