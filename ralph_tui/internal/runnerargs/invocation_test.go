// Package runnerargs provides tests for runner invocation helpers.
package runnerargs

import "testing"

func TestBuildRunnerCommandCodexNonInteractive(t *testing.T) {
	cmd, err := BuildRunnerCommand("codex", []string{"-c", "model=foo"}, "", "/tmp/prompt.md", false)
	if err != nil {
		t.Fatalf("BuildRunnerCommand failed: %v", err)
	}
	if cmd.Name != "codex" {
		t.Fatalf("expected codex, got %q", cmd.Name)
	}
	expected := []string{"exec", "-c", "model=foo", "-"}
	if !matchesArgs(cmd.Args, expected) {
		t.Fatalf("expected args %v, got %v", expected, cmd.Args)
	}
	if cmd.PromptStdinPath != "/tmp/prompt.md" {
		t.Fatalf("expected prompt stdin path set, got %q", cmd.PromptStdinPath)
	}
}

func TestBuildRunnerCommandOpencodeNonInteractive(t *testing.T) {
	cmd, err := BuildRunnerCommand("opencode", []string{"--model", "test"}, "", "/tmp/prompt.md", false)
	if err != nil {
		t.Fatalf("BuildRunnerCommand failed: %v", err)
	}
	expected := []string{"run", "--model", "test", "--file", "/tmp/prompt.md", "--", opencodePromptFileMessage}
	if !matchesArgs(cmd.Args, expected) {
		t.Fatalf("expected args %v, got %v", expected, cmd.Args)
	}
}

func TestBuildRunnerCommandInteractive(t *testing.T) {
	cmd, err := BuildRunnerCommand("codex", []string{"--flag"}, "hello", "", true)
	if err != nil {
		t.Fatalf("BuildRunnerCommand failed: %v", err)
	}
	expected := []string{"--flag", "hello"}
	if !matchesArgs(cmd.Args, expected) {
		t.Fatalf("expected args %v, got %v", expected, cmd.Args)
	}
}

func TestValidateOpencodeArgs(t *testing.T) {
	if err := ValidateOpencodeArgs([]string{"--model", "test"}); err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if err := ValidateOpencodeArgs([]string{"run"}); err == nil {
		t.Fatalf("expected error for run arg")
	}
	if err := ValidateOpencodeArgs([]string{"--file", "path"}); err == nil {
		t.Fatalf("expected error for --file arg")
	}
	if err := ValidateOpencodeArgs([]string{"--"}); err == nil {
		t.Fatalf("expected error for -- arg")
	}
}

func matchesArgs(got []string, expected []string) bool {
	if len(got) != len(expected) {
		return false
	}
	for idx, value := range expected {
		if got[idx] != value {
			return false
		}
	}
	return true
}
