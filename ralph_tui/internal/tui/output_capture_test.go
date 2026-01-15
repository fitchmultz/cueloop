// Package tui provides tests for lossless output capture.
package tui

import (
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"
)

func TestLosslessOutputPersistenceUnderLoad(t *testing.T) {
	t.Parallel()

	outputDir := t.TempDir()
	outputPath := filepath.Join(outputDir, "output.log")
	writer := &outputFileWriter{}
	if err := writer.Reset(outputPath); err != nil {
		t.Fatalf("reset output writer: %v", err)
	}

	logCh := make(chan string, 4)
	sink := logChannelSink{ch: logCh}
	stream := newStreamWriter(sink)

	errCh := make(chan error, 1)
	doneCh := make(chan struct{})
	go func() {
		defer close(doneCh)
		for {
			batch := drainLogChannel(1, logCh, 256)
			if len(batch.Lines) > 0 {
				if err := writer.AppendLines(batch.Lines); err != nil {
					errCh <- err
					return
				}
			}
			if batch.Done {
				return
			}
		}
	}()

	const totalLines = 5000
	for i := 0; i < totalLines; i++ {
		if _, err := fmt.Fprintf(stream, "line %d\n", i); err != nil {
			t.Fatalf("write log line: %v", err)
		}
	}
	stream.Flush()
	close(logCh)

	<-doneCh
	select {
	case err := <-errCh:
		t.Fatalf("append lines: %v", err)
	default:
	}

	if err := writer.Close(); err != nil {
		t.Fatalf("close output writer: %v", err)
	}

	data, err := os.ReadFile(outputPath)
	if err != nil {
		t.Fatalf("read output file: %v", err)
	}
	contents := strings.TrimSpace(string(data))
	lines := strings.Split(contents, "\n")
	if len(lines) != totalLines {
		t.Fatalf("expected %d lines, got %d", totalLines, len(lines))
	}
	if lines[0] != "line 0" {
		t.Fatalf("expected first line to be %q, got %q", "line 0", lines[0])
	}
	if lines[len(lines)-1] != fmt.Sprintf("line %d", totalLines-1) {
		t.Fatalf("expected last line to be line %d, got %q", totalLines-1, lines[len(lines)-1])
	}
}

func TestOutputFileWriterFlushThresholds(t *testing.T) {
	t.Parallel()

	outputDir := t.TempDir()
	outputPath := filepath.Join(outputDir, "output.log")
	writer := &outputFileWriter{
		flushInterval:   time.Hour,
		maxPendingLines: 2,
		maxPendingBytes: 1 << 20,
		bufferSize:      1024,
	}
	if err := writer.Reset(outputPath); err != nil {
		t.Fatalf("reset output writer: %v", err)
	}

	if err := writer.AppendLines([]string{"line 1"}); err != nil {
		t.Fatalf("append first line: %v", err)
	}
	if writer.pendingLines != 1 {
		t.Fatalf("expected pending lines to be 1, got %d", writer.pendingLines)
	}
	if writer.w.Buffered() == 0 {
		t.Fatalf("expected buffered data after first append")
	}

	if err := writer.AppendLines([]string{"line 2"}); err != nil {
		t.Fatalf("append second line: %v", err)
	}
	if writer.pendingLines != 0 {
		t.Fatalf("expected pending lines to reset after flush, got %d", writer.pendingLines)
	}
	if writer.w.Buffered() != 0 {
		t.Fatalf("expected buffer to flush after reaching threshold")
	}

	if err := writer.Close(); err != nil {
		t.Fatalf("close writer: %v", err)
	}
}

func BenchmarkOutputFileWriterAppendLines(b *testing.B) {
	outputDir := b.TempDir()
	outputPath := filepath.Join(outputDir, "output.log")
	writer := &outputFileWriter{
		flushInterval:   time.Hour,
		maxPendingLines: 10_000,
		maxPendingBytes: 1 << 20,
		bufferSize:      256 * 1024,
	}
	if err := writer.Reset(outputPath); err != nil {
		b.Fatalf("reset output writer: %v", err)
	}
	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		if err := writer.AppendLines([]string{"line"}); err != nil {
			b.Fatalf("append line: %v", err)
		}
	}
	b.StopTimer()
	if err := writer.Close(); err != nil {
		b.Fatalf("close writer: %v", err)
	}
}
