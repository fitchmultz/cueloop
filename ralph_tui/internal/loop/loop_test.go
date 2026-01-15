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
			block := contextBuilderPolicyBlock("medium", tt.mandatory, tt.forced)
			for _, fragment := range tt.expect {
				if !strings.Contains(block, fragment) {
					t.Fatalf("expected %q in policy block, got:\n%s", fragment, block)
				}
			}
		})
	}
}
