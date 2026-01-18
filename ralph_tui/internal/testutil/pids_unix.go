//go:build !windows

package testutil

import (
	"syscall"
)

// IsPIDRunning reports whether a PID exists (best-effort).
func IsPIDRunning(pid int) bool {
	if pid <= 0 {
		return false
	}
	return syscall.Kill(pid, 0) == nil
}
