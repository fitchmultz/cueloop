// Package pin provides tests for duplicate queue ID detection and repair.
package pin

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestDuplicateIDsDetectsCrossDuplicates(t *testing.T) {
	fixture := mustLocateFixtures(t)

	tmpDir := t.TempDir()
	queuePath := copyFixture(t, fixture.queue, filepath.Join(tmpDir, "implementation_queue.md"))
	donePath := copyFixture(t, fixture.done, filepath.Join(tmpDir, "implementation_done.md"))

	data, err := os.ReadFile(queuePath)
	if err != nil {
		t.Fatalf("read queue: %v", err)
	}
	updated := strings.Replace(string(data), "RQ-0001", "RQ-0005", 1)
	if updated == string(data) {
		t.Fatalf("failed to introduce duplicate ID in queue")
	}
	if err := os.WriteFile(queuePath, []byte(updated), 0o600); err != nil {
		t.Fatalf("write queue: %v", err)
	}

	report, err := DuplicateIDs(Files{
		QueuePath: queuePath,
		DonePath:  donePath,
	})
	if err != nil {
		t.Fatalf("DuplicateIDs failed: %v", err)
	}
	if len(report.Cross) != 1 || report.Cross[0] != "RQ-0005" {
		t.Fatalf("expected cross duplicate RQ-0005, got %#v", report.Cross)
	}
	if len(report.Fixable) != 1 || report.Fixable[0] != "RQ-0005" {
		t.Fatalf("expected fixable duplicate RQ-0005, got %#v", report.Fixable)
	}
}

func TestFixDuplicateQueueIDsRenumbersQueue(t *testing.T) {
	fixture := mustLocateFixtures(t)

	tmpDir := t.TempDir()
	queuePath := copyFixture(t, fixture.queue, filepath.Join(tmpDir, "implementation_queue.md"))
	donePath := copyFixture(t, fixture.done, filepath.Join(tmpDir, "implementation_done.md"))
	lookupPath := copyFixture(t, fixture.lookup, filepath.Join(tmpDir, "lookup_table.md"))
	readmePath := copyFixture(t, fixture.readme, filepath.Join(tmpDir, "README.md"))

	data, err := os.ReadFile(queuePath)
	if err != nil {
		t.Fatalf("read queue: %v", err)
	}
	updated := strings.Replace(string(data), "RQ-0001", "RQ-0005", 1)
	if updated == string(data) {
		t.Fatalf("failed to introduce duplicate ID in queue")
	}
	if err := os.WriteFile(queuePath, []byte(updated), 0o600); err != nil {
		t.Fatalf("write queue: %v", err)
	}

	result, err := FixDuplicateQueueIDs(Files{
		QueuePath:  queuePath,
		DonePath:   donePath,
		LookupPath: lookupPath,
		ReadmePath: readmePath,
	}, "")
	if err != nil {
		t.Fatalf("FixDuplicateQueueIDs failed: %v", err)
	}
	if len(result.Fixed) != 1 {
		t.Fatalf("expected 1 fix, got %#v", result.Fixed)
	}

	updatedQueue, err := os.ReadFile(queuePath)
	if err != nil {
		t.Fatalf("read updated queue: %v", err)
	}
	if strings.Contains(string(updatedQueue), "RQ-0005 [") {
		t.Fatalf("expected duplicate ID to be renumbered in queue")
	}
	if !strings.Contains(string(updatedQueue), "RQ-0006") {
		t.Fatalf("expected queue to include new ID RQ-0006")
	}

	if err := ValidatePin(Files{
		QueuePath:  queuePath,
		DonePath:   donePath,
		LookupPath: lookupPath,
		ReadmePath: readmePath,
	}); err != nil {
		t.Fatalf("ValidatePin failed after fix: %v", err)
	}
}

func TestFixDuplicateQueueIDsRejectsDoneDuplicates(t *testing.T) {
	fixture := mustLocateFixtures(t)

	tmpDir := t.TempDir()
	queuePath := copyFixture(t, fixture.queue, filepath.Join(tmpDir, "implementation_queue.md"))
	donePath := copyFixture(t, fixture.done, filepath.Join(tmpDir, "implementation_done.md"))

	doneData, err := os.ReadFile(donePath)
	if err != nil {
		t.Fatalf("read done: %v", err)
	}
	duplicate := "\n- [x] RQ-0005 [docs]: Another done entry. (README.md)\n  - Evidence: duplicate ID\n  - Plan: update manually\n"
	if err := os.WriteFile(donePath, append(doneData, []byte(duplicate)...), 0o600); err != nil {
		t.Fatalf("write done: %v", err)
	}

	_, err = FixDuplicateQueueIDs(Files{
		QueuePath: queuePath,
		DonePath:  donePath,
	}, "")
	if err == nil {
		t.Fatalf("expected error for done duplicates")
	}
	if !strings.Contains(err.Error(), "done log") {
		t.Fatalf("expected done log error, got %v", err)
	}
}
