// Package tui provides rendering helpers for the Bubble Tea model.
// Entrypoint: withFinalNewline.
package tui

import (
	"strings"

	"github.com/charmbracelet/lipgloss"
)

// withFinalNewline preserves leading/trailing spaces but ensures exactly one trailing newline.
// TUIs treat whitespace as layout; never TrimSpace rendered output.
func withFinalNewline(s string) string {
	s = strings.TrimRight(s, "\n")
	return s + "\n"
}

// clampToSize ensures the rendered output never exceeds the provided width or height.
func clampToSize(s string, width int, height int) string {
	lines := strings.Split(s, "\n")
	if height > 0 && len(lines) > height {
		lines = lines[:height]
	}
	if width > 0 {
		clampStyle := lipgloss.NewStyle().Width(width)
		for i, line := range lines {
			lines[i] = clampStyle.Render(line)
		}
	}
	return strings.Join(lines, "\n")
}
