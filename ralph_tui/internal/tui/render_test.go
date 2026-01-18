// Package tui tests render helpers.
package tui

import (
	"strings"
	"testing"

	"github.com/charmbracelet/x/ansi"
)

func TestClampToSizeDoesNotWrap(t *testing.T) {
	longLine := strings.Repeat("x", 200)
	output := clampToSize(longLine, 20, 1)
	lines := strings.Split(output, "\n")
	if len(lines) != 1 {
		t.Fatalf("expected 1 line, got %d", len(lines))
	}
	if width := ansi.StringWidth(lines[0]); width > 20 {
		t.Fatalf("expected width <= 20, got %d", width)
	}
}
