// Package tui provides test helpers for hermetic TUI model construction.
package tui

import (
	"os"
	"path/filepath"
	"strings"
	"testing"

	"github.com/mitchfultz/ralph/ralph_tui/internal/config"
	"github.com/mitchfultz/ralph/ralph_tui/internal/paths"
)

func newHermeticModel(t *testing.T) (model, paths.Locations, config.Config) {
	t.Helper()

	repoRoot := t.TempDir()
	pinDir := filepath.Join(repoRoot, ".ralph", "pin")
	if err := os.MkdirAll(pinDir, 0o755); err != nil {
		t.Fatalf("create pin dir: %v", err)
	}

	queueContent := strings.Join([]string{
		"## Queue",
		"- [ ] RQ-0001 [ui]: Sample task (tui)",
		"  - Evidence: test fixture",
		"  - Plan: test fixture",
		"",
		"## Blocked",
		"",
		"## Parking Lot",
		"",
	}, "\n")
	writeTestFile(t, filepath.Join(pinDir, "implementation_queue.md"), queueContent)
	writeTestFile(t, filepath.Join(pinDir, "implementation_done.md"), "## Done\n")
	writeTestFile(t, filepath.Join(pinDir, "lookup_table.md"), "## Lookup\n")
	writeTestFile(t, filepath.Join(pinDir, "README.md"), "Ralph pin fixtures.\n")
	writeTestFile(t, filepath.Join(pinDir, "specs_builder.md"), "Use AGENTS.md for instructions.\n\n{{INTERACTIVE_INSTRUCTIONS}}\n\n{{INNOVATE_INSTRUCTIONS}}\n")

	base, err := config.DefaultConfig()
	if err != nil {
		t.Fatalf("default config: %v", err)
	}
	cfg := config.ResolvePaths(base, repoRoot)
	if err := cfg.Validate(); err != nil {
		t.Fatalf("validate config: %v", err)
	}

	locs := paths.Locations{
		CWD:              repoRoot,
		RepoRoot:         repoRoot,
		HomeDir:          repoRoot,
		GlobalConfigPath: "",
		RepoConfigPath:   "",
	}

	return newModel(cfg, locs), locs, cfg
}

func writeTestFile(t *testing.T, path string, content string) {
	t.Helper()
	if err := os.WriteFile(path, []byte(content), 0o644); err != nil {
		t.Fatalf("write %s: %v", path, err)
	}
}
