// Package loop provides tests for queue parsing and prompt policy output.
// Entrypoint: go test ./...
package loop

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestFirstUncheckedItemWithTags(t *testing.T) {
	content := "## Queue\n" +
		"- [ ] RQ-0001 [db]: First item. (a)\n" +
		"- [ ] RQ-0002 [ui]: Second item. (b)\n" +
		"\n## Blocked\n\n## Parking Lot\n"
	path := filepath.Join(t.TempDir(), "queue.md")
	if err := os.WriteFile(path, []byte(content), 0o600); err != nil {
		t.Fatalf("write: %v", err)
	}

	item, err := FirstUncheckedItem(path, []string{"ui"})
	if err != nil {
		t.Fatalf("FirstUncheckedItem failed: %v", err)
	}
	if item == nil || item.ID != "RQ-0002" {
		t.Fatalf("expected RQ-0002, got %#v", item)
	}
}

func TestFirstUncheckedItemWithTagsIgnoresInlineBrackets(t *testing.T) {
	content := "## Queue\n" +
		"- [ ] RQ-0001 [code]: Fix label [ui] in docs. (a)\n" +
		"- [ ] RQ-0002 [ui]: Second item. (b)\n" +
		"\n## Blocked\n\n## Parking Lot\n"
	path := filepath.Join(t.TempDir(), "queue.md")
	if err := os.WriteFile(path, []byte(content), 0o600); err != nil {
		t.Fatalf("write: %v", err)
	}

	item, err := FirstUncheckedItem(path, []string{"ui"})
	if err != nil {
		t.Fatalf("FirstUncheckedItem failed: %v", err)
	}
	if item == nil || item.ID != "RQ-0002" {
		t.Fatalf("expected RQ-0002, got %#v", item)
	}
}

func TestFirstUncheckedItemWithMetadataBlock(t *testing.T) {
	content := "## Queue\n" +
		"- [ ] RQ-0001 [code]: Metadata item. (a)\n" +
		"  - Evidence: Confirm metadata does not break parsing.\n" +
		"  - Plan: Keep metadata indented.\n" +
		"  - Notes: Add extra context.\n" +
		"    - Link: https://example.com/item\n" +
		"  ```yaml\n" +
		"  owner: ralph-team\n" +
		"  severity: low\n" +
		"  ```\n" +
		"\n## Blocked\n\n## Parking Lot\n"
	path := filepath.Join(t.TempDir(), "queue.md")
	if err := os.WriteFile(path, []byte(content), 0o600); err != nil {
		t.Fatalf("write: %v", err)
	}

	item, err := FirstUncheckedItem(path, []string{"code"})
	if err != nil {
		t.Fatalf("FirstUncheckedItem failed: %v", err)
	}
	if item == nil || item.ID != "RQ-0001" {
		t.Fatalf("expected RQ-0001, got %#v", item)
	}
}

func TestContextBuilderPolicyBlock(t *testing.T) {
	tests := []struct {
		name      string
		mandatory bool
		forced    bool
		expect    []string
	}{
		{
			name:      "optional when not mandatory",
			mandatory: false,
			forced:    false,
			expect:    []string{"OPTIONAL:"},
		},
		{
			name:      "mandatory when forced",
			mandatory: true,
			forced:    true,
			expect:    []string{"MANDATORY:", "Force context_builder is ENABLED"},
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			block := contextBuilderPolicyBlock("medium", tt.mandatory, tt.forced, "")
			for _, fragment := range tt.expect {
				if !strings.Contains(block, fragment) {
					t.Fatalf("expected %q in policy block, got:\n%s", fragment, block)
				}
			}
		})
	}
}

func TestContextBuilderPolicyBlockIncludesNote(t *testing.T) {
	note := "Auto reasoning effort target applied (P1 item): high."
	block := contextBuilderPolicyBlock("high", false, false, note)
	if !strings.Contains(block, note) {
		t.Fatalf("expected note in policy block, got:\n%s", block)
	}
}

func TestRunnerNormalizesRunnerInput(t *testing.T) {
	t.Setenv("RALPH_LOOP_SKIP_RUNNER_CHECK", "1")

	tests := []struct {
		name   string
		input  string
		expect string
	}{
		{name: "codex mixed case", input: " Codex ", expect: "codex"},
		{name: "opencode mixed case", input: " OPENcode ", expect: "opencode"},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			repoRoot := t.TempDir()
			pinDir := t.TempDir()
			runner, err := NewRunner(Options{
				RepoRoot: repoRoot,
				PinDir:   pinDir,
				Runner:   tt.input,
			})
			if err != nil {
				t.Fatalf("NewRunner failed: %v", err)
			}
			if runner.opts.Runner != tt.expect {
				t.Fatalf("expected runner %q, got %q", tt.expect, runner.opts.Runner)
			}
			if err := runner.verifyRunner(); err != nil {
				t.Fatalf("verifyRunner failed: %v", err)
			}
		})
	}
}
