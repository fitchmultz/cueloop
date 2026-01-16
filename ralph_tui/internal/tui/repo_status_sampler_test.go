// Package tui verifies repo status sampling behavior and caching.
package tui

import (
	"context"
	"os"
	"path/filepath"
	"sync/atomic"
	"testing"
	"time"
)

func TestRepoStatusSampler_ThrottlesWhenNoChanges(t *testing.T) {
	repoRoot := t.TempDir()
	gitDir := filepath.Join(repoRoot, ".git")
	if err := os.MkdirAll(gitDir, 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(gitDir, "HEAD"), []byte("ref: refs/heads/main\n"), 0o644); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(gitDir, "index"), []byte("idx"), 0o644); err != nil {
		t.Fatal(err)
	}

	now := time.Date(2026, 1, 1, 0, 0, 0, 0, time.UTC)
	timeNow := func() time.Time { return now }

	var calls int32
	disableWorktree := false
	sampler := NewRepoStatusSampler(repoRoot, RepoStatusSamplerOptions{
		Cooldown: 30 * time.Second,
		TimeNow:  timeNow,
		Fetch: func(ctx context.Context, root string) (repoStatusSnapshot, error) {
			atomic.AddInt32(&calls, 1)
			return repoStatusSnapshot{Branch: "main"}, nil
		},
		EnableWorktreeSignature: &disableWorktree,
		LookPath:                func(string) (string, error) { return "/usr/bin/git", nil },
	})

	res1 := sampler.Sample(context.Background(), false)
	if res1.Err != nil {
		t.Fatalf("expected nil err, got %v", res1.Err)
	}
	if atomic.LoadInt32(&calls) != 1 {
		t.Fatalf("expected 1 fetch call, got %d", calls)
	}

	res2 := sampler.Sample(context.Background(), false)
	if atomic.LoadInt32(&calls) != 1 {
		t.Fatalf("expected still 1 fetch call, got %d", calls)
	}
	if !res2.FromCache || !res2.Throttled {
		t.Fatalf("expected cached+throttled result")
	}

	now = now.Add(31 * time.Second)
	res3 := sampler.Sample(context.Background(), false)
	if res3.Err != nil {
		t.Fatalf("expected nil err, got %v", res3.Err)
	}
	if atomic.LoadInt32(&calls) != 2 {
		t.Fatalf("expected 2 fetch calls, got %d", calls)
	}
}

func TestRepoStatusSampler_RefreshesWhenHeadChanges(t *testing.T) {
	repoRoot := t.TempDir()
	gitDir := filepath.Join(repoRoot, ".git")
	if err := os.MkdirAll(gitDir, 0o755); err != nil {
		t.Fatal(err)
	}
	headPath := filepath.Join(gitDir, "HEAD")
	if err := os.WriteFile(headPath, []byte("ref: refs/heads/main\n"), 0o644); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(gitDir, "index"), []byte("idx"), 0o644); err != nil {
		t.Fatal(err)
	}

	now := time.Date(2026, 1, 1, 0, 0, 0, 0, time.UTC)
	timeNow := func() time.Time { return now }

	var calls int32
	disableWorktree := false
	sampler := NewRepoStatusSampler(repoRoot, RepoStatusSamplerOptions{
		Cooldown: 30 * time.Second,
		TimeNow:  timeNow,
		Fetch: func(ctx context.Context, root string) (repoStatusSnapshot, error) {
			atomic.AddInt32(&calls, 1)
			return repoStatusSnapshot{Branch: "main"}, nil
		},
		EnableWorktreeSignature: &disableWorktree,
		LookPath:                func(string) (string, error) { return "/usr/bin/git", nil },
	})

	_ = sampler.Sample(context.Background(), false)
	if atomic.LoadInt32(&calls) != 1 {
		t.Fatalf("expected 1 fetch call, got %d", calls)
	}

	if err := os.WriteFile(headPath, []byte("ref: refs/heads/other\n"), 0o644); err != nil {
		t.Fatal(err)
	}
	if err := os.Chtimes(headPath, time.Now().Add(2*time.Second), time.Now().Add(2*time.Second)); err != nil {
		t.Fatal(err)
	}

	_ = sampler.Sample(context.Background(), false)
	if atomic.LoadInt32(&calls) != 2 {
		t.Fatalf("expected 2 fetch calls after HEAD change, got %d", calls)
	}
}

func TestRepoStatusSampler_CachesNotGitRepo(t *testing.T) {
	repoRoot := t.TempDir()

	now := time.Date(2026, 1, 1, 0, 0, 0, 0, time.UTC)
	timeNow := func() time.Time { return now }

	var calls int32
	disableWorktree := false
	sampler := NewRepoStatusSampler(repoRoot, RepoStatusSamplerOptions{
		NotGitRepoCooldown: 5 * time.Minute,
		TimeNow:            timeNow,
		Fetch: func(ctx context.Context, root string) (repoStatusSnapshot, error) {
			atomic.AddInt32(&calls, 1)
			return repoStatusSnapshot{Branch: "main"}, nil
		},
		EnableWorktreeSignature: &disableWorktree,
		LookPath:                func(string) (string, error) { return "/usr/bin/git", nil },
	})

	res1 := sampler.Sample(context.Background(), false)
	if res1.Err == nil {
		t.Fatalf("expected err")
	}
	if atomic.LoadInt32(&calls) != 0 {
		t.Fatalf("expected 0 fetch calls, got %d", calls)
	}

	if err := os.MkdirAll(filepath.Join(repoRoot, ".git"), 0o755); err != nil {
		t.Fatal(err)
	}

	res2 := sampler.Sample(context.Background(), false)
	if res2.Err == nil {
		t.Fatalf("expected err")
	}
	if atomic.LoadInt32(&calls) != 0 {
		t.Fatalf("expected 0 fetch calls, got %d", calls)
	}

	_ = sampler.Sample(context.Background(), true)
}

func TestRepoStatusSampler_WorktreeSignatureOverridesCooldown(t *testing.T) {
	repoRoot := t.TempDir()
	gitDir := filepath.Join(repoRoot, ".git")
	if err := os.MkdirAll(gitDir, 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(gitDir, "HEAD"), []byte("ref: refs/heads/main\n"), 0o644); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(gitDir, "index"), []byte("idx"), 0o644); err != nil {
		t.Fatal(err)
	}

	now := time.Date(2026, 1, 1, 0, 0, 0, 0, time.UTC)
	timeNow := func() time.Time { return now }

	var fetchCalls int32
	var sigCalls int32
	currentSignature := "sig-a"

	sampler := NewRepoStatusSampler(repoRoot, RepoStatusSamplerOptions{
		Cooldown:              30 * time.Second,
		TimeNow:               timeNow,
		WorktreeCheckInterval: 2 * time.Second,
		WorktreeSignature: func(ctx context.Context, root string) (string, error) {
			atomic.AddInt32(&sigCalls, 1)
			return currentSignature, nil
		},
		Fetch: func(ctx context.Context, root string) (repoStatusSnapshot, error) {
			atomic.AddInt32(&fetchCalls, 1)
			return repoStatusSnapshot{Branch: "main", worktreeSignature: currentSignature}, nil
		},
		LookPath: func(string) (string, error) { return "/usr/bin/git", nil },
	})

	_ = sampler.Sample(context.Background(), false)
	if atomic.LoadInt32(&fetchCalls) != 1 {
		t.Fatalf("expected 1 fetch call, got %d", fetchCalls)
	}
	if atomic.LoadInt32(&sigCalls) != 0 {
		t.Fatalf("expected 0 worktree signature calls, got %d", sigCalls)
	}

	_ = sampler.Sample(context.Background(), false)
	if atomic.LoadInt32(&fetchCalls) != 1 {
		t.Fatalf("expected still 1 fetch call, got %d", fetchCalls)
	}
	if atomic.LoadInt32(&sigCalls) != 0 {
		t.Fatalf("expected still 0 worktree signature calls, got %d", sigCalls)
	}

	now = now.Add(3 * time.Second)
	currentSignature = "sig-b"
	_ = sampler.Sample(context.Background(), false)
	if atomic.LoadInt32(&sigCalls) != 1 {
		t.Fatalf("expected 1 worktree signature call, got %d", sigCalls)
	}
	if atomic.LoadInt32(&fetchCalls) != 2 {
		t.Fatalf("expected 2 fetch calls after signature change, got %d", fetchCalls)
	}

	_ = sampler.Sample(context.Background(), false)
	if atomic.LoadInt32(&sigCalls) != 1 {
		t.Fatalf("expected worktree signature throttled, got %d calls", sigCalls)
	}
	if atomic.LoadInt32(&fetchCalls) != 2 {
		t.Fatalf("expected still 2 fetch calls, got %d", fetchCalls)
	}
}
