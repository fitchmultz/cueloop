// Package tui provides tests for key logging summaries.
package tui

import (
	"testing"

	tea "github.com/charmbracelet/bubbletea"
)

func TestKeyEventSummaryDoesNotLogRunes(t *testing.T) {
	msg := tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune("secret")}
	summary := keyEventSummary(msg)

	if _, ok := summary["key"]; ok {
		t.Fatalf("expected no key value for rune input")
	}
	if summary["rune_count"] != len([]rune("secret")) {
		t.Fatalf("expected rune_count to match input")
	}
}
