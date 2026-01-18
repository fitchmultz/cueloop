// Package tui provides shared output path helpers for log persistence.
package tui

import (
	"path/filepath"
	"strings"
)

const (
	loopOutputLogFilename  = "loop_output.log"
	specsOutputLogFilename = "specs_output.log"
)

func loopOutputLogPath(cacheDir string) string {
	return outputLogPath(cacheDir, loopOutputLogFilename)
}

func specsOutputLogPath(cacheDir string) string {
	return outputLogPath(cacheDir, specsOutputLogFilename)
}

func outputLogPath(cacheDir string, filename string) string {
	if strings.TrimSpace(cacheDir) == "" {
		return ""
	}
	return filepath.Join(cacheDir, filename)
}
