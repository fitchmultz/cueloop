// Package tui provides repo status sampling for the dashboard.
// Entrypoint: repoStatusCmd.
package tui

import (
	"context"
	"errors"
	"fmt"
	"strings"

	tea "github.com/charmbracelet/bubbletea"
	"github.com/mitchfultz/ralph/ralph_tui/internal/loop"
)

type repoStatusSnapshot struct {
	Branch             string
	BranchNote         string
	ShortHead          string
	ShortHeadNote      string
	StatusSummary      string
	StatusSummaryNote  string
	DirtyCount         int
	AheadCount         int
	AheadNote          string
	LastCommit         string
	LastCommitNote     string
	LastCommitStat     string
	LastCommitStatNote string
}

type repoStatusMsg struct {
	status repoStatusSnapshot
	err    error
}

func repoStatusCmd(repoRoot string) tea.Cmd {
	return func() tea.Msg {
		status, err := fetchRepoStatus(context.Background(), repoRoot)
		return repoStatusMsg{status: status, err: err}
	}
}

func fetchRepoStatus(ctx context.Context, repoRoot string) (repoStatusSnapshot, error) {
	snapshot := repoStatusSnapshot{}
	if strings.TrimSpace(repoRoot) == "" {
		return snapshot, fmt.Errorf("repo root unavailable")
	}

	branch, err := loop.CurrentBranch(ctx, repoRoot)
	if err != nil {
		snapshot.BranchNote = repoNoteFromError(err, "unavailable")
	} else {
		snapshot.Branch = branch
	}

	shortHead, err := loop.ShortHeadSHA(ctx, repoRoot)
	if err != nil {
		snapshot.ShortHeadNote = repoNoteFromError(err, "unavailable")
	} else {
		snapshot.ShortHead = shortHead
	}

	summary, err := loop.StatusSummary(ctx, repoRoot)
	if err != nil {
		snapshot.StatusSummaryNote = repoNoteFromError(err, "unavailable")
	} else {
		statusLine, dirtyCount := summarizeStatusSummary(summary)
		snapshot.StatusSummary = statusLine
		snapshot.DirtyCount = dirtyCount
	}

	ahead, err := loop.AheadCount(ctx, repoRoot)
	if err != nil {
		snapshot.AheadNote = repoNoteFromError(err, "unavailable")
	} else {
		snapshot.AheadCount = ahead
	}

	lastCommit, err := loop.LastCommitSummary(ctx, repoRoot)
	if err != nil {
		snapshot.LastCommitNote = repoNoteFromError(err, "unavailable")
	} else {
		snapshot.LastCommit = lastCommit
	}

	lastStat, err := loop.LastCommitDiffStat(ctx, repoRoot)
	if err != nil {
		snapshot.LastCommitStatNote = repoNoteFromError(err, "unavailable")
	} else {
		snapshot.LastCommitStat = lastStat
	}

	return snapshot, nil
}

func summarizeStatusSummary(summary string) (string, int) {
	lines := strings.Split(strings.TrimSpace(summary), "\n")
	if len(lines) == 0 {
		return "", 0
	}
	statusLine := strings.TrimSpace(lines[0])
	dirtyCount := 0
	for _, line := range lines[1:] {
		if strings.TrimSpace(line) != "" {
			dirtyCount++
		}
	}
	return statusLine, dirtyCount
}

func repoNoteFromError(err error, fallback string) string {
	if err == nil {
		return ""
	}
	message := strings.ToLower(gitErrorMessage(err))
	switch {
	case strings.Contains(message, "no upstream"):
		return "no upstream"
	case strings.Contains(message, "does not have any commits yet"):
		return "no commits"
	case strings.Contains(message, "bad revision"):
		return "no commits"
	case strings.Contains(message, "unknown revision"):
		return "no commits"
	case strings.Contains(message, "not a git repository"):
		return "not a git repo"
	case strings.Contains(message, "executable file not found"):
		return "git missing"
	default:
		return fallback
	}
}

func gitErrorMessage(err error) string {
	if err == nil {
		return ""
	}
	var gitErr *loop.GitCommandError
	if errors.As(err, &gitErr) {
		if strings.TrimSpace(gitErr.Stderr) != "" {
			return gitErr.Stderr
		}
	}
	return err.Error()
}
