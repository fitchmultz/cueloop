// Package tui provides tests for the logs view rendering.
package tui

import (
	"encoding/json"
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

func TestLogsViewFormattedRendersJSONL(t *testing.T) {
	tmpDir := t.TempDir()
	logPath := filepath.Join(tmpDir, "ralph_tui.log")
	entry := logEntry{
		Timestamp: "2026-01-14T12:34:56Z",
		Level:     "info",
		Message:   "tui.start",
		Fields: map[string]any{
			"screen": "logs",
			"count":  2,
		},
	}
	payload, err := json.Marshal(entry)
	if err != nil {
		t.Fatalf("marshal log entry: %v", err)
	}
	if err := os.WriteFile(logPath, append(payload, '\n'), 0o600); err != nil {
		t.Fatalf("write log file: %v", err)
	}

	view := newLogsView(logPath)
	view.Refresh(nil, nil)
	view.ToggleFormat()
	content := view.renderContent()

	if strings.Contains(content, string(payload)) {
		t.Fatalf("expected formatted output instead of raw JSONL, got %q", content)
	}
	if !strings.Contains(content, "2026-01-14 12:34:56Z") {
		t.Fatalf("expected formatted timestamp, got %q", content)
	}
	if !strings.Contains(content, "INFO") || !strings.Contains(content, "tui.start") {
		t.Fatalf("expected formatted level and message, got %q", content)
	}
	if !strings.Contains(content, "count=2") || !strings.Contains(content, "screen=logs") {
		t.Fatalf("expected formatted fields, got %q", content)
	}
}
