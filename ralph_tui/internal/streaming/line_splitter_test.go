// Package streaming provides tests for line splitting behavior.
package streaming

import "testing"

type lineCollector struct {
	lines []string
}

func (c *lineCollector) emit(line string) {
	c.lines = append(c.lines, line)
}

func TestLineSplitterSplitsOnNewline(t *testing.T) {
	collector := &lineCollector{}
	splitter := NewLineSplitter(0)

	splitter.Write([]byte("step 1\nstep 2\n"), collector.emit)

	if len(collector.lines) != 2 {
		t.Fatalf("expected 2 lines, got %d", len(collector.lines))
	}
	if collector.lines[0] != "step 1" || collector.lines[1] != "step 2" {
		t.Fatalf("unexpected lines: %#v", collector.lines)
	}
}

func TestLineSplitterSplitsOnCarriageReturn(t *testing.T) {
	collector := &lineCollector{}
	splitter := NewLineSplitter(0)

	splitter.Write([]byte("step 1\rstep 2\r"), collector.emit)

	if len(collector.lines) != 2 {
		t.Fatalf("expected 2 lines, got %d", len(collector.lines))
	}
	if collector.lines[0] != "step 1" || collector.lines[1] != "step 2" {
		t.Fatalf("unexpected lines: %#v", collector.lines)
	}
}

func TestLineSplitterCRLFAcrossChunksNoEmptyLine(t *testing.T) {
	collector := &lineCollector{}
	splitter := NewLineSplitter(0)

	splitter.Write([]byte("alpha\r"), collector.emit)
	splitter.Write([]byte("\nbeta\n"), collector.emit)

	if len(collector.lines) != 2 {
		t.Fatalf("expected 2 lines, got %d", len(collector.lines))
	}
	if collector.lines[0] != "alpha" || collector.lines[1] != "beta" {
		t.Fatalf("unexpected lines: %#v", collector.lines)
	}
}

func TestLineSplitterDoesNotFlushAtExactLimitThenNewline(t *testing.T) {
	collector := &lineCollector{}
	splitter := NewLineSplitter(5)

	splitter.Write([]byte("abcde"), collector.emit)
	if len(collector.lines) != 0 {
		t.Fatalf("expected no lines before newline, got %d", len(collector.lines))
	}

	splitter.Write([]byte("\n"), collector.emit)
	if len(collector.lines) != 1 {
		t.Fatalf("expected 1 line after newline, got %d", len(collector.lines))
	}
	if collector.lines[0] != "abcde" {
		t.Fatalf("unexpected line: %q", collector.lines[0])
	}
}

func TestLineSplitterExceedsLimitTriggersPartialFlush(t *testing.T) {
	collector := &lineCollector{}
	splitter := NewLineSplitter(5)

	splitter.Write([]byte("abcdef"), collector.emit)

	if len(collector.lines) != 1 {
		t.Fatalf("expected 1 line, got %d", len(collector.lines))
	}
	if collector.lines[0] != "abcdef" {
		t.Fatalf("unexpected line: %q", collector.lines[0])
	}
}

func TestLineSplitterFlushEmitsPartialLine(t *testing.T) {
	collector := &lineCollector{}
	splitter := NewLineSplitter(0)

	splitter.Write([]byte("partial"), collector.emit)
	splitter.Flush(collector.emit)

	if len(collector.lines) != 1 {
		t.Fatalf("expected 1 line, got %d", len(collector.lines))
	}
	if collector.lines[0] != "partial" {
		t.Fatalf("unexpected line: %q", collector.lines[0])
	}
}
