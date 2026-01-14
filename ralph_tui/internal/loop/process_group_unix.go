//go:build !windows

// Package loop provides process group cancellation helpers for Unix systems.
package loop

import (
	"os/exec"
	"syscall"
)

func configureProcessGroup(cmd *exec.Cmd) {
	if cmd == nil {
		return
	}
	if cmd.SysProcAttr == nil {
		cmd.SysProcAttr = &syscall.SysProcAttr{}
	}
	cmd.SysProcAttr.Setpgid = true
	if cmd.Cancel != nil {
		cmd.Cancel = func() error {
			if cmd.Process == nil {
				return nil
			}
			pgid, err := syscall.Getpgid(cmd.Process.Pid)
			if err != nil {
				return cmd.Process.Kill()
			}
			return syscall.Kill(-pgid, syscall.SIGKILL)
		}
	}
}
