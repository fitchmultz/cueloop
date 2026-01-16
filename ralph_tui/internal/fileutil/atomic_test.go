package fileutil

import (
	"errors"
	"os"
	"path/filepath"
	"runtime"
	"testing"
)

func TestWriteFileAtomicCreatesFileWithDataAndPerms(t *testing.T) {
	tmpDir := t.TempDir()
	path := filepath.Join(tmpDir, "config.txt")
	data := []byte("hello")
	perm := os.FileMode(0o640)

	if err := WriteFileAtomic(path, data, perm); err != nil {
		t.Fatalf("WriteFileAtomic returned error: %v", err)
	}

	got, err := os.ReadFile(path)
	if err != nil {
		t.Fatalf("ReadFile returned error: %v", err)
	}
	if string(got) != string(data) {
		t.Fatalf("expected data %q, got %q", data, got)
	}

	if runtime.GOOS != "windows" {
		info, err := os.Stat(path)
		if err != nil {
			t.Fatalf("Stat returned error: %v", err)
		}
		if info.Mode().Perm() != perm {
			t.Fatalf("expected perms %o, got %o", perm, info.Mode().Perm())
		}
	}
}

func TestWriteFileAtomicOverwritesExistingFile(t *testing.T) {
	tmpDir := t.TempDir()
	path := filepath.Join(tmpDir, "config.txt")

	if err := os.WriteFile(path, []byte("old"), 0o600); err != nil {
		t.Fatalf("WriteFile returned error: %v", err)
	}

	data := []byte("new")
	perm := os.FileMode(0o600)
	if err := WriteFileAtomic(path, data, perm); err != nil {
		t.Fatalf("WriteFileAtomic returned error: %v", err)
	}

	got, err := os.ReadFile(path)
	if err != nil {
		t.Fatalf("ReadFile returned error: %v", err)
	}
	if string(got) != string(data) {
		t.Fatalf("expected data %q, got %q", data, got)
	}
}

func TestWriteFileAtomicMissingDirectory(t *testing.T) {
	tmpDir := t.TempDir()
	path := filepath.Join(tmpDir, "missing", "config.txt")

	err := WriteFileAtomic(path, []byte("data"), 0o600)
	if err == nil {
		t.Fatal("expected error, got nil")
	}
	if !errors.Is(err, os.ErrNotExist) {
		t.Fatalf("expected not exist error, got %v", err)
	}
}
