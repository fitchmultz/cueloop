// Package tui provides tests for the logs view rendering.
package tui

import (
	"encoding/json"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"strings"
	"sync"
	"testing"
	"time"
)

func TestLogsViewRefreshRendersContent(t *testing.T) {
	tmpDir := t.TempDir()
	logPath := filepath.Join(tmpDir, "ralph_tui.log")
	if err := os.WriteFile(logPath, []byte("first\nsecond\n"), 0o600); err != nil {
		t.Fatalf("write log file: %v", err)
	}
	loopPath := loopOutputLogPath(tmpDir)
	if err := os.WriteFile(loopPath, []byte("loop line\n"), 0o600); err != nil {
		t.Fatalf("write loop output: %v", err)
	}
	specsPath := specsOutputLogPath(tmpDir)
	if err := os.WriteFile(specsPath, []byte("spec line\n"), 0o600); err != nil {
		t.Fatalf("write specs output: %v", err)
	}

	view := newLogsView(logPath)
	view.SetCacheDir(tmpDir)
	view.Refresh()
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
	view.Refresh()
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

func TestTailFileLines(t *testing.T) {
	t.Parallel()

	tmpDir := t.TempDir()
	missingPath := filepath.Join(tmpDir, "missing.log")

	makeLines := func(count int) []string {
		lines := make([]string, count)
		for i := 0; i < count; i++ {
			lines[i] = fmt.Sprintf("line-%d", i)
		}
		return lines
	}

	tests := []struct {
		name    string
		content string
		limit   int
		want    []string
		path    string
	}{
		{
			name:  "missing file",
			path:  missingPath,
			limit: 5,
			want:  []string{},
		},
		{
			name:    "empty file",
			content: "",
			limit:   5,
			want:    []string{},
		},
		{
			name:    "fewer than limit",
			content: "a\nb\n",
			limit:   200,
			want:    []string{"a", "b"},
		},
		{
			name:    "crlf trailing newline",
			content: "a\r\nb\r\n",
			limit:   10,
			want:    []string{"a", "b"},
		},
		{
			name:    "crlf no trailing newline",
			content: "a\r\nb",
			limit:   10,
			want:    []string{"a", "b"},
		},
		{
			name:    "exactly limit",
			content: strings.Join(makeLines(5), "\n") + "\n",
			limit:   5,
			want:    makeLines(5),
		},
		{
			name:    "no trailing newline",
			content: "a\nb",
			limit:   10,
			want:    []string{"a", "b"},
		},
		{
			name:    "large file tail",
			content: strings.Join(makeLines(20000), "\n") + "\n",
			limit:   200,
			want:    makeLines(20000)[19800:],
		},
		{
			name:    "long line boundary",
			content: "start\n" + strings.Repeat("x", 70*1024) + "\nend1\nend2\n",
			limit:   2,
			want:    []string{"end1", "end2"},
		},
	}

	for _, test := range tests {
		test := test
		t.Run(test.name, func(t *testing.T) {
			path := test.path
			if path == "" {
				path = filepath.Join(tmpDir, test.name+".log")
			}
			if test.path == "" {
				if err := os.WriteFile(path, []byte(test.content), 0o600); err != nil {
					t.Fatalf("write log file: %v", err)
				}
			}

			got, err := tailFileLines(path, test.limit)
			if err != nil {
				t.Fatalf("tailFileLines: %v", err)
			}
			if strings.Join(got, "\n") != strings.Join(test.want, "\n") {
				t.Fatalf("unexpected lines: got %q want %q", got, test.want)
			}
		})
	}
}

func TestTailFileLinesFromHandleTruncationDuringRead(t *testing.T) {
	initialLines := []string{"line-1", "line-2", "line-3", "line-4", "line-5"}
	initialData := []byte(strings.Join(initialLines, "\n") + "\n")

	file := newMemoryTailFile(initialData)
	file.onRead = func(f *memoryTailFile) {
		f.data = []byte("fresh-1\nfresh-2\n")
	}

	lines, err := tailFileLinesFromHandle(file, 5)
	if err != nil {
		t.Fatalf("tailFileLinesFromHandle: %v", err)
	}
	if strings.Join(lines, "\n") != "fresh-1\nfresh-2" {
		t.Fatalf("unexpected lines after truncation: %q", lines)
	}
}

func TestLogsViewRefreshRotationGapKeepsDebugTail(t *testing.T) {
	tmpDir := t.TempDir()
	logPath := filepath.Join(tmpDir, "ralph_tui.log")
	if err := os.WriteFile(logPath, []byte("first\nsecond\n"), 0o600); err != nil {
		t.Fatalf("write log file: %v", err)
	}

	view := newLogsView(logPath)
	view.Refresh()
	if !strings.Contains(view.renderContent(), "second") {
		t.Fatalf("expected initial log content")
	}

	rotatedPath := logPath + ".1"
	if err := os.Rename(logPath, rotatedPath); err != nil {
		t.Fatalf("rotate log file: %v", err)
	}
	view.Refresh()
	if !strings.Contains(view.renderContent(), "second") {
		t.Fatalf("expected logs view to keep previous tail during rotation gap")
	}

	if err := os.WriteFile(logPath, []byte("fresh\n"), 0o600); err != nil {
		t.Fatalf("write new log file: %v", err)
	}
	view.Refresh()
	if !strings.Contains(view.renderContent(), "fresh") {
		t.Fatalf("expected logs view to read new log content after rotation")
	}
}

func TestLogsViewRefreshSuppressesRedundantViewportUpdates(t *testing.T) {
	tmpDir := t.TempDir()
	logPath := filepath.Join(tmpDir, "ralph_tui.log")
	if err := os.WriteFile(logPath, []byte("first\nsecond\n"), 0o600); err != nil {
		t.Fatalf("write log file: %v", err)
	}
	loopPath := loopOutputLogPath(tmpDir)
	if err := os.WriteFile(loopPath, []byte("loop line\n"), 0o600); err != nil {
		t.Fatalf("write loop output: %v", err)
	}
	specsPath := specsOutputLogPath(tmpDir)
	if err := os.WriteFile(specsPath, []byte("spec line\n"), 0o600); err != nil {
		t.Fatalf("write specs output: %v", err)
	}

	view := newLogsView(logPath)
	view.SetCacheDir(tmpDir)
	view.Refresh()
	if view.viewportSetContentCalls != 1 {
		t.Fatalf("expected one viewport update, got %d", view.viewportSetContentCalls)
	}

	view.Refresh()
	if view.viewportSetContentCalls != 1 {
		t.Fatalf("expected cached viewport update, got %d", view.viewportSetContentCalls)
	}

	handle, err := os.OpenFile(logPath, os.O_APPEND|os.O_WRONLY, 0o600)
	if err != nil {
		t.Fatalf("open log file: %v", err)
	}
	if _, err := handle.WriteString("third\n"); err != nil {
		_ = handle.Close()
		t.Fatalf("append log file: %v", err)
	}
	if err := handle.Close(); err != nil {
		t.Fatalf("close log file: %v", err)
	}

	view.Refresh()
	if view.viewportSetContentCalls != 2 {
		t.Fatalf("expected refreshed viewport update, got %d", view.viewportSetContentCalls)
	}
}

func TestLogsViewFilters(t *testing.T) {
	tmpDir := t.TempDir()
	logPath := filepath.Join(tmpDir, "ralph_tui.log")
	entries := []logEntry{
		{Timestamp: "2026-01-14T12:34:56Z", Level: "info", Message: "tui.start"},
		{Timestamp: "2026-01-14T12:35:00Z", Level: "error", Message: "specs.fail"},
		{Timestamp: "2026-01-14T12:35:30Z", Level: "warn", Message: "loop.warn"},
	}
	lines := make([]string, 0, len(entries)+1)
	for _, entry := range entries {
		payload, err := json.Marshal(entry)
		if err != nil {
			t.Fatalf("marshal log entry: %v", err)
		}
		lines = append(lines, string(payload))
	}
	lines = append(lines, "not-json")
	if err := os.WriteFile(logPath, []byte(strings.Join(lines, "\n")+"\n"), 0o600); err != nil {
		t.Fatalf("write log file: %v", err)
	}

	view := newLogsView(logPath)
	view.Refresh()
	view.ToggleFormat()

	view.SetLevelFilter("error")
	content := view.renderContent()
	if !strings.Contains(content, "ERROR") || !strings.Contains(content, "specs.fail") {
		t.Fatalf("expected error log line, got %q", content)
	}
	if strings.Contains(content, "INFO") || strings.Contains(content, "WARN") || strings.Contains(content, "not-json") {
		t.Fatalf("expected non-error lines to be filtered, got %q", content)
	}

	view.ClearFilters()
	view.SetComponentFilter("LOOP")
	content = view.renderContent()
	if !strings.Contains(content, "loop.warn") || !strings.Contains(content, "WARN") {
		t.Fatalf("expected loop component line, got %q", content)
	}
	if strings.Contains(content, "tui.start") || strings.Contains(content, "specs.fail") {
		t.Fatalf("expected non-loop components to be filtered, got %q", content)
	}
}

func TestTailFileLinesFromHandleTruncationAcrossChunks(t *testing.T) {
	largeLine := strings.Repeat("x", 70*1024)
	initialData := []byte(largeLine + "\nline-1\nline-2\nline-3\n")

	file := newMemoryTailFile(initialData)
	file.onReadCount = func(f *memoryTailFile, count int) {
		if count == 2 {
			f.data = []byte("fresh-1\nfresh-2\n")
		}
	}

	lines, err := tailFileLinesFromHandle(file, 5)
	if err != nil {
		t.Fatalf("tailFileLinesFromHandle: %v", err)
	}
	if strings.Join(lines, "\n") != "fresh-1\nfresh-2" {
		t.Fatalf("unexpected lines after truncation: %q", lines)
	}
}

type memoryTailFile struct {
	mu          sync.Mutex
	data        []byte
	onRead      func(*memoryTailFile)
	readCount   int
	onReadCount func(*memoryTailFile, int)
}

func newMemoryTailFile(data []byte) *memoryTailFile {
	return &memoryTailFile{data: data}
}

func (m *memoryTailFile) Stat() (os.FileInfo, error) {
	m.mu.Lock()
	defer m.mu.Unlock()
	return memoryFileInfo{size: int64(len(m.data))}, nil
}

func (m *memoryTailFile) ReadAt(p []byte, off int64) (int, error) {
	m.mu.Lock()
	m.readCount++
	if m.onReadCount != nil {
		m.onReadCount(m, m.readCount)
	}
	if m.onRead != nil {
		m.onRead(m)
		m.onRead = nil
	}
	if off >= int64(len(m.data)) {
		m.mu.Unlock()
		return 0, io.EOF
	}
	n := copy(p, m.data[off:])
	m.mu.Unlock()
	if n < len(p) {
		return n, io.EOF
	}
	return n, nil
}

type memoryFileInfo struct {
	size int64
}

func (m memoryFileInfo) Name() string       { return "memory.log" }
func (m memoryFileInfo) Size() int64        { return m.size }
func (m memoryFileInfo) Mode() os.FileMode  { return 0o600 }
func (m memoryFileInfo) ModTime() time.Time { return time.Unix(0, 0) }
func (m memoryFileInfo) IsDir() bool        { return false }
func (m memoryFileInfo) Sys() any           { return nil }
