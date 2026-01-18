// Package paths verifies repo/root resolution and config path derivation.
package paths

import (
	"os"
	"path/filepath"
	"testing"
)

type resolveCase struct {
	name  string
	setup func(t *testing.T) (cwd string, repoRoot string)
}

func TestResolve(t *testing.T) {
	cases := []resolveCase{
		{
			name: "repo-root",
			setup: func(t *testing.T) (string, string) {
				root := t.TempDir()
				ensureGitDir(t, root)
				return root, root
			},
		},
		{
			name: "nested-dir",
			setup: func(t *testing.T) (string, string) {
				root := t.TempDir()
				ensureGitDir(t, root)
				nested := filepath.Join(root, "a", "b")
				if err := os.MkdirAll(nested, 0o755); err != nil {
					t.Fatalf("create nested dir: %v", err)
				}
				return nested, root
			},
		},
		{
			name: "no-repo",
			setup: func(t *testing.T) (string, string) {
				root := t.TempDir()
				return root, root
			},
		},
	}

	for _, tc := range cases {
		tc := tc
		t.Run(tc.name, func(t *testing.T) {
			cwd, repoRoot := tc.setup(t)

			locs, err := Resolve(cwd)
			if err != nil {
				t.Fatalf("Resolve error: %v", err)
			}

			if locs.CWD != cwd {
				t.Fatalf("CWD = %q, want %q", locs.CWD, cwd)
			}
			if locs.RepoRoot != repoRoot {
				t.Fatalf("RepoRoot = %q, want %q", locs.RepoRoot, repoRoot)
			}
			if locs.RepoConfigPath != filepath.Join(repoRoot, ".ralph", "ralph.json") {
				t.Fatalf("RepoConfigPath = %q", locs.RepoConfigPath)
			}
			if locs.HomeDir != "" && locs.GlobalConfigPath != filepath.Join(locs.HomeDir, ".ralph", "ralph.json") {
				t.Fatalf("GlobalConfigPath = %q", locs.GlobalConfigPath)
			}
		})
	}
}

func ensureGitDir(t *testing.T, root string) {
	gitDir := filepath.Join(root, ".git")
	if err := os.MkdirAll(gitDir, 0o755); err != nil {
		t.Fatalf("create .git dir: %v", err)
	}
}
