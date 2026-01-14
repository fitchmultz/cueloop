// Package tui provides helpers for persisting TUI run output to disk.
package tui

import (
	"bufio"
	"errors"
	"os"
	"path/filepath"
	"strings"
	"sync"
)

type outputFileWriter struct {
	mu   sync.Mutex
	path string
	file *os.File
	w    *bufio.Writer
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
	o.path = path
	o.file = file
	o.w = bufio.NewWriter(file)
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
	return o.w.Flush()
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
	return err
}
