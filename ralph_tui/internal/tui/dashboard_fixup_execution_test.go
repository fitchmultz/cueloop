// Package tui provides dashboard fixup execution tests.
package tui

import (
	"context"
	"strings"
	"testing"

	tea "github.com/charmbracelet/bubbletea"
	"github.com/mitchfultz/ralph/ralph_tui/internal/loop"
)

func TestDashboardFixupKeyExecutesRunnerAndSurfacesFailures(t *testing.T) {
	m, _, _ := newHermeticModel(t)
	m.screen = screenDashboard
	m.navFocused = true

	called := false
	m.fixupRunner = func(ctx context.Context, opts loop.FixupOptions) (loop.FixupResult, error) {
		called = true
		if opts.Logger != nil {
			opts.Logger.WriteLine(">> [RALPH] Fixup test log line")
		}
		return loop.FixupResult{
			ScannedBlocked: 1,
			Eligible:       1,
			FailedIDs:      []string{"RQ-0009"},
			FailedReasons: []loop.FixupFailure{{
				ID:     "RQ-0009",
				Reason: "ci failed",
			}},
		}, nil
	}

	updated, cmd := m.Update(tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune("f")})
	m = unwrapModel(t, updated)
	m = driveCmdsUntil(t, m, cmd, func(next model) bool {
		return next.fixup.hasSummary && !next.fixup.running
	})

	if !called {
		t.Fatalf("expected fixup runner to be invoked")
	}
	if !strings.Contains(m.fixup.lastLogLine, "Fixup test log line") {
		t.Fatalf("expected fixup last log line to be captured, got %q", m.fixup.lastLogLine)
	}
	view := m.contentView()
	if !strings.Contains(view, "Fixup: Scanned 1 | Eligible 1 | Requeued 0 | Skipped 0 | Failed 1") {
		t.Fatalf("expected fixup counts in dashboard status, got %q", view)
	}
	if !strings.Contains(view, "Failed RQ-0009: ci failed") {
		t.Fatalf("expected fixup failure details in dashboard status, got %q", view)
	}
}
