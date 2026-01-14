// Package tui provides tests for loop view behaviors.
package tui

import (
	"testing"

	tea "github.com/charmbracelet/bubbletea"
	"github.com/mitchfultz/ralph/ralph_tui/internal/config"
	"github.com/mitchfultz/ralph/ralph_tui/internal/paths"
)

func TestLoopStopTransitionsToStopping(t *testing.T) {
	cfg := config.Config{
		Loop: config.LoopConfig{
			SleepSeconds:      0,
			MaxIterations:     0,
			MaxStalled:        0,
			MaxRepairAttempts: 0,
			OnlyTags:          "",
			RequireMain:       false,
		},
		Git: config.GitConfig{
			AutoCommit: false,
			AutoPush:   false,
		},
	}
	view := newLoopView(cfg, paths.Locations{})
	view.mode = loopRunning
	cancelled := false
	view.cancel = func() { cancelled = true }

	_ = view.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune{'s'}}, newKeyMap())

	if !cancelled {
		t.Fatalf("expected stop to invoke cancel")
	}
	if view.mode != loopStopping {
		t.Fatalf("expected loopStopping, got %v", view.mode)
	}
	if view.status != "Stopping..." {
		t.Fatalf("expected status to be Stopping..., got %q", view.status)
	}
}
