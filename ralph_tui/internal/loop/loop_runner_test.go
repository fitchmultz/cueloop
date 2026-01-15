// Package loop provides tests for loop behaviors.
// Entrypoint: go test ./...
package loop

import (
	"context"
	"os"
	"path/filepath"
	"strings"
	"testing"

	"github.com/mitchfultz/ralph/ralph_tui/internal/redaction"
)

type bufferLogger struct {
	lines []string
}

func TestRunnerEffortAutoUsesQueuePriority(t *testing.T) {
	repoRoot := t.TempDir()
	pinDir := filepath.Join(repoRoot, ".ralph", "pin")
	if err := os.MkdirAll(pinDir, 0o700); err != nil {
		t.Fatalf("mkdir: %v", err)
	}

	queue := filepath.Join(pinDir, "implementation_queue.md")
	done := filepath.Join(pinDir, "implementation_done.md")
	lookup := filepath.Join(pinDir, "lookup_table.md")
	readme := filepath.Join(pinDir, "README.md")

	writeFile(t, queue, "## Queue\n- [ ] RQ-0001 [P1] [code]: test item. (x)\n## Blocked\n## Parking Lot\n")
	writeFile(t, done, "## Done\n")
	writeFile(t, lookup, "")
	writeFile(t, readme, "")

	logger := &bufferLogger{}
	runner, err := NewRunner(Options{
		RepoRoot:          repoRoot,
		PinDir:            pinDir,
		PromptPath:        "",
		SupervisorPrompt:  "",
		Runner:            "codex",
		RunnerArgs:        []string{},
		ReasoningEffort:   "auto",
		SleepSeconds:      0,
		MaxIterations:     1,
		MaxStalled:        0,
		MaxRepairAttempts: 0,
		OnlyTags:          []string{},
		Once:              true,
		RequireMain:       false,
		AutoCommit:        false,
		AutoPush:          false,
		RedactionMode:     redaction.ModeSecretsOnly,
		Logger:            logger,
	})
	if err != nil {
		t.Fatalf("NewRunner failed: %v", err)
	}

	_ = runner.Run(context.Background())

	if runner.effectiveEffort != "high" {
		t.Fatalf("expected effective effort high, got %q", runner.effectiveEffort)
	}
}

func (b *bufferLogger) WriteLine(line string) {
	b.lines = append(b.lines, line)
}

func TestRunnerStopsOnEmptyQueue(t *testing.T) {
	repoRoot := t.TempDir()
	pinDir := filepath.Join(repoRoot, ".ralph", "pin")
	if err := os.MkdirAll(pinDir, 0o700); err != nil {
		t.Fatalf("mkdir: %v", err)
	}

	queue := filepath.Join(pinDir, "implementation_queue.md")
	done := filepath.Join(pinDir, "implementation_done.md")
	lookup := filepath.Join(pinDir, "lookup_table.md")
	readme := filepath.Join(pinDir, "README.md")

	writeFile(t, queue, "## Queue\n\n## Blocked\n\n## Parking Lot\n")
	writeFile(t, done, "## Done\n")
	writeFile(t, lookup, "")
	writeFile(t, readme, "")

	logger := &bufferLogger{}
	runner, err := NewRunner(Options{
		RepoRoot:          repoRoot,
		PinDir:            pinDir,
		PromptPath:        "",
		SupervisorPrompt:  "",
		Runner:            "codex",
		ReasoningEffort:   "auto",
		SleepSeconds:      0,
		MaxIterations:     0,
		MaxStalled:        0,
		MaxRepairAttempts: 0,
		OnlyTags:          []string{},
		Once:              true,
		RequireMain:       false,
		AutoCommit:        false,
		AutoPush:          false,
		RedactionMode:     redaction.ModeSecretsOnly,
		Logger:            logger,
	})
	if err != nil {
		t.Fatalf("NewRunner failed: %v", err)
	}

	if err := runner.Run(context.Background()); err != nil {
		t.Fatalf("Run failed: %v", err)
	}
}

func TestRunnerLogsGitErrors(t *testing.T) {
	requireTool(t, "git")
	repoRoot := t.TempDir()
	pinDir := filepath.Join(repoRoot, ".ralph", "pin")
	if err := os.MkdirAll(pinDir, 0o700); err != nil {
		t.Fatalf("mkdir: %v", err)
	}

	queue := filepath.Join(pinDir, "implementation_queue.md")
	done := filepath.Join(pinDir, "implementation_done.md")
	lookup := filepath.Join(pinDir, "lookup_table.md")
	readme := filepath.Join(pinDir, "README.md")

	writeFile(t, queue, "## Queue\n- [ ] RQ-0001 [code]: test item. (x)\n## Blocked\n## Parking Lot\n")
	writeFile(t, done, "## Done\n")
	writeFile(t, lookup, "")
	writeFile(t, readme, "")

	logger := &bufferLogger{}
	runner, err := NewRunner(Options{
		RepoRoot:          repoRoot,
		PinDir:            pinDir,
		PromptPath:        "",
		SupervisorPrompt:  "",
		Runner:            "codex",
		RunnerArgs:        []string{},
		ReasoningEffort:   "auto",
		SleepSeconds:      0,
		MaxIterations:     1,
		MaxStalled:        0,
		MaxRepairAttempts: 0,
		OnlyTags:          []string{},
		Once:              true,
		RequireMain:       true,
		AutoCommit:        false,
		AutoPush:          false,
		RedactionMode:     redaction.ModeSecretsOnly,
		Logger:            logger,
	})
	if err != nil {
		t.Fatalf("NewRunner failed: %v", err)
	}

	if err := runner.Run(context.Background()); err == nil {
		t.Fatal("expected run error for non-git repo")
	}

	found := false
	for _, line := range logger.lines {
		if strings.Contains(line, "git command failed") {
			found = true
			break
		}
	}
	if !found {
		t.Fatalf("expected git error details in logs, got: %v", logger.lines)
	}
}

func writeFile(t *testing.T, path string, content string) {
	t.Helper()
	if err := os.WriteFile(path, []byte(content), 0o600); err != nil {
		t.Fatalf("write file: %v", err)
	}
}
