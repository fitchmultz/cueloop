// Package tui provides the log viewer screen for recent activity.
package tui

import (
	"bytes"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"os"
	"sort"
	"strconv"
	"strings"
	"time"

	"github.com/charmbracelet/bubbles/viewport"
	tea "github.com/charmbracelet/bubbletea"
)

const (
	logsTailLines = 200
)

type logsFormat int

const (
	logsFormatRaw logsFormat = iota
	logsFormatFormatted
)

var logsLevelCycle = []string{"", "error", "warn", "info", "debug"}

type logsFilters struct {
	Level     string
	Component string
}

type parsedLogLine struct {
	raw       string
	parsed    bool
	level     string
	component string
	formatted string
}

type logLineCache struct {
	input  []string
	parsed []parsedLogLine
}

func (c *logLineCache) Reset() {
	c.input = nil
	c.parsed = nil
}

func (c *logLineCache) Update(lines []string) {
	if len(lines) == 0 {
		c.Reset()
		return
	}
	if len(c.input) == 0 || len(c.input) != len(c.parsed) {
		c.input = lines
		c.parsed = parseLogLines(lines)
		return
	}
	old := c.input
	oldParsed := c.parsed
	reused := 0
	for i, j := len(old)-1, len(lines)-1; i >= 0 && j >= 0; i, j = i-1, j-1 {
		if old[i] != lines[j] {
			break
		}
		reused++
	}
	parsed := make([]parsedLogLine, len(lines))
	updatedCount := len(lines) - reused
	if updatedCount > 0 {
		copy(parsed, parseLogLines(lines[:updatedCount]))
	}
	if reused > 0 && reused <= len(oldParsed) {
		copy(parsed[updatedCount:], oldParsed[len(oldParsed)-reused:])
	}
	c.input = lines
	c.parsed = parsed
}

type logSectionRenderOptions struct {
	AllowFormat               bool
	AllowFilters              bool
	KeepUnparsedWhenFiltering bool
}

type logsView struct {
	viewport                viewport.Model
	logPath                 string
	cacheDir                string
	loopPath                string
	specsPath               string
	loggerErr               string
	debugErr                string
	loopErr                 string
	specsErr                string
	debugStamp              fileStamp
	loopStamp               fileStamp
	specsStamp              fileStamp
	debugLines              []string
	loopLines               []string
	specsLines              []string
	format                  logsFormat
	filters                 logsFilters
	width                   int
	height                  int
	lastRenderedContent     string
	viewportSetContentCalls int
	forceRefresh            bool
	debugCache              logLineCache
	loopCache               logLineCache
	specsCache              logLineCache
}

func newLogsView(logPath string) *logsView {
	return &logsView{
		viewport: viewport.New(80, 20),
		logPath:  logPath,
		format:   logsFormatRaw,
	}
}

func (l *logsView) SetLogPath(path string) {
	if l.logPath == path {
		return
	}
	l.logPath = path
	l.debugErr = ""
	l.debugStamp = fileStamp{}
	l.debugLines = nil
	l.debugCache.Reset()
	l.lastRenderedContent = ""
	l.forceRefresh = true
}

func (l *logsView) SetCacheDir(cacheDir string) {
	cacheDir = strings.TrimSpace(cacheDir)
	if l.cacheDir == cacheDir {
		return
	}
	l.cacheDir = cacheDir
	l.loopPath = loopOutputLogPath(cacheDir)
	l.specsPath = specsOutputLogPath(cacheDir)
	l.loopErr = ""
	l.specsErr = ""
	l.loopStamp = fileStamp{}
	l.specsStamp = fileStamp{}
	l.loopLines = nil
	l.specsLines = nil
	l.loopCache.Reset()
	l.specsCache.Reset()
	l.lastRenderedContent = ""
	l.forceRefresh = true
}

func (l *logsView) SetLoggerError(err error) {
	if err == nil {
		l.loggerErr = ""
		return
	}
	l.loggerErr = err.Error()
}

func (l *logsView) Update(msg tea.Msg) tea.Cmd {
	updated, cmd := l.viewport.Update(msg)
	l.viewport = updated
	return cmd
}

func (l *logsView) ToggleFormat() {
	atBottom := l.viewport.AtBottom()
	if l.format == logsFormatRaw {
		l.format = logsFormatFormatted
	} else {
		l.format = logsFormatRaw
	}
	l.setViewportContentIfChanged(l.renderContent(), atBottom)
}

func (l *logsView) View() string {
	header := "Logs"
	status := l.statusLine()
	content := l.viewport.View()
	return withFinalNewline(header + "\n" + status + "\n\n" + content)
}

func (l *logsView) Resize(width int, height int) {
	l.width = width
	l.height = height
	contentHeight := height - 3
	if contentHeight < 0 {
		contentHeight = 0
	}
	resizeViewportToFit(&l.viewport, max(0, width), max(0, contentHeight), paddedViewportStyle)
}

func (l *logsView) Refresh() {
	atBottom := l.viewport.AtBottom()
	contentChanged := l.forceRefresh
	l.forceRefresh = false

	if l.refreshTailedFile(l.logPath, &l.debugStamp, &l.debugLines, &l.debugErr, &l.debugCache, true) {
		contentChanged = true
	}
	if l.refreshTailedFile(l.loopPath, &l.loopStamp, &l.loopLines, &l.loopErr, &l.loopCache, true) {
		contentChanged = true
	}
	if l.refreshTailedFile(l.specsPath, &l.specsStamp, &l.specsLines, &l.specsErr, &l.specsCache, true) {
		contentChanged = true
	}

	if !contentChanged {
		return
	}
	rendered := l.renderContent()
	l.setViewportContentIfChanged(rendered, atBottom)
}

type cacheResetter interface {
	Reset()
}

func (l *logsView) refreshTailedFile(path string, stamp *fileStamp, lines *[]string, errText *string, cache cacheResetter, clearOnEmptyPath bool) bool {
	if strings.TrimSpace(path) == "" {
		changed := false
		if clearOnEmptyPath && len(*lines) > 0 {
			*lines = nil
			if cache != nil {
				cache.Reset()
			}
			*errText = ""
			changed = true
		}
		if *errText != "" {
			*errText = ""
			changed = true
		}
		return changed
	}
	nextStamp, changed, err := fileChanged(path, *stamp)
	if err != nil {
		if errText != nil {
			*errText = err.Error()
		}
		return false
	}
	if !nextStamp.Exists {
		stateChanged := false
		if len(*lines) > 0 {
			*lines = nil
			stateChanged = true
		}
		if errText != nil && *errText != "" {
			*errText = ""
			stateChanged = true
		}
		if cache != nil {
			cache.Reset()
		}
		if stamp != nil {
			*stamp = nextStamp
		}
		return stateChanged
	}
	if !changed && errText != nil && *errText == "" {
		if stamp != nil {
			*stamp = nextStamp
		}
		return false
	}
	tail, err := tailFileLines(path, logsTailLines)
	if err != nil {
		if errText != nil {
			*errText = err.Error()
		}
		return false
	}
	if errText != nil {
		*errText = ""
	}
	*lines = tail
	if stamp != nil {
		*stamp = nextStamp
	}
	return true
}

func (l *logsView) statusLine() string {
	formatNote := "Format (json): " + l.formatLabel()
	filterNote := "Filters (json): Level=" + filterLabel(l.filters.Level) + " | Component=" + filterLabel(l.filters.Component)
	errParts := make([]string, 0, 4)
	if l.loggerErr != "" {
		errParts = append(errParts, "Logger: "+l.loggerErr)
	}
	if l.debugErr != "" {
		errParts = append(errParts, "Debug: "+l.debugErr)
	}
	if l.loopErr != "" {
		errParts = append(errParts, "Loop: "+l.loopErr)
	}
	if l.specsErr != "" {
		errParts = append(errParts, "Specs: "+l.specsErr)
	}
	if len(errParts) > 0 {
		return "Error: " + strings.Join(errParts, " | ") + " | " + formatNote + " | " + filterNote
	}
	if strings.TrimSpace(l.logPath) == "" {
		return "Debug log unavailable. | " + formatNote + " | " + filterNote
	}
	return "Log file: " + l.logPath + " | " + formatNote + " | " + filterNote
}

func (l *logsView) renderContent() string {
	sections := []string{
		"Debug Log (tail)",
		l.renderLogLines(l.debugLines, "No log entries yet.", &l.debugCache, logSectionRenderOptions{
			AllowFormat:               true,
			AllowFilters:              true,
			KeepUnparsedWhenFiltering: false,
		}),
		"",
		"Loop Output (tail)",
		l.renderLogLines(l.loopLines, "No loop output yet.", &l.loopCache, logSectionRenderOptions{
			AllowFormat:               true,
			AllowFilters:              true,
			KeepUnparsedWhenFiltering: true,
		}),
		"",
		"Specs Output (tail)",
		l.renderLogLines(l.specsLines, "No specs output yet.", &l.specsCache, logSectionRenderOptions{
			AllowFormat:               true,
			AllowFilters:              true,
			KeepUnparsedWhenFiltering: true,
		}),
	}
	return strings.Join(sections, "\n")
}

func (l *logsView) formatLabel() string {
	if l.format == logsFormatFormatted {
		return "formatted"
	}
	return "raw"
}

type tailReadAtStater interface {
	Stat() (os.FileInfo, error)
	ReadAt(p []byte, off int64) (n int, err error)
}

func tailFileLines(path string, limit int) ([]string, error) {
	if limit <= 0 {
		return []string{}, nil
	}
	file, err := os.Open(path)
	if err != nil {
		if errors.Is(err, os.ErrNotExist) {
			return []string{}, nil
		}
		return nil, err
	}
	defer file.Close()

	return tailFileLinesFromHandle(file, limit)
}

func tailFileLinesFromHandle(file tailReadAtStater, limit int) ([]string, error) {
	info, err := file.Stat()
	if err != nil {
		return nil, err
	}
	if info.Size() == 0 {
		return []string{}, nil
	}
	const chunkSize int64 = 64 * 1024
	pos := info.Size()
	newlineCount := 0
	chunks := make([][]byte, 0, 8)
	trimTrailing := true
	retriesRemaining := 2

	for pos > 0 && newlineCount < limit+1 {
		readLen := chunkSize
		if pos < readLen {
			readLen = pos
		}
		pos -= readLen

		buf := make([]byte, int(readLen))
		n, err := file.ReadAt(buf, pos)
		if err != nil && !errors.Is(err, io.EOF) {
			return nil, err
		}
		if errors.Is(err, io.EOF) && int64(n) < readLen {
			if retriesRemaining > 0 {
				info, statErr := file.Stat()
				if statErr == nil && info.Size() < pos+readLen {
					pos = info.Size()
					retriesRemaining--
					newlineCount = 0
					trimTrailing = true
					chunks = chunks[:0]
					continue
				}
			}
		}
		if n == 0 {
			if errors.Is(err, io.EOF) && retriesRemaining > 0 {
				info, statErr := file.Stat()
				if statErr == nil && info.Size() < pos+readLen {
					pos = info.Size()
					retriesRemaining--
					newlineCount = 0
					trimTrailing = true
					chunks = chunks[:0]
					continue
				}
			}
			break
		}
		buf = buf[:n]

		if trimTrailing {
			buf = bytes.TrimRight(buf, "\r\n")
			if len(buf) == 0 {
				continue
			}
			trimTrailing = false
		}

		newlineCount += bytes.Count(buf, []byte{'\n'})
		chunks = append(chunks, buf)
	}

	if len(chunks) == 0 {
		return []string{}, nil
	}

	totalLen := 0
	for _, chunk := range chunks {
		totalLen += len(chunk)
	}
	data := make([]byte, 0, totalLen)
	for i := len(chunks) - 1; i >= 0; i-- {
		data = append(data, chunks[i]...)
	}
	if len(data) == 0 {
		return []string{}, nil
	}

	content := string(data)
	if content == "" {
		return []string{}, nil
	}
	lines := normalizeTailLines(strings.Split(content, "\n"))
	return tailLines(lines, limit), nil
}

func tailLines(lines []string, limit int) []string {
	if limit <= 0 {
		return []string{}
	}
	if len(lines) <= limit {
		return lines
	}
	return lines[len(lines)-limit:]
}

func normalizeTailLines(lines []string) []string {
	if len(lines) == 0 {
		return lines
	}
	normalized := make([]string, len(lines))
	for i, line := range lines {
		normalized[i] = strings.TrimSuffix(line, "\r")
	}
	return normalized
}

func (l *logsView) setViewportContentIfChanged(content string, wasAtBottom bool) {
	if content == l.lastRenderedContent {
		return
	}
	l.viewport.SetContent(content)
	l.viewportSetContentCalls++
	l.lastRenderedContent = content
	if wasAtBottom {
		l.viewport.GotoBottom()
	}
}

func (l *logsView) CycleLevelFilter() {
	l.SetLevelFilter(nextCycleValue(normalizeLogLevel(l.filters.Level), logsLevelCycle))
}

func (l *logsView) CycleComponentFilter() {
	components := l.availableComponents()
	l.SetComponentFilter(nextCycleValue(l.filters.Component, components))
}

func (l *logsView) ClearFilters() {
	if l.filters.Level == "" && l.filters.Component == "" {
		return
	}
	l.filters.Level = ""
	l.filters.Component = ""
	l.refreshContent()
}

func (l *logsView) SetLevelFilter(level string) {
	normalized := normalizeLogLevel(level)
	if l.filters.Level == normalized {
		return
	}
	l.filters.Level = normalized
	l.refreshContent()
}

func (l *logsView) SetComponentFilter(component string) {
	normalized := normalizeComponent(component)
	if l.filters.Component == normalized {
		return
	}
	l.filters.Component = normalized
	l.refreshContent()
}

func (l *logsView) refreshContent() {
	atBottom := l.viewport.AtBottom()
	l.setViewportContentIfChanged(l.renderContent(), atBottom)
}

func (l *logsView) renderLogLines(lines []string, fallback string, cache *logLineCache, opts logSectionRenderOptions) string {
	if len(lines) == 0 {
		if cache != nil {
			cache.Reset()
		}
		return fallback
	}
	if cache != nil {
		cache.Update(lines)
	}
	var parsed []parsedLogLine
	if cache != nil {
		parsed = cache.parsed
	} else {
		parsed = parseLogLines(lines)
	}
	if len(parsed) == 0 {
		return fallback
	}
	out := make([]string, 0, len(parsed))
	useFormat := l.format == logsFormatFormatted && opts.AllowFormat
	for _, entry := range parsed {
		if opts.AllowFilters && !l.logLinePassesFilters(entry, opts.KeepUnparsedWhenFiltering) {
			continue
		}
		if useFormat && entry.formatted != "" {
			out = append(out, entry.formatted)
			continue
		}
		out = append(out, entry.raw)
	}
	if len(out) == 0 {
		return fallback
	}
	return strings.Join(out, "\n")
}

func (l *logsView) logLinePassesFilters(entry parsedLogLine, keepUnparsed bool) bool {
	if l.filters.Level == "" && l.filters.Component == "" {
		return true
	}
	if !entry.parsed {
		return keepUnparsed
	}
	if l.filters.Level != "" && entry.level != l.filters.Level {
		return false
	}
	if l.filters.Component != "" && entry.component != l.filters.Component {
		return false
	}
	return true
}

func (l *logsView) availableComponents() []string {
	l.debugCache.Update(l.debugLines)
	l.loopCache.Update(l.loopLines)
	l.specsCache.Update(l.specsLines)
	components := make(map[string]struct{})
	caches := []*logLineCache{&l.debugCache, &l.loopCache, &l.specsCache}
	for _, cache := range caches {
		for _, entry := range cache.parsed {
			if !entry.parsed || entry.component == "" {
				continue
			}
			components[entry.component] = struct{}{}
		}
	}
	available := make([]string, 0, len(components)+1)
	for component := range components {
		available = append(available, component)
	}
	sort.Strings(available)
	available = append([]string{""}, available...)
	return available
}

func formatLogEntry(entry logEntry, fallback string) string {
	if entry.Message == "" && entry.Level == "" && entry.Timestamp == "" && len(entry.Fields) == 0 {
		return fallback
	}
	timestamp := formatLogTimestamp(entry.Timestamp)
	level := strings.ToUpper(strings.TrimSpace(entry.Level))
	message := strings.TrimSpace(entry.Message)
	fields := formatLogFields(entry.Fields)

	parts := make([]string, 0, 4)
	if timestamp != "" {
		parts = append(parts, timestamp)
	}
	if level != "" {
		parts = append(parts, level)
	}
	if message != "" {
		parts = append(parts, message)
	}
	lineOut := strings.Join(parts, " ")
	if fields != "" {
		if lineOut == "" {
			lineOut = fields
		} else {
			lineOut = lineOut + " | " + fields
		}
	}
	if lineOut == "" {
		return fallback
	}
	return lineOut
}

func parseLogLines(lines []string) []parsedLogLine {
	parsed := make([]parsedLogLine, len(lines))
	for i, line := range lines {
		parsed[i] = parseLogLine(line)
	}
	return parsed
}

func parseLogLine(raw string) parsedLogLine {
	trimmed := strings.TrimSpace(raw)
	if trimmed == "" {
		return parsedLogLine{raw: raw, parsed: false, formatted: raw}
	}
	var entry logEntry
	if err := json.Unmarshal([]byte(trimmed), &entry); err != nil {
		return parsedLogLine{raw: raw, parsed: false, formatted: raw}
	}
	if entry.Message == "" && entry.Level == "" && entry.Timestamp == "" && len(entry.Fields) == 0 {
		return parsedLogLine{raw: raw, parsed: false, formatted: raw}
	}
	return parsedLogLine{
		raw:       raw,
		parsed:    true,
		level:     normalizeLogLevel(entry.Level),
		component: componentFromMessage(entry.Message),
		formatted: formatLogEntry(entry, raw),
	}
}

func normalizeLogLevel(level string) string {
	return strings.ToLower(strings.TrimSpace(level))
}

func normalizeComponent(component string) string {
	return strings.ToLower(strings.TrimSpace(component))
}

func componentFromMessage(message string) string {
	message = strings.TrimSpace(message)
	if message == "" {
		return ""
	}
	parts := strings.SplitN(message, ".", 2)
	if len(parts) == 0 {
		return ""
	}
	return normalizeComponent(parts[0])
}

func filterLabel(value string) string {
	if value == "" {
		return "all"
	}
	return value
}

func nextCycleValue(current string, cycle []string) string {
	if len(cycle) == 0 {
		return current
	}
	for i, value := range cycle {
		if value == current {
			return cycle[(i+1)%len(cycle)]
		}
	}
	return cycle[0]
}

func formatLogTimestamp(raw string) string {
	raw = strings.TrimSpace(raw)
	if raw == "" {
		return ""
	}
	parsed, err := time.Parse(time.RFC3339Nano, raw)
	if err != nil {
		return raw
	}
	return parsed.UTC().Format("2006-01-02 15:04:05Z")
}

func formatLogFields(fields map[string]any) string {
	if len(fields) == 0 {
		return ""
	}
	keys := make([]string, 0, len(fields))
	for key := range fields {
		keys = append(keys, key)
	}
	sort.Strings(keys)
	parts := make([]string, 0, len(keys))
	for _, key := range keys {
		value := formatLogFieldValue(fields[key])
		if value == "" {
			parts = append(parts, key+"=")
			continue
		}
		parts = append(parts, fmt.Sprintf("%s=%s", key, value))
	}
	return strings.Join(parts, " ")
}

func formatLogFieldValue(value any) string {
	switch typed := value.(type) {
	case string:
		if strings.ContainsAny(typed, " \t\n") {
			return strconv.Quote(typed)
		}
		return typed
	default:
		return fmt.Sprint(value)
	}
}
