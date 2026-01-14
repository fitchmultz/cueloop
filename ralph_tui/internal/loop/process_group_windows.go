//go:build windows

// Package loop provides no-op process group configuration on Windows.
package loop

import "os/exec"

func configureProcessGroup(cmd *exec.Cmd) {}
