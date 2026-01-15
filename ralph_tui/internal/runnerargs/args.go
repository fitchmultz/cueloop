// Package runnerargs centralizes runner argument parsing and reasoning effort handling.
// Entrypoint: ApplyReasoningEffort.
package runnerargs

import "strings"

// NormalizeArgs trims and drops empty args while preserving order.
func NormalizeArgs(args []string) []string {
	trimmed := make([]string, 0, len(args))
	for _, arg := range args {
		value := strings.TrimSpace(arg)
		if value == "" {
			continue
		}
		trimmed = append(trimmed, value)
	}
	if len(trimmed) == 0 {
		return nil
	}
	return trimmed
}

// MergeArgs appends extra args after the base args, normalizing both.
func MergeArgs(base []string, extra []string) []string {
	normalizedBase := NormalizeArgs(base)
	normalizedExtra := NormalizeArgs(extra)
	if len(normalizedBase) == 0 && len(normalizedExtra) == 0 {
		return nil
	}
	merged := append([]string{}, normalizedBase...)
	merged = append(merged, normalizedExtra...)
	return merged
}
