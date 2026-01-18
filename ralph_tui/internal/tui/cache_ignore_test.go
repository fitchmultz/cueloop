// Package tui provides tests for cache and log ignore behavior.
package tui

import (
	"path/filepath"
	"strings"
	"testing"

	"github.com/mitchfultz/ralph/ralph_tui/internal/config"
)

func TestCacheOutputsIgnoredByGit(t *testing.T) {
	t.Parallel()

	ensureGit(t)
	repoRoot := t.TempDir()
	runGit(t, repoRoot, "init", "-b", "main")
	runGit(t, repoRoot, "config", "user.email", "test@example.com")
	runGit(t, repoRoot, "config", "user.name", "Test User")

	writeTestFile(t, filepath.Join(repoRoot, ".gitignore"), ".ralph/cache/\n")
	writeTestFile(t, filepath.Join(repoRoot, ".repo_ignore"), ".ralph/cache/\n")
	runGit(t, repoRoot, "add", ".gitignore", ".repo_ignore")
	runGit(t, repoRoot, "commit", "-m", "init")

	cacheDir := filepath.Join(repoRoot, ".ralph", "cache")
	base, err := config.DefaultConfig()
	if err != nil {
		t.Fatalf("default config: %v", err)
	}
	base.Paths.CacheDir = cacheDir

	logger, err := newTUILogger(base)
	if err != nil {
		t.Fatalf("new logger: %v", err)
	}
	logger.Info("test log entry", nil)
	if err := logger.Close(); err != nil {
		t.Fatalf("close logger: %v", err)
	}

	loopWriter := &outputFileWriter{}
	if err := loopWriter.Reset(filepath.Join(cacheDir, "loop_output.log")); err != nil {
		t.Fatalf("reset loop writer: %v", err)
	}
	if err := loopWriter.AppendLines([]string{"loop output"}); err != nil {
		t.Fatalf("append loop output: %v", err)
	}
	if err := loopWriter.Close(); err != nil {
		t.Fatalf("close loop writer: %v", err)
	}

	specsWriter := &outputFileWriter{}
	if err := specsWriter.Reset(filepath.Join(cacheDir, "specs_output.log")); err != nil {
		t.Fatalf("reset specs writer: %v", err)
	}
	if err := specsWriter.AppendLines([]string{"specs output"}); err != nil {
		t.Fatalf("append specs output: %v", err)
	}
	if err := specsWriter.Close(); err != nil {
		t.Fatalf("close specs writer: %v", err)
	}

	status := strings.TrimSpace(runGit(t, repoRoot, "status", "--porcelain"))
	if status != "" {
		t.Fatalf("expected clean git status, got: %s", status)
	}
}
