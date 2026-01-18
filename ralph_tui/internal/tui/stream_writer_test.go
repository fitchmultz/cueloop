// Package tui provides tests for stream writer behavior.
package tui

import "testing"

type captureSink struct {
	lines []string
}

func (c *captureSink) PushLine(line string) {
	c.lines = append(c.lines, line)
}

func TestStreamWriterHonorsMaxBufferedBytes(t *testing.T) {
	sink := &captureSink{}
	writer := newStreamWriter(sink, 4)

	if _, err := writer.Write([]byte("abcde")); err != nil {
		t.Fatalf("write: %v", err)
	}

	if len(sink.lines) != 1 {
		t.Fatalf("expected 1 line, got %d", len(sink.lines))
	}
	if sink.lines[0] != "abcde" {
		t.Fatalf("unexpected line: %q", sink.lines[0])
	}
}
