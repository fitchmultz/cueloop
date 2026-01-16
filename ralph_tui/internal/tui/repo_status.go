// Package tui provides repo status sampling for the dashboard.
// Entrypoint: repoStatusCmd.
package tui

import (
	"context"
	"errors"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"sync"
	"time"

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

type repoStatusResult struct {
	Snapshot       repoStatusSnapshot
	Err            error
	SampledAt      time.Time
	FromCache      bool
	Throttled      bool
	ThrottleReason string // cooldown | in-flight | error-cooldown
	NextAllowedAt  time.Time
}

type repoStatusMsg struct {
	result repoStatusResult
}

func repoStatusCmd(ctx context.Context, sampler *RepoStatusSampler, force bool) tea.Cmd {
	return func() tea.Msg {
		if sampler == nil {
			return repoStatusMsg{result: repoStatusResult{Err: fmt.Errorf("repo status sampler not configured")}}
		}
		return repoStatusMsg{result: sampler.Sample(ctx, force)}
	}
}

type RepoStatusSamplerOptions struct {
	Cooldown           time.Duration
	NotGitRepoCooldown time.Duration
	GitMissingCooldown time.Duration
	Logger             *tuiLogger
	Fetch              func(context.Context, string) (repoStatusSnapshot, error)
	TimeNow            func() time.Time
	LookPath           func(string) (string, error)
}

type RepoStatusSampler struct {
	repoRoot string

	cooldown           time.Duration
	notGitRepoCooldown time.Duration
	gitMissingCooldown time.Duration

	fetch    func(context.Context, string) (repoStatusSnapshot, error)
	now      func() time.Time
	lookPath func(string) (string, error)

	mu            sync.Mutex
	last          repoStatusResult
	nextAllowedAt time.Time

	lastHeadStamp fileStamp
	lastIdxStamp  fileStamp

	inFlight bool
	logger   *tuiLogger
}

func NewRepoStatusSampler(repoRoot string, opts RepoStatusSamplerOptions) *RepoStatusSampler {
	cooldown := opts.Cooldown
	if cooldown <= 0 {
		cooldown = 30 * time.Second
	}
	notGitCooldown := opts.NotGitRepoCooldown
	if notGitCooldown <= 0 {
		notGitCooldown = 5 * time.Minute
	}
	gitMissingCooldown := opts.GitMissingCooldown
	if gitMissingCooldown <= 0 {
		gitMissingCooldown = 5 * time.Minute
	}
	fetch := opts.Fetch
	if fetch == nil {
		fetch = fetchRepoStatus
	}
	now := opts.TimeNow
	if now == nil {
		now = time.Now
	}
	lookPath := opts.LookPath
	if lookPath == nil {
		lookPath = exec.LookPath
	}
	return &RepoStatusSampler{
		repoRoot:           repoRoot,
		cooldown:           cooldown,
		notGitRepoCooldown: notGitCooldown,
		gitMissingCooldown: gitMissingCooldown,
		fetch:              fetch,
		now:                now,
		lookPath:           lookPath,
		last:               repoStatusResult{},
		nextAllowedAt:      time.Time{},
		lastHeadStamp:      fileStamp{},
		lastIdxStamp:       fileStamp{},
		logger:             opts.Logger,
	}
}

func (s *RepoStatusSampler) SetLogger(logger *tuiLogger) {
	if s == nil {
		return
	}
	s.mu.Lock()
	defer s.mu.Unlock()
	s.logger = logger
}

func (s *RepoStatusSampler) Sample(ctx context.Context, force bool) repoStatusResult {
	start := time.Now()
	now := s.now()

	s.mu.Lock()
	returnWithLog := func(res repoStatusResult) repoStatusResult {
		logger := s.logger
		s.mu.Unlock()
		s.logSampleDuration(logger, start, force, res)
		return res
	}

	if strings.TrimSpace(s.repoRoot) == "" {
		return returnWithLog(repoStatusResult{Err: fmt.Errorf("repo root unavailable")})
	}

	if !force && s.last.Err != nil && !s.nextAllowedAt.IsZero() && now.Before(s.nextAllowedAt) {
		res := s.last
		res.FromCache = true
		res.Throttled = true
		res.ThrottleReason = "error-cooldown"
		res.NextAllowedAt = s.nextAllowedAt
		return returnWithLog(res)
	}

	if !force && s.inFlight {
		res := s.last
		res.FromCache = true
		res.Throttled = true
		res.ThrottleReason = "in-flight"
		res.NextAllowedAt = s.nextAllowedAt
		return returnWithLog(res)
	}

	gitDir, err := resolveGitDir(s.repoRoot)
	if err != nil {
		res := repoStatusResult{
			Err:       err,
			SampledAt: now,
		}
		s.last = res
		s.nextAllowedAt = now.Add(s.notGitRepoCooldown)
		res.NextAllowedAt = s.nextAllowedAt
		return returnWithLog(res)
	}

	headPath := filepath.Join(gitDir, "HEAD")
	idxPath := filepath.Join(gitDir, "index")

	headStamp, headChanged, headErr := fileChanged(headPath, s.lastHeadStamp)
	if headErr != nil {
		res := repoStatusResult{Err: headErr, SampledAt: now}
		s.last = res
		s.nextAllowedAt = now.Add(s.cooldown)
		res.NextAllowedAt = s.nextAllowedAt
		return returnWithLog(res)
	}
	idxStamp, idxChanged, idxErr := fileChanged(idxPath, s.lastIdxStamp)
	if idxErr != nil {
		res := repoStatusResult{Err: idxErr, SampledAt: now}
		s.last = res
		s.nextAllowedAt = now.Add(s.cooldown)
		res.NextAllowedAt = s.nextAllowedAt
		return returnWithLog(res)
	}

	filesChanged := headChanged || idxChanged
	if !force && !filesChanged && !s.last.SampledAt.IsZero() && !s.nextAllowedAt.IsZero() && now.Before(s.nextAllowedAt) {
		res := s.last
		res.FromCache = true
		res.Throttled = true
		res.ThrottleReason = "cooldown"
		res.NextAllowedAt = s.nextAllowedAt
		return returnWithLog(res)
	}

	if _, lookErr := s.lookPath("git"); lookErr != nil {
		res := repoStatusResult{
			Err:       fmt.Errorf("git missing"),
			SampledAt: now,
		}
		s.last = res
		s.nextAllowedAt = now.Add(s.gitMissingCooldown)
		res.NextAllowedAt = s.nextAllowedAt
		return returnWithLog(res)
	}

	s.inFlight = true
	s.mu.Unlock()

	snapshot, fetchErr := s.fetch(ctx, s.repoRoot)

	s.mu.Lock()
	s.inFlight = false

	res := repoStatusResult{
		Snapshot:  snapshot,
		Err:       fetchErr,
		SampledAt: now,
	}

	s.lastHeadStamp = headStamp
	s.lastIdxStamp = idxStamp

	cooldown := s.cooldown
	if fetchErr != nil {
		msg := strings.ToLower(fetchErr.Error())
		if strings.Contains(msg, "not a git repo") || strings.Contains(msg, "not a git repository") {
			cooldown = s.notGitRepoCooldown
		}
		if strings.Contains(msg, "git missing") {
			cooldown = s.gitMissingCooldown
		}
	}
	s.nextAllowedAt = now.Add(cooldown)
	res.NextAllowedAt = s.nextAllowedAt

	s.last = res
	logger := s.logger
	s.mu.Unlock()
	s.logSampleDuration(logger, start, force, res)
	return res
}

func (s *RepoStatusSampler) logSampleDuration(logger *tuiLogger, start time.Time, force bool, res repoStatusResult) {
	if logger == nil {
		return
	}
	fields := map[string]any{
		"duration_ms": time.Since(start).Milliseconds(),
		"force":       force,
	}
	if res.FromCache {
		fields["from_cache"] = true
	}
	if res.Throttled {
		fields["throttled"] = true
	}
	if res.ThrottleReason != "" {
		fields["throttle_reason"] = res.ThrottleReason
	}
	if res.Err != nil {
		fields["error"] = res.Err.Error()
	}
	logger.Debug("repo_status.sample", fields)
}

func resolveGitDir(repoRoot string) (string, error) {
	dotGit := filepath.Join(repoRoot, ".git")
	info, err := os.Stat(dotGit)
	if err != nil {
		if errors.Is(err, os.ErrNotExist) {
			return "", fmt.Errorf("not a git repo")
		}
		return "", err
	}
	if info.IsDir() {
		return dotGit, nil
	}

	data, err := os.ReadFile(dotGit)
	if err != nil {
		return "", err
	}
	content := strings.TrimSpace(string(data))
	lines := strings.Split(content, "\n")
	for _, line := range lines {
		line = strings.TrimSpace(line)
		if strings.HasPrefix(strings.ToLower(line), "gitdir:") {
			path := strings.TrimSpace(line[len("gitdir:"):])
			if path == "" {
				break
			}
			if !filepath.IsAbs(path) {
				path = filepath.Clean(filepath.Join(repoRoot, path))
			}
			return path, nil
		}
	}
	return "", fmt.Errorf("not a git repo")
}

func fetchRepoStatus(ctx context.Context, repoRoot string) (repoStatusSnapshot, error) {
	snapshot := repoStatusSnapshot{}
	if strings.TrimSpace(repoRoot) == "" {
		return snapshot, fmt.Errorf("repo root unavailable")
	}

	branch, err := loop.CurrentBranch(ctx, repoRoot)
	if err != nil {
		note := repoNoteFromError(err, "unavailable")
		snapshot.BranchNote = note
		if note == "not a git repo" || note == "git missing" {
			return snapshot, errors.New(note)
		}
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
