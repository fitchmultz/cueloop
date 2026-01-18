// Package tui provides runner argument helpers for forms and views.
// Entrypoint: parseArgsLines and formatArgsLines.
package tui

import (
	"strings"
)

func formatArgsLines(args []string) string {
	if len(args) == 0 {
		return ""
	}
	return strings.Join(args, "\n")
}

func parseArgsLines(value string) []string {
	if strings.TrimSpace(value) == "" {
		return nil
	}
	lines := strings.Split(value, "\n")
	args := make([]string, 0, len(lines))
	for _, line := range lines {
		trimmed := strings.TrimSpace(line)
		if trimmed == "" {
			continue
		}
		args = append(args, trimmed)
	}
	return args
}
