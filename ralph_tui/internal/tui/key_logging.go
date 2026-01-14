// Package tui provides logging helpers for input events.
package tui

import (
	"unicode"

	tea "github.com/charmbracelet/bubbletea"
)

func keyEventSummary(msg tea.KeyMsg) map[string]any {
	summary := map[string]any{
		"type": msg.Type.String(),
		"alt":  msg.Alt,
	}
	if msg.Type == tea.KeyRunes {
		summary["rune_count"] = len(msg.Runes)
		summary["printable"] = runesPrintable(msg.Runes)
		return summary
	}
	summary["key"] = msg.String()
	return summary
}

func runesPrintable(runes []rune) bool {
	for _, r := range runes {
		if !unicode.IsPrint(r) {
			return false
		}
	}
	return true
}
