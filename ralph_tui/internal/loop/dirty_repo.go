// Package loop defines dirty-repo policies for the Ralph loop.
// Entrypoint: ParseDirtyRepoPolicy.
package loop

import (
	"fmt"
	"strings"
)

// DirtyRepoPolicy describes how the loop should respond to a dirty repo.
type DirtyRepoPolicy string

const (
	DirtyRepoPolicyError      DirtyRepoPolicy = "error"
	DirtyRepoPolicyWarn       DirtyRepoPolicy = "warn"
	DirtyRepoPolicyQuarantine DirtyRepoPolicy = "quarantine"
)

// ParseDirtyRepoPolicy normalizes and validates a dirty-repo policy string.
func ParseDirtyRepoPolicy(value string) (DirtyRepoPolicy, error) {
	normalized := strings.ToLower(strings.TrimSpace(value))
	switch normalized {
	case "":
		return "", nil
	case string(DirtyRepoPolicyError):
		return DirtyRepoPolicyError, nil
	case string(DirtyRepoPolicyWarn):
		return DirtyRepoPolicyWarn, nil
	case string(DirtyRepoPolicyQuarantine):
		return DirtyRepoPolicyQuarantine, nil
	default:
		return "", fmt.Errorf("dirty repo policy must be error, warn, or quarantine (got: %s)", value)
	}
}

// DirtyRepoError reports a dirty repo at a specific stage.
type DirtyRepoError struct {
	RepoRoot       string
	Stage          string
	AllowUntracked bool
	Status         GitStatus
}

func (e *DirtyRepoError) Error() string {
	if e == nil {
		return "dirty repo"
	}
	mode := "tracked-only"
	if !e.AllowUntracked {
		mode = "tracked+untracked"
	}
	return fmt.Sprintf("working tree is dirty (%s) at %s", mode, e.Stage)
}
