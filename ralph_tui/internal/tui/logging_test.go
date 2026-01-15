// Package tui provides tests for the TUI debug logger.
package tui

import (
	"bytes"
	"os"
	"path/filepath"
	"strings"
	"testing"

	"github.com/mitchfultz/ralph/ralph_tui/internal/config"
)

func TestTUILoggerWritesJSONL(t *testing.T) {
	tmpDir := t.TempDir()
	logPath := filepath.Join(tmpDir, "ralph_tui.log")
	cfg := config.Config{
		Logging: config.LoggingConfig{
			Level: "debug",
			File:  logPath,
		},
		Paths: config.PathsConfig{
			CacheDir: tmpDir,
		},
	}

	logger, err := newTUILogger(cfg)
	if err != nil {
		t.Fatalf("newTUILogger failed: %v", err)
	}
	t.Cleanup(func() {
		_ = logger.Close()
	})
	logger.Info("test.event", map[string]any{"note": "hello"})

	data, err := os.ReadFile(logPath)
	if err != nil {
		t.Fatalf("read log file: %v", err)
	}
	payload := string(data)
	if !strings.Contains(payload, "\"msg\":\"test.event\"") {
		t.Fatalf("expected log entry in payload, got %q", payload)
	}
}

func TestTUILoggerRotatesOversizedLogOnStartup(t *testing.T) {
	tmpDir := t.TempDir()
	logPath := filepath.Join(tmpDir, "ralph_tui.log")
	oversized := bytes.Repeat([]byte("a"), int(maxLogSizeBytes+10))
	if err := os.WriteFile(logPath, oversized, 0o600); err != nil {
		t.Fatalf("write oversized log: %v", err)
	}

	cfg := config.Config{
		Logging: config.LoggingConfig{
			Level: "debug",
			File:  logPath,
		},
		Paths: config.PathsConfig{
			CacheDir: tmpDir,
		},
	}

	logger, err := newTUILogger(cfg)
	if err != nil {
		t.Fatalf("newTUILogger failed: %v", err)
	}
	t.Cleanup(func() {
		_ = logger.Close()
	})

	backupPath := logPath + ".1"
	backupInfo, err := os.Stat(backupPath)
	if err != nil {
		t.Fatalf("expected rotated log at %s: %v", backupPath, err)
	}
	if backupInfo.Size() != int64(len(oversized)) {
		t.Fatalf("expected rotated log size %d, got %d", len(oversized), backupInfo.Size())
	}

	logger.Info("post.rotate", nil)

	currentData, err := os.ReadFile(logPath)
	if err != nil {
		t.Fatalf("read current log: %v", err)
	}
	if !strings.Contains(string(currentData), "\"msg\":\"post.rotate\"") {
		t.Fatalf("expected post-rotation log entry in current log")
	}

	backupData, err := os.ReadFile(backupPath)
	if err != nil {
		t.Fatalf("read rotated log: %v", err)
	}
	if strings.Contains(string(backupData), "\"msg\":\"post.rotate\"") {
		t.Fatalf("expected rotated log to remain unchanged after rotation")
	}
}
