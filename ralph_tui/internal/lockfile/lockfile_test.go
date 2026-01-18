package lockfile

import (
	"os"
	"path/filepath"
	"testing"
)

func TestAcquireNewLockAcquiresAndReleases(t *testing.T) {
	lockDir := filepath.Join(t.TempDir(), "lock")
	lock, err := Acquire(lockDir, AcquireOptions{})
	if err != nil {
		t.Fatalf("Acquire returned error: %v", err)
	}
	if lock == nil || !lock.Acquired() {
		t.Fatalf("expected lock to be acquired")
	}
	lock.Release()
	if _, err := os.Stat(lockDir); !os.IsNotExist(err) {
		t.Fatalf("expected lock dir to be removed, stat err: %v", err)
	}
}

func TestAcquireSameProcessWithoutAllowAncestorFails(t *testing.T) {
	lockDir := filepath.Join(t.TempDir(), "lock")
	if err := os.Mkdir(lockDir, 0o700); err != nil {
		t.Fatalf("failed to create lock dir: %v", err)
	}
	ownerPath := filepath.Join(lockDir, ownerFilename)
	if err := writeOwnerPID(ownerPath); err != nil {
		t.Fatalf("failed to write owner pid: %v", err)
	}

	if _, err := Acquire(lockDir, AcquireOptions{}); err == nil {
		t.Fatalf("expected Acquire to fail for same-process lock")
	}
}

func TestAcquireAllowAncestorWithSelfPIDReturnsNotAcquired(t *testing.T) {
	lockDir := filepath.Join(t.TempDir(), "lock")
	if err := os.Mkdir(lockDir, 0o700); err != nil {
		t.Fatalf("failed to create lock dir: %v", err)
	}
	ownerPath := filepath.Join(lockDir, ownerFilename)
	if err := writeOwnerPID(ownerPath); err != nil {
		t.Fatalf("failed to write owner pid: %v", err)
	}

	lock, err := Acquire(lockDir, AcquireOptions{AllowAncestor: true})
	if err != nil {
		t.Fatalf("Acquire returned error: %v", err)
	}
	if lock == nil || lock.Acquired() {
		t.Fatalf("expected lock to be present but not acquired")
	}
}

func TestAcquireStalePIDReclaimsLock(t *testing.T) {
	lockDir := filepath.Join(t.TempDir(), "lock")
	if err := os.Mkdir(lockDir, 0o700); err != nil {
		t.Fatalf("failed to create lock dir: %v", err)
	}
	ownerPath := filepath.Join(lockDir, ownerFilename)
	if err := os.WriteFile(ownerPath, []byte("-1"), 0o600); err != nil {
		t.Fatalf("failed to write owner pid: %v", err)
	}

	lock, err := Acquire(lockDir, AcquireOptions{})
	if err != nil {
		t.Fatalf("Acquire returned error: %v", err)
	}
	if lock == nil || !lock.Acquired() {
		t.Fatalf("expected lock to be acquired")
	}
}
