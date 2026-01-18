//go:build !windows

package lockfile

import (
	"errors"
	"os/exec"
	"strconv"
	"strings"
	"syscall"
)

func isPIDRunning(pid int) bool {
	if pid <= 0 {
		return false
	}
	err := syscall.Kill(pid, 0)
	if err == nil {
		return true
	}
	return errors.Is(err, syscall.EPERM)
}

func parentPID(pid int) (int, error) {
	cmd := exec.Command("ps", "-o", "ppid=", "-p", strconv.Itoa(pid))
	output, err := cmd.Output()
	if err != nil {
		return 0, err
	}
	trimmed := strings.TrimSpace(string(output))
	if trimmed == "" {
		return 0, nil
	}
	return strconv.Atoi(trimmed)
}
