// Package runnerargs centralizes runner argument parsing and reasoning effort handling.
// Entrypoint: ApplyReasoningEffort.
package runnerargs

import (
	"fmt"
	"strings"
)

const opencodePromptFileMessage = "Follow the attached prompt file verbatim."

// RunnerCommand describes the command and args needed to invoke a runner.
type RunnerCommand struct {
	Name            string
	Args            []string
	PromptStdinPath string
}

// BuildRunnerCommand constructs the runner command/args for the given mode.
func BuildRunnerCommand(runner string, runnerArgs []string, prompt string, promptPath string, interactive bool) (RunnerCommand, error) {
	normalized := NormalizeRunner(runner)
	switch normalized {
	case "codex":
		if interactive {
			args := append([]string{}, runnerArgs...)
			args = append(args, prompt)
			return RunnerCommand{Name: "codex", Args: args}, nil
		}
		args := append([]string{"exec"}, runnerArgs...)
		args = append(args, "-")
		return RunnerCommand{Name: "codex", Args: args, PromptStdinPath: promptPath}, nil
	case "opencode":
		if err := ValidateOpencodeArgs(runnerArgs); err != nil {
			return RunnerCommand{}, err
		}
		if interactive {
			args := append([]string{}, runnerArgs...)
			args = append(args, prompt)
			return RunnerCommand{Name: "opencode", Args: args}, nil
		}
		args := append([]string{"run"}, runnerArgs...)
		args = append(args, "--file", promptPath, "--", opencodePromptFileMessage)
		return RunnerCommand{Name: "opencode", Args: args}, nil
	default:
		return RunnerCommand{}, fmt.Errorf("--runner must be codex or opencode (got: %s)", runner)
	}
}

// ValidateOpencodeArgs rejects opencode args that Ralph manages internally.
func ValidateOpencodeArgs(args []string) error {
	invalid := make([]string, 0, 2)
	for _, arg := range args {
		value := strings.TrimSpace(arg)
		if value == "" {
			continue
		}
		switch {
		case value == "run":
			invalid = append(invalid, "run")
		case value == "--":
			invalid = append(invalid, "--")
		case value == "--file" || strings.HasPrefix(value, "--file="):
			invalid = append(invalid, "--file")
		}
	}
	if len(invalid) == 0 {
		return nil
	}
	return fmt.Errorf("opencode runner args must not include %s; Ralph supplies these flags", strings.Join(invalid, ", "))
}
