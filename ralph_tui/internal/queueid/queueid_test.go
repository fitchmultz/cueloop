// Package queueid provides tests for queue ID parsing helpers.
package queueid

import "testing"

func TestParseQueueID(t *testing.T) {
	prefix, number, ok := Parse("RQ-0123")
	if !ok {
		t.Fatalf("expected Parse to succeed")
	}
	if prefix != "RQ" || number != 123 {
		t.Fatalf("unexpected parse: %s %d", prefix, number)
	}

	if _, _, ok := Parse("rq-0123"); ok {
		t.Fatalf("expected Parse to reject lowercase prefix")
	}
	if _, _, ok := Parse("RQ-123"); ok {
		t.Fatalf("expected Parse to reject non-4-digit IDs")
	}
	if _, _, ok := Parse("RQ-0123a"); ok {
		t.Fatalf("expected Parse to reject invalid suffix")
	}
}

func TestNextQueueID(t *testing.T) {
	next, err := Next("RQ", []string{"RQ-0001", "RQ-0020", "AB-0003", "RQ-0010"})
	if err != nil {
		t.Fatalf("Next returned error: %v", err)
	}
	if next != "RQ-0021" {
		t.Fatalf("expected next ID RQ-0021, got %s", next)
	}
}

func TestNextQueueIDRequiresPrefix(t *testing.T) {
	if _, err := Next("", []string{"RQ-0001"}); err == nil {
		t.Fatalf("expected error for missing prefix")
	}
}
