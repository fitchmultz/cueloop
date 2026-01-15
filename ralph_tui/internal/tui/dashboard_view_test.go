// Package tui provides tests for the dashboard view summary behavior.
// Entrypoint: go test ./...
package tui

import (
	"os"
	"path/filepath"
	"testing"

	"github.com/mitchfultz/ralph/ralph_tui/internal/pin"
)

func TestDashboardQueueSummaryUsesUnfilteredItemsAndDoneFile(t *testing.T) {
	tmpDir := t.TempDir()
	donePath := filepath.Join(tmpDir, "implementation_done.md")
	doneContent := "# Implementation Done\n\n## Done\n" +
		"- [x] RQ-0999 [ui]: Most recent done item. (dashboard_view.go)\n" +
		"  - Evidence: recent\n" +
		"  - Plan: done\n" +
		"- [x] RQ-0998 [ui]: Older done item. (dashboard_view.go)\n" +
		"  - Evidence: older\n" +
		"  - Plan: done\n"
	if err := os.WriteFile(donePath, []byte(doneContent), 0o600); err != nil {
		t.Fatalf("write done fixture: %v", err)
	}

	allItems := []pin.QueueItem{
		{ID: "RQ-0001", Checked: true},
		{ID: "RQ-0002", Checked: false},
		{ID: "RQ-0003", Checked: false},
	}
	filtered := []pin.QueueItem{allItems[2]}

	view := &pinView{
		files:        pin.Files{DonePath: donePath},
		items:        filtered,
		allItems:     allItems,
		blockedCount: 2,
	}

	summary := dashboardQueueSummaryFor(view)
	if summary.summary != "3 queued | 2 unchecked | 2 blocked" {
		t.Fatalf("unexpected summary: %q", summary.summary)
	}
	if summary.nextID != "RQ-0002" {
		t.Fatalf("unexpected next ID: %q", summary.nextID)
	}
	if summary.lastID != "RQ-0999" {
		t.Fatalf("unexpected last done ID: %q", summary.lastID)
	}
}
