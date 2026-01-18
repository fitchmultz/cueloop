// Package loop provides queue parsing helpers.
// Entrypoint: FirstUncheckedItem, ItemCompletionStatus, ExtractItemID.
package loop

import (
	"fmt"
	"strings"

	"github.com/mitchfultz/ralph/ralph_tui/internal/pin"
	"github.com/mitchfultz/ralph/ralph_tui/internal/queueid"
)

// QueueItem captures a parsed queue item header and block.
type QueueItem = pin.QueueItem

// FirstUncheckedItem returns the first unchecked queue item matching tags.
func FirstUncheckedItem(queuePath string, onlyTags []string) (*QueueItem, error) {
	items, err := readQueueItems(queuePath)
	if err != nil {
		return nil, err
	}
	for _, item := range items {
		if item.Checked {
			continue
		}
		if pin.MatchesAnyTag(item.Header, onlyTags) {
			return &item, nil
		}
	}
	return nil, nil
}

// ItemCompletionStatus returns true when the item is completed (checked or in Done).
func ItemCompletionStatus(queuePath string, donePath string, itemID string) (bool, error) {
	items, err := readQueueItems(queuePath)
	if err != nil {
		return false, err
	}
	for _, item := range items {
		if item.ID == itemID {
			return item.Checked, nil
		}
	}

	doneItems, err := readDoneItems(donePath)
	if err != nil {
		return false, err
	}
	for _, item := range doneItems {
		if item.ID == itemID {
			return true, nil
		}
	}

	return false, nil
}

// CurrentItemBlock returns the block for the given item ID.
func CurrentItemBlock(queuePath string, itemID string) (string, error) {
	items, err := readQueueItems(queuePath)
	if err != nil {
		return "", err
	}
	for _, item := range items {
		if item.ID == itemID {
			return strings.Join(item.Lines, "\n"), nil
		}
	}
	return "", fmt.Errorf("item %s not found", itemID)
}

// ExtractItemID returns the ID from a queue header line.
func ExtractItemID(line string) string {
	return queueid.Extract(line)
}

// ExtractItemTitle returns the title portion of a queue item line.
func ExtractItemTitle(line string) string {
	trimmed := strings.TrimSpace(line)
	if strings.HasPrefix(trimmed, "- [") {
		trimmed = strings.TrimSpace(trimmed[5:])
	}
	if id := queueid.Extract(trimmed); id != "" {
		idx := strings.Index(trimmed, id)
		if idx >= 0 {
			trimmed = strings.TrimSpace(trimmed[idx+len(id):])
		}
	}
	trimmed = strings.TrimSpace(strings.TrimPrefix(trimmed, ":"))
	trimmed = strings.TrimSpace(strings.TrimPrefix(trimmed, ":"))
	if strings.HasPrefix(trimmed, "[") {
		if closing := strings.Index(trimmed, "]"); closing >= 0 {
			trimmed = strings.TrimSpace(trimmed[closing+1:])
		}
	}
	trimmed = strings.TrimSpace(strings.TrimPrefix(trimmed, ":"))
	return strings.TrimSpace(trimmed)
}

func readQueueItems(queuePath string) ([]QueueItem, error) {
	return pin.ReadQueueItems(queuePath)
}

func readDoneItems(donePath string) ([]QueueItem, error) {
	return pin.ReadDoneItems(donePath)
}
