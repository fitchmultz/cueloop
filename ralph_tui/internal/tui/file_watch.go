// Package tui provides lightweight file change helpers for periodic refresh.
package tui

import (
	"errors"
	"os"
	"time"
)

type fileStamp struct {
	Exists  bool
	ModTime time.Time
	Size    int64
}

func getFileStamp(path string) (fileStamp, error) {
	info, err := os.Stat(path)
	if err != nil {
		if errors.Is(err, os.ErrNotExist) {
			return fileStamp{Exists: false}, nil
		}
		return fileStamp{}, err
	}
	return fileStamp{
		Exists:  true,
		ModTime: info.ModTime(),
		Size:    info.Size(),
	}, nil
}

func fileChanged(path string, last fileStamp) (fileStamp, bool, error) {
	stamp, err := getFileStamp(path)
	if err != nil {
		return fileStamp{}, false, err
	}
	return stamp, !sameFileStamp(stamp, last), nil
}

func sameFileStamp(left fileStamp, right fileStamp) bool {
	if left.Exists != right.Exists {
		return false
	}
	if !left.Exists {
		return true
	}
	if left.Size != right.Size {
		return false
	}
	return left.ModTime.Equal(right.ModTime)
}
