// Package prompts provides embedded default prompt templates for the loop runner.
package prompts

import (
	"embed"
	"fmt"

	"github.com/mitchfultz/ralph/ralph_tui/internal/project"
)

type Runner string

const (
	RunnerCodex    Runner = "codex"
	RunnerOpencode Runner = "opencode"
)

//go:embed defaults/*
var defaultPrompts embed.FS

// WorkerPrompt returns the default worker prompt content for a runner and project type.
func WorkerPrompt(runner Runner, projectType project.Type) (string, error) {
	normalized := project.NormalizeType(string(projectType))
	if normalized == "" {
		normalized = project.DefaultType()
	}
	if !project.ValidType(normalized) {
		return "", fmt.Errorf("unsupported project type: %s", projectType)
	}

	filename, err := workerPromptFilename(runner, normalized)
	if err != nil {
		return "", err
	}

	content, err := defaultPrompts.ReadFile(filename)
	if err != nil {
		return "", err
	}
	return string(content), nil
}

// SupervisorPrompt returns the default supervisor prompt content for a project type.
func SupervisorPrompt(projectType project.Type) (string, error) {
	normalized := project.NormalizeType(string(projectType))
	if normalized == "" {
		normalized = project.DefaultType()
	}
	if !project.ValidType(normalized) {
		return "", fmt.Errorf("unsupported project type: %s", projectType)
	}

	content, err := defaultPrompts.ReadFile("defaults/supervisor_prompt.md")
	if err != nil {
		return "", err
	}
	return string(content), nil
}

func workerPromptFilename(runner Runner, projectType project.Type) (string, error) {
	switch runner {
	case RunnerCodex:
		if projectType == project.TypeDocs {
			return "defaults/prompt_codex_docs.md", nil
		}
		return "defaults/prompt_codex.md", nil
	case RunnerOpencode:
		if projectType == project.TypeDocs {
			return "defaults/prompt_opencode_docs.md", nil
		}
		return "defaults/prompt_opencode.md", nil
	default:
		return "", fmt.Errorf("unsupported runner: %s", runner)
	}
}
