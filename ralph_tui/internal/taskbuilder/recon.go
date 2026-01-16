package taskbuilder

import (
	"context"
	"errors"
	"fmt"
	"io/fs"
	"path/filepath"
	"sort"
	"strings"

	"github.com/mitchfultz/ralph/ralph_tui/internal/loop"
	"github.com/mitchfultz/ralph/ralph_tui/internal/project"
)

// ReconResult captures repo summary data for queue evidence/plan.
type ReconResult struct {
	DetectedProjectType project.Type
	DetectSummary       project.DetectSummary

	TotalFiles     int
	ExtCounts      map[string]int
	FileScanLimit  int
	FileScanCapped bool

	GitBranch         string
	GitStatusSummary  string
	GitDirtyCount     int
	GitAheadCount     int
	LastCommitSummary string
}

// ReconOptions controls how recon is collected.
type ReconOptions struct {
	MaxFiles int
}

var errStopWalk = errors.New("stop walk")

// Recon gathers repo evidence (git + file composition).
func Recon(repoRoot string, opts ReconOptions) (ReconResult, error) {
	if strings.TrimSpace(repoRoot) == "" {
		return ReconResult{}, fmt.Errorf("repo root required")
	}

	result := ReconResult{
		ExtCounts: make(map[string]int),
	}

	detectedType, summary, err := project.DetectType(repoRoot)
	if err != nil {
		return ReconResult{}, err
	}
	result.DetectedProjectType = detectedType
	result.DetectSummary = summary

	maxFiles := opts.MaxFiles
	if maxFiles < 0 {
		maxFiles = 0
	}
	result.FileScanLimit = maxFiles

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

	fileCount := 0
	walkErr := filepath.WalkDir(repoRoot, func(path string, entry fs.DirEntry, walkErr error) error {
		if walkErr != nil {
			return walkErr
		}
		if entry.IsDir() {
			if _, ok := skipDirs[entry.Name()]; ok {
				return fs.SkipDir
			}
			return nil
		}
		fileCount++
		ext := strings.ToLower(filepath.Ext(entry.Name()))
		if ext == "" {
			ext = "(none)"
		}
		result.ExtCounts[ext]++
		if maxFiles > 0 && fileCount >= maxFiles {
			result.FileScanCapped = true
			return errStopWalk
		}
		return nil
	})
	if walkErr != nil && !errors.Is(walkErr, errStopWalk) {
		return ReconResult{}, walkErr
	}
	result.TotalFiles = fileCount

	ctx := context.Background()
	branch, err := loop.CurrentBranch(ctx, repoRoot)
	if err == nil {
		result.GitBranch = branch
	}

	status, err := loop.StatusDetails(ctx, repoRoot)
	if err != nil {
		result.GitStatusSummary = fmt.Sprintf("unavailable (%v)", err)
	} else {
		tracked := len(status.TrackedEntries())
		untracked := len(status.UntrackedEntries())
		result.GitDirtyCount = len(status.Entries)
		if result.GitDirtyCount == 0 {
			result.GitStatusSummary = "clean"
		} else {
			result.GitStatusSummary = fmt.Sprintf("dirty: %d (tracked %d, untracked %d)", result.GitDirtyCount, tracked, untracked)
		}
	}

	ahead, err := loop.AheadCount(ctx, repoRoot)
	if err == nil {
		result.GitAheadCount = ahead
	}

	lastCommit, err := loop.LastCommitSummary(ctx, repoRoot)
	if err == nil {
		result.LastCommitSummary = lastCommit
	}

	return result, nil
}

func formatExtSummary(counts map[string]int, limit int) string {
	if len(counts) == 0 {
		return ""
	}
	type pair struct {
		ext   string
		count int
	}
	pairs := make([]pair, 0, len(counts))
	for ext, count := range counts {
		pairs = append(pairs, pair{ext: ext, count: count})
	}
	sort.Slice(pairs, func(i, j int) bool {
		if pairs[i].count == pairs[j].count {
			return pairs[i].ext < pairs[j].ext
		}
		return pairs[i].count > pairs[j].count
	})
	if limit > 0 && len(pairs) > limit {
		pairs = pairs[:limit]
	}
	parts := make([]string, 0, len(pairs))
	for _, pair := range pairs {
		parts = append(parts, fmt.Sprintf("%s=%d", pair.ext, pair.count))
	}
	return strings.Join(parts, ", ")
}
