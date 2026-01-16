package taskbuilder

import (
	"fmt"
	"strings"
	"unicode/utf8"

	"github.com/mitchfultz/ralph/ralph_tui/internal/prompts"
)

// FormatOptions controls queue item formatting.
type FormatOptions struct {
	ID          string
	Tags        []string
	Description string
	Scope       string
	Prompt      string
	Recon       ReconResult
}

// FormatQueueItemBlock returns a block that satisfies pin queue validation rules.
func FormatQueueItemBlock(opts FormatOptions) ([]string, error) {
	if strings.TrimSpace(opts.ID) == "" {
		return nil, fmt.Errorf("queue ID required")
	}
	if strings.TrimSpace(opts.Description) == "" {
		return nil, fmt.Errorf("queue description required")
	}
	tags := normalizeTags(opts.Tags)
	if len(tags) == 0 {
		return nil, fmt.Errorf("at least one routing tag required")
	}
	scope := normalizeScope(opts.Scope)

	header := fmt.Sprintf("- [ ] %s %s: %s %s", opts.ID, formatTags(tags), opts.Description, scope)

	evidenceTemplate, err := prompts.TaskBuilderEvidenceTemplate()
	if err != nil {
		return nil, err
	}
	planTemplate, err := prompts.TaskBuilderPlanTemplate()
	if err != nil {
		return nil, err
	}

	replacements := map[string]string{
		"PROMPT":       summarizePrompt(opts.Prompt),
		"REPO_SUMMARY": formatRepoSummary(opts.Recon),
		"FILE_SUMMARY": formatFileSummary(opts.Recon),
		"PROJECT_TYPE": string(opts.Recon.DetectedProjectType),
		"SCOPE":        strings.Trim(scope, "()"),
	}

	evidenceLines, err := expandTemplateLines(evidenceTemplate, replacements)
	if err != nil {
		return nil, err
	}
	planLines, err := expandTemplateLines(planTemplate, replacements)
	if err != nil {
		return nil, err
	}

	lines := []string{header}
	lines = append(lines, "  - Evidence: Prompt input and repo recon.")
	lines = appendIndentedLines(lines, evidenceLines)
	lines = append(lines, "  - Plan: Deliver the work scoped by this prompt.")
	lines = appendIndentedLines(lines, planLines)
	return lines, nil
}

func normalizeScope(scope string) string {
	trimmed := strings.TrimSpace(scope)
	if trimmed == "" {
		trimmed = "repo"
	}
	trimmed = strings.TrimSpace(strings.Trim(trimmed, "()"))
	return fmt.Sprintf("(%s)", trimmed)
}

func normalizeTags(tags []string) []string {
	normalized := make([]string, 0, len(tags))
	seen := make(map[string]struct{}, len(tags))
	for _, tag := range tags {
		value := strings.ToLower(strings.TrimSpace(tag))
		if value == "" {
			continue
		}
		if _, ok := seen[value]; ok {
			continue
		}
		seen[value] = struct{}{}
		normalized = append(normalized, value)
	}
	return normalized
}

func formatTags(tags []string) string {
	parts := make([]string, 0, len(tags))
	for _, tag := range tags {
		parts = append(parts, fmt.Sprintf("[%s]", tag))
	}
	return strings.Join(parts, " ")
}

func appendIndentedLines(lines []string, extra []string) []string {
	for _, line := range extra {
		if strings.TrimSpace(line) == "" {
			continue
		}
		lines = append(lines, "    - "+line)
	}
	return lines
}

func summarizePrompt(prompt string) string {
	trimmed := strings.TrimSpace(prompt)
	if trimmed == "" {
		return "n/a"
	}
	collapsed := strings.Join(strings.Fields(trimmed), " ")
	return truncateRunes(collapsed, 180)
}

func truncateRunes(value string, max int) string {
	if max <= 0 || value == "" {
		return value
	}
	if utf8.RuneCountInString(value) <= max {
		return value
	}
	runes := []rune(value)
	if max <= 1 {
		return string(runes[:max])
	}
	return string(runes[:max-1]) + "..."
}

func formatRepoSummary(recon ReconResult) string {
	parts := make([]string, 0, 4)
	if strings.TrimSpace(recon.GitBranch) != "" {
		parts = append(parts, fmt.Sprintf("branch %s", recon.GitBranch))
	}
	if strings.TrimSpace(recon.GitStatusSummary) != "" {
		parts = append(parts, recon.GitStatusSummary)
	}
	if recon.GitAheadCount > 0 {
		parts = append(parts, fmt.Sprintf("ahead %d", recon.GitAheadCount))
	}
	if strings.TrimSpace(recon.LastCommitSummary) != "" {
		parts = append(parts, fmt.Sprintf("last commit %s", recon.LastCommitSummary))
	}
	if len(parts) == 0 {
		return "git status unavailable"
	}
	return strings.Join(parts, "; ")
}

func formatFileSummary(recon ReconResult) string {
	limit := recon.FileScanLimit
	scan := fmt.Sprintf("scanned %d files", recon.TotalFiles)
	if recon.FileScanCapped && limit > 0 {
		scan = fmt.Sprintf("scanned first %d files (cap %d)", recon.TotalFiles, limit)
	}
	extSummary := formatExtSummary(recon.ExtCounts, 5)
	if extSummary != "" {
		return fmt.Sprintf("%s; top extensions: %s", scan, extSummary)
	}
	return scan
}

func deriveDescription(prompt string) string {
	trimmed := strings.TrimSpace(prompt)
	if trimmed == "" {
		return ""
	}
	lines := strings.Split(trimmed, "\n")
	first := strings.TrimSpace(lines[0])
	if first == "" {
		first = strings.TrimSpace(strings.Join(lines, " "))
	}
	if first == "" {
		return ""
	}
	return truncateRunes(first, 90)
}

func expandTemplateLines(template string, replacements map[string]string) ([]string, error) {
	resolved := template
	for key, value := range replacements {
		resolved = strings.ReplaceAll(resolved, "{{"+key+"}}", value)
	}
	if strings.Contains(resolved, "{{") {
		return nil, fmt.Errorf("unresolved template placeholders in task builder template")
	}
	lines := strings.Split(resolved, "\n")
	output := make([]string, 0, len(lines))
	for _, line := range lines {
		trimmed := strings.TrimSpace(line)
		if strings.HasPrefix(trimmed, "- ") {
			trimmed = strings.TrimSpace(strings.TrimPrefix(trimmed, "- "))
		}
		if trimmed == "" {
			continue
		}
		output = append(output, trimmed)
	}
	return output, nil
}
