// Package tui provides lightweight file change helpers for periodic refresh.
package tui

import (
	"crypto/sha256"
	"encoding/hex"
	"errors"
	"fmt"
	"io"
	"os"
	"strings"
	"time"
)

type fileStamp struct {
	Exists   bool
	ModTime  time.Time
	Size     int64
	Inode    uint64
	HasInode bool
	Ctime    time.Time
	HasCtime bool
	Hash     string
	HasHash  bool
}

const fileStampHashMaxBytes int64 = 64 * 1024

func getFileStamp(path string) (fileStamp, error) {
	stamp, err := statFileStamp(path)
	if err != nil {
		return fileStamp{}, err
	}
	if !stamp.Exists {
		return stamp, nil
	}
	return addFileHash(path, stamp)
}

func fileChanged(path string, last fileStamp) (fileStamp, bool, error) {
	stamp, err := statFileStamp(path)
	if err != nil {
		return fileStamp{}, false, err
	}
	if !sameFileStampStat(stamp, last) {
		if needsContentHash(stamp, last) {
			updated, err := addFileHash(path, stamp)
			if err != nil {
				return fileStamp{}, false, err
			}
			stamp = updated
		}
		return stamp, true, nil
	}
	if !stamp.Exists {
		return stamp, false, nil
	}
	if !needsContentHash(stamp, last) {
		return stamp, false, nil
	}
	updated, err := addFileHash(path, stamp)
	if err != nil {
		return fileStamp{}, false, err
	}
	if last.HasHash && updated.HasHash && last.Hash != updated.Hash {
		return updated, true, nil
	}
	return updated, false, nil
}

func sameFileStampStat(left fileStamp, right fileStamp) bool {
	if left.Exists != right.Exists {
		return false
	}
	if !left.Exists {
		return true
	}
	if left.Size != right.Size {
		return false
	}
	if !left.ModTime.Equal(right.ModTime) {
		return false
	}
	if left.HasInode && right.HasInode && left.Inode != right.Inode {
		return false
	}
	if left.HasCtime && right.HasCtime && !left.Ctime.Equal(right.Ctime) {
		return false
	}
	return true
}

func sameFileStamp(left fileStamp, right fileStamp) bool {
	if !sameFileStampStat(left, right) {
		return false
	}
	if left.HasHash && right.HasHash && left.Hash != right.Hash {
		return false
	}
	return true
}

func fileStampSignature(stamp fileStamp) string {
	if !stamp.Exists {
		return "missing"
	}
	parts := []string{
		fmt.Sprintf("size=%d", stamp.Size),
		fmt.Sprintf("mtime=%d", stamp.ModTime.UnixNano()),
	}
	if stamp.HasInode {
		parts = append(parts, fmt.Sprintf("inode=%d", stamp.Inode))
	}
	if stamp.HasCtime {
		parts = append(parts, fmt.Sprintf("ctime=%d", stamp.Ctime.UnixNano()))
	}
	if stamp.HasHash {
		parts = append(parts, "hash="+stamp.Hash)
	}
	return strings.Join(parts, ";")
}

func statFileStamp(path string) (fileStamp, error) {
	info, err := os.Stat(path)
	if err != nil {
		if errors.Is(err, os.ErrNotExist) {
			return fileStamp{Exists: false}, nil
		}
		return fileStamp{}, err
	}
	stamp := fileStamp{
		Exists:  true,
		ModTime: info.ModTime(),
		Size:    info.Size(),
	}
	if details, ok := readFileStatDetails(info); ok {
		stamp.Inode = details.inode
		stamp.HasInode = details.hasInode
		stamp.Ctime = details.ctime
		stamp.HasCtime = details.hasCtime
	}
	return stamp, nil
}

func needsContentHash(stamp fileStamp, last fileStamp) bool {
	if !stamp.Exists || stamp.Size > fileStampHashMaxBytes {
		return false
	}
	if stamp.Size <= 0 {
		return false
	}
	return !(stamp.HasCtime && last.HasCtime)
}

func addFileHash(path string, stamp fileStamp) (fileStamp, error) {
	if !stamp.Exists || stamp.Size > fileStampHashMaxBytes || stamp.Size <= 0 {
		return stamp, nil
	}
	handle, err := os.Open(path)
	if err != nil {
		return fileStamp{}, err
	}
	defer handle.Close()

	hash, bytesRead, err := hashFilePrefixAt(handle, stamp.Size)
	if err != nil {
		return fileStamp{}, err
	}
	if bytesRead == stamp.Size {
		stamp.Hash = hash
		stamp.HasHash = true
	}
	return stamp, nil
}

func hashFilePrefixAt(reader io.ReaderAt, size int64) (string, int64, error) {
	if size <= 0 {
		return "", 0, nil
	}
	hasher := sha256.New()
	const bufSize = 32 * 1024
	buf := make([]byte, bufSize)
	var offset int64
	for offset < size {
		toRead := size - offset
		if toRead > int64(len(buf)) {
			toRead = int64(len(buf))
		}
		n, err := reader.ReadAt(buf[:toRead], offset)
		if n > 0 {
			if _, writeErr := hasher.Write(buf[:n]); writeErr != nil {
				return "", offset, writeErr
			}
			offset += int64(n)
		}
		if err != nil {
			if errors.Is(err, io.EOF) {
				break
			}
			return "", offset, err
		}
	}
	return hex.EncodeToString(hasher.Sum(nil)), offset, nil
}
