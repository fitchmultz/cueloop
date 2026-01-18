// Package pin provides queue ID allocation helpers.
// Entrypoint: NextQueueID.
package pin

import "github.com/mitchfultz/ralph/ralph_tui/internal/queueid"

// NextQueueID returns the next available queue ID across queue and done files.
func NextQueueID(files Files, prefix string) (string, error) {
	if err := requireFile(files.QueuePath); err != nil {
		return "", err
	}
	if err := requireFile(files.DonePath); err != nil {
		return "", err
	}

	queueLines, err := readLines(files.QueuePath)
	if err != nil {
		return "", err
	}
	doneLines, err := readLines(files.DonePath)
	if err != nil {
		return "", err
	}

	ids := append(extractIDs(queueLines), extractIDs(doneLines)...)
	if prefix == "" {
		prefix = queueid.DefaultPrefix
	}
	return queueid.Next(prefix, ids)
}
