// Package project provides heuristics for detecting repository project type.
package project

import (
	"io/fs"
	"path/filepath"
	"strings"
)

// DetectSummary captures counts used to infer project type.
type DetectSummary struct {
	CodeFiles int
	DocsFiles int
}

// DetectType infers project type from repository contents.
func DetectType(repoRoot string) (Type, DetectSummary, error) {
	var summary DetectSummary
	if strings.TrimSpace(repoRoot) == "" {
		return DefaultType(), summary, nil
	}

	skipDirs := map[string]struct{}{
		".git":         {},
		".ralph":       {},
		"node_modules": {},
		"vendor":       {},
		"dist":         {},
		"build":        {},
		".venv":        {},
		".idea":        {},
		".vscode":      {},
	}

	codeExts := map[string]struct{}{
		".go":    {},
		".py":    {},
		".ts":    {},
		".tsx":   {},
		".js":    {},
		".jsx":   {},
		".rs":    {},
		".java":  {},
		".rb":    {},
		".cs":    {},
		".cpp":   {},
		".c":     {},
		".h":     {},
		".hpp":   {},
		".swift": {},
		".kt":    {},
		".php":   {},
		".scala": {},
		".m":     {},
		".mm":    {},
		".lua":   {},
		".sql":   {},
		".sh":    {},
		".bash":  {},
		".zsh":   {},
	}
	docsExts := map[string]struct{}{
		".md":       {},
		".mdx":      {},
		".rst":      {},
		".txt":      {},
		".adoc":     {},
		".asciidoc": {},
	}

	err := filepath.WalkDir(repoRoot, func(path string, entry fs.DirEntry, walkErr error) error {
		if walkErr != nil {
			return walkErr
		}
		name := entry.Name()
		if entry.IsDir() {
			if _, ok := skipDirs[name]; ok {
				return fs.SkipDir
			}
			return nil
		}
		ext := strings.ToLower(filepath.Ext(name))
		if _, ok := codeExts[ext]; ok {
			summary.CodeFiles++
			return nil
		}
		if _, ok := docsExts[ext]; ok {
			summary.DocsFiles++
		}
		return nil
	})
	if err != nil {
		return DefaultType(), summary, err
	}

	if summary.DocsFiles > summary.CodeFiles {
		return TypeDocs, summary, nil
	}
	return TypeCode, summary, nil
}
