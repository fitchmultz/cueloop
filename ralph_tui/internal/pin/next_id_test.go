// Package pin provides tests for queue ID allocation helpers.
package pin

import "testing"

func TestNextQueueIDFromFixtures(t *testing.T) {
	fixture := mustLocateFixtures(t)
	files := Files{
		QueuePath: fixture.queue,
		DonePath:  fixture.done,
	}

	nextID, err := NextQueueID(files, "RQ")
	if err != nil {
		t.Fatalf("NextQueueID failed: %v", err)
	}
	if nextID != "RQ-0006" {
		t.Fatalf("expected RQ-0006, got %s", nextID)
	}
}
