// Package ci validates local CI guardrails from repo metadata.
package ci

import (
	"bufio"
	"os"
	"path/filepath"
	"runtime"
	"strings"
	"testing"
)

func TestMakefileCIRunsPinValidate(t *testing.T) {
	root := findRepoRoot(t)
	makefilePath := filepath.Join(root, "Makefile")
	content, err := os.ReadFile(makefilePath)
	if err != nil {
		t.Fatalf("read Makefile: %v", err)
	}
	text := string(content)

	if !strings.Contains(text, "\npin-validate:") && !strings.HasPrefix(text, "pin-validate:") {
		t.Fatalf("expected Makefile to define pin-validate target")
	}

	deps := extractMakeTargetDeps(text, "ci")
	if len(deps) == 0 {
		t.Fatalf("expected Makefile to define ci target dependencies")
	}
	if !contains(deps, "pin-validate") {
		t.Fatalf("expected make ci to include pin-validate, got %v", deps)
	}
}

func findRepoRoot(t *testing.T) string {
	t.Helper()
	_, file, _, ok := runtime.Caller(0)
	if !ok {
		t.Fatalf("unable to resolve test path")
	}

	dir := filepath.Dir(file)
	for {
		if _, err := os.Stat(filepath.Join(dir, "Makefile")); err == nil {
			return dir
		}
		parent := filepath.Dir(dir)
		if parent == dir {
			break
		}
		dir = parent
	}

	t.Fatalf("unable to locate repo root containing Makefile")
	return ""
}

func extractMakeTargetDeps(content string, target string) []string {
	scanner := bufio.NewScanner(strings.NewReader(content))
	needle := target + ":"
	deps := make([]string, 0)
	inTarget := false

	for scanner.Scan() {
		line := scanner.Text()
		trimmed := strings.TrimSpace(line)
		if strings.HasPrefix(trimmed, needle) {
			inTarget = true
			rest := strings.TrimSpace(strings.TrimPrefix(trimmed, needle))
			deps = append(deps, strings.Fields(strings.TrimSuffix(rest, "\\"))...)
			if !strings.HasSuffix(trimmed, "\\") {
				inTarget = false
			}
			continue
		}
		if inTarget {
			deps = append(deps, strings.Fields(strings.TrimSuffix(trimmed, "\\"))...)
			if !strings.HasSuffix(trimmed, "\\") {
				inTarget = false
			}
		}
	}

	return deps
}

func contains(items []string, target string) bool {
	for _, item := range items {
		if item == target {
			return true
		}
	}
	return false
}
