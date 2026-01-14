// Package tui provides a lightweight JSONL file logger for the TUI.
package tui

import (
	"encoding/json"
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"sync"
	"time"

	"github.com/mitchfultz/ralph/ralph_tui/internal/config"
)

type logLevel int

const (
	logDebug logLevel = iota
	logInfo
	logWarn
	logError
)

const maxLogSizeBytes int64 = 2 * 1024 * 1024

type logEntry struct {
	Timestamp string         `json:"ts"`
	Level     string         `json:"level"`
	Message   string         `json:"msg"`
	Fields    map[string]any `json:"fields,omitempty"`
}

type tuiLogger struct {
	path     string
	level    logLevel
	maxBytes int64
	mu       sync.Mutex
}

func newTUILogger(cfg config.Config) (*tuiLogger, error) {
	path, err := resolveLogPath(cfg)
	if err != nil {
		return nil, err
	}
	if err := os.MkdirAll(filepath.Dir(path), 0o700); err != nil {
		return nil, err
	}

	return &tuiLogger{
		path:     path,
		level:    parseLogLevel(cfg.Logging.Level),
		maxBytes: maxLogSizeBytes,
	}, nil
}

func resolveLogPath(cfg config.Config) (string, error) {
	if strings.TrimSpace(cfg.Logging.File) != "" {
		return filepath.Clean(cfg.Logging.File), nil
	}
	if strings.TrimSpace(cfg.Paths.CacheDir) == "" {
		return "", fmt.Errorf("cache dir is required to resolve log path")
	}
	return filepath.Join(cfg.Paths.CacheDir, "ralph_tui.log"), nil
}

func (l *tuiLogger) Path() string {
	if l == nil {
		return ""
	}
	return l.path
}

func (l *tuiLogger) Debug(message string, fields map[string]any) {
	l.Log(logDebug, message, fields)
}

func (l *tuiLogger) Info(message string, fields map[string]any) {
	l.Log(logInfo, message, fields)
}

func (l *tuiLogger) Warn(message string, fields map[string]any) {
	l.Log(logWarn, message, fields)
}

func (l *tuiLogger) Error(message string, fields map[string]any) {
	l.Log(logError, message, fields)
}

func (l *tuiLogger) Log(level logLevel, message string, fields map[string]any) {
	if l == nil || level < l.level {
		return
	}

	entry := logEntry{
		Timestamp: time.Now().UTC().Format(time.RFC3339Nano),
		Level:     level.String(),
		Message:   message,
		Fields:    fields,
	}

	payload, err := json.Marshal(entry)
	if err != nil {
		return
	}

	l.mu.Lock()
	defer l.mu.Unlock()

	_ = l.rotateIfNeeded()
	file, err := os.OpenFile(l.path, os.O_APPEND|os.O_CREATE|os.O_WRONLY, 0o600)
	if err != nil {
		return
	}
	defer file.Close()

	_, _ = file.Write(append(payload, '\n'))
}

func (l *tuiLogger) rotateIfNeeded() error {
	info, err := os.Stat(l.path)
	if err != nil {
		if errors.Is(err, os.ErrNotExist) {
			return nil
		}
		return err
	}
	if info.Size() < l.maxBytes {
		return nil
	}
	backup := l.path + ".1"
	_ = os.Remove(backup)
	if err := os.Rename(l.path, backup); err != nil {
		return err
	}
	return nil
}

func parseLogLevel(level string) logLevel {
	switch strings.ToLower(strings.TrimSpace(level)) {
	case "debug":
		return logDebug
	case "warn":
		return logWarn
	case "error":
		return logError
	default:
		return logInfo
	}
}

func (l logLevel) String() string {
	switch l {
	case logDebug:
		return "debug"
	case logWarn:
		return "warn"
	case logError:
		return "error"
	default:
		return "info"
	}
}
