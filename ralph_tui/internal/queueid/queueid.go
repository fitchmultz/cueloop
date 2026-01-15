// Package queueid provides exact queue item ID parsing helpers.
package queueid

import (
	"fmt"
	"regexp"
	"strconv"
	"strings"
)

const DefaultPrefix = "RQ"

var (
	idPattern      = regexp.MustCompile(`[A-Z0-9]{2,10}-\d{4}`)
	idPartsPattern = regexp.MustCompile(`^([A-Z0-9]{2,10})-(\d{4})$`)
)

// Extract returns the first exact ID match in the line, ignoring partial suffixes.
func Extract(line string) string {
	matches := idPattern.FindAllStringIndex(line, -1)
	for _, match := range matches {
		end := match[1]
		if end < len(line) {
			next := line[end]
			if next >= '0' && next <= '9' {
				continue
			}
		}
		return line[match[0]:match[1]]
	}
	return ""
}

// Parse returns the normalized prefix and numeric portion for a well-formed ID.
func Parse(id string) (string, int, bool) {
	trimmed := strings.TrimSpace(id)
	matches := idPartsPattern.FindStringSubmatch(trimmed)
	if len(matches) != 3 {
		return "", 0, false
	}
	value, err := strconv.Atoi(matches[2])
	if err != nil {
		return "", 0, false
	}
	return matches[1], value, true
}

// Next returns the next available ID for the prefix, using the provided IDs as a source of truth.
func Next(prefix string, ids []string) (string, error) {
	normalized := strings.ToUpper(strings.TrimSpace(prefix))
	if normalized == "" {
		return "", fmt.Errorf("prefix is required")
	}

	max := 0
	for _, id := range ids {
		idPrefix, number, ok := Parse(id)
		if !ok {
			continue
		}
		if idPrefix != normalized {
			continue
		}
		if number > max {
			max = number
		}
	}

	return fmt.Sprintf("%s-%04d", normalized, max+1), nil
}
