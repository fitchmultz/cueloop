// Package tui provides runner argument helpers for forms and views.
// Entrypoint: parseArgsLines and formatArgsLines.
package tui

import (
	"fmt"
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

func containsEffortArg(args []string) bool {
	for _, token := range args {
		if strings.Contains(token, "model_reasoning_effort") {
			return true
		}
	}
	return false
}

func normalizeReasoningEffort(value string, defaultValue string) string {
	normalized := strings.ToLower(strings.TrimSpace(value))
	if normalized == "" || normalized == "auto" {
		return defaultValue
	}
	return normalized
}

func applyReasoningEffort(runner string, args []string, effort string, defaultEffort string) []string {
	if strings.ToLower(strings.TrimSpace(runner)) != "codex" {
		return args
	}
	if containsEffortArg(args) {
		return args
	}
	normalized := normalizeReasoningEffort(effort, defaultEffort)
	if normalized == "" {
		return args
	}
	return append([]string{"-c", fmt.Sprintf("model_reasoning_effort=\"%s\"", normalized)}, args...)
}

func displayReasoningEffort(value string) string {
	normalized := strings.ToLower(strings.TrimSpace(value))
	if normalized == "" {
		return "auto"
	}
	return normalized
}
