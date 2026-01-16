package prompts

import (
	"fmt"

	"github.com/mitchfultz/ralph/ralph_tui/internal/project"
)

const (
	bugSweepCodePath = "defaults/specs_bug_sweep_code.md"
	bugSweepDocsPath = "defaults/specs_bug_sweep_docs.md"
)

// BugSweepEntry returns the default bug-sweep prompt entry for a project type.
func BugSweepEntry(projectType project.Type) (string, error) {
	normalized := project.NormalizeType(string(projectType))
	if normalized == "" {
		normalized = project.DefaultType()
	}
	if !project.ValidType(normalized) {
		return "", fmt.Errorf("unsupported project type: %s", projectType)
	}

	filename := bugSweepCodePath
	if normalized == project.TypeDocs {
		filename = bugSweepDocsPath
	}
	content, err := defaultPrompts.ReadFile(filename)
	if err != nil {
		return "", err
	}
	return string(content), nil
}
