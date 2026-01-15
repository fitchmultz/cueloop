// Package tui provides helpers for persisting TUI run output to disk.
package tui

import (
	"bufio"
	"errors"
	"os"
	"path/filepath"
	"strings"
	"sync"
	"time"
)

const (
	outputWriterBufferSize      = 256 * 1024
	outputWriterFlushInterval   = 250 * time.Millisecond
	outputWriterMaxPendingLines = 128
	outputWriterMaxPendingBytes = 64 * 1024
)

type outputFileWriter struct {
	mu              sync.Mutex
	path            string
	file            *os.File
	w               *bufio.Writer
	lastFlush       time.Time
	pendingLines    int
	flushInterval   time.Duration
	maxPendingLines int
	maxPendingBytes int
	bufferSize      int
}

func (o *outputFileWriter) Path() string {
	if o == nil {
		return ""
	}
	o.mu.Lock()
	defer o.mu.Unlock()
	return o.path
}

func (o *outputFileWriter) Reset(path string) error {
	if o == nil {
		return errors.New("output writer is nil")
	}
	o.mu.Lock()
	defer o.mu.Unlock()
	if err := o.closeLocked(); err != nil {
		return err
	}
	if strings.TrimSpace(path) == "" {
		o.path = ""
		return errors.New("output path is empty")
	}
	if err := os.MkdirAll(filepath.Dir(path), 0o700); err != nil {
		return err
	}
	file, err := os.OpenFile(path, os.O_CREATE|os.O_TRUNC|os.O_WRONLY, 0o600)
	if err != nil {
		return err
	}
	if o.flushInterval == 0 {
		o.flushInterval = outputWriterFlushInterval
	}
	if o.maxPendingLines == 0 {
		o.maxPendingLines = outputWriterMaxPendingLines
	}
	if o.maxPendingBytes == 0 {
		o.maxPendingBytes = outputWriterMaxPendingBytes
	}
	if o.bufferSize == 0 {
		o.bufferSize = outputWriterBufferSize
	}
	o.path = path
	o.file = file
	o.w = bufio.NewWriterSize(file, o.bufferSize)
	o.lastFlush = time.Now()
	o.pendingLines = 0
	return nil
}

func (o *outputFileWriter) AppendLines(lines []string) error {
	if o == nil {
		return nil
	}
	if len(lines) == 0 {
		return nil
	}
	o.mu.Lock()
	defer o.mu.Unlock()
	if o.file == nil || o.w == nil {
		return errors.New("output file not initialized")
	}
	for _, line := range lines {
		line = strings.TrimRight(line, "\n")
		if _, err := o.w.WriteString(line + "\n"); err != nil {
			return err
		}
	}
	o.pendingLines += len(lines)
	now := time.Now()
	if o.shouldFlushLocked(now) {
		if err := o.w.Flush(); err != nil {
			return err
		}
		o.pendingLines = 0
		o.lastFlush = now
	}
	return nil
}

func (o *outputFileWriter) Close() error {
	if o == nil {
		return nil
	}
	o.mu.Lock()
	defer o.mu.Unlock()
	return o.closeLocked()
}

func (o *outputFileWriter) closeLocked() error {
	var err error
	if o.w != nil {
		if flushErr := o.w.Flush(); flushErr != nil && err == nil {
			err = flushErr
		}
	}
	if o.file != nil {
		if closeErr := o.file.Close(); closeErr != nil && err == nil {
			err = closeErr
		}
	}
	o.w = nil
	o.file = nil
	o.pendingLines = 0
	o.lastFlush = time.Time{}
	return err
}

func (o *outputFileWriter) shouldFlushLocked(now time.Time) bool {
	if o.maxPendingLines > 0 && o.pendingLines >= o.maxPendingLines {
		return true
	}
	if o.maxPendingBytes > 0 && o.w.Buffered() >= o.maxPendingBytes {
		return true
	}
	if o.flushInterval > 0 && !o.lastFlush.IsZero() && now.Sub(o.lastFlush) >= o.flushInterval {
		return true
	}
	return false
}
