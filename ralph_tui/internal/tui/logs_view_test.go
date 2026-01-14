// Package tui provides tests for the logs view rendering.
package tui

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestLogsViewRefreshRendersContent(t *testing.T) {
	tmpDir := t.TempDir()
	logPath := filepath.Join(tmpDir, "ralph_tui.log")
	if err := os.WriteFile(logPath, []byte("first\nsecond\n"), 0o600); err != nil {
		t.Fatalf("write log file: %v", err)
	}

	view := newLogsView(logPath)
	view.Refresh([]string{"loop line"}, []string{"spec line"})
	content := view.renderContent()

	if !strings.Contains(content, "second") {
		t.Fatalf("expected debug log content, got %q", content)
	}
	if !strings.Contains(content, "loop line") {
		t.Fatalf("expected loop content, got %q", content)
	}
	if !strings.Contains(content, "spec line") {
		t.Fatalf("expected specs content, got %q", content)
	}
}
