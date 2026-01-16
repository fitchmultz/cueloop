package project

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestDetectType(t *testing.T) {
	cases := []struct {
		name     string
		files    []string
		repoRoot string
		wantType Type
		wantCode int
		wantDocs int
		wantErr  bool
	}{
		{
			name:     "code-heavy",
			files:    []string{"main.go", "app.py", "web/index.ts", "README.md"},
			wantType: TypeCode,
			wantCode: 3,
			wantDocs: 1,
		},
		{
			name:     "docs-heavy",
			files:    []string{"README.md", "docs/guide.md", "notes.txt", "app.go"},
			wantType: TypeDocs,
			wantCode: 1,
			wantDocs: 3,
		},
		{
			name:     "ambiguous-equal",
			files:    []string{"main.go", "README.md"},
			wantType: TypeCode,
			wantCode: 1,
			wantDocs: 1,
		},
		{
			name:     "empty",
			files:    nil,
			wantType: TypeCode,
			wantCode: 0,
			wantDocs: 0,
		},
		{
			name:     "blank-repo-root",
			repoRoot: "   ",
			wantType: TypeCode,
			wantCode: 0,
			wantDocs: 0,
		},
	}

	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			repoRoot := tc.repoRoot
			blankRoot := repoRoot != "" && strings.TrimSpace(repoRoot) == ""
			if repoRoot == "" {
				repoRoot = t.TempDir()
			}

			if !blankRoot {
				if err := writeFiles(repoRoot, tc.files); err != nil {
					t.Fatalf("writeFiles: %v", err)
				}
			}

			gotType, summary, err := DetectType(repoRoot)
			if tc.wantErr {
				if err == nil {
					t.Fatalf("expected error, got nil")
				}
				return
			}
			if err != nil {
				t.Fatalf("DetectType error: %v", err)
			}
			if gotType != tc.wantType {
				t.Fatalf("type mismatch: got %q want %q", gotType, tc.wantType)
			}
			if summary.CodeFiles != tc.wantCode || summary.DocsFiles != tc.wantDocs {
				t.Fatalf("summary mismatch: got code=%d docs=%d want code=%d docs=%d", summary.CodeFiles, summary.DocsFiles, tc.wantCode, tc.wantDocs)
			}
		})
	}
}

func writeFiles(root string, files []string) error {
	for _, file := range files {
		path := filepath.Join(root, file)
		if err := os.MkdirAll(filepath.Dir(path), 0o755); err != nil {
			return err
		}
		if err := os.WriteFile(path, []byte("test"), 0o644); err != nil {
			return err
		}
	}
	return nil
}
