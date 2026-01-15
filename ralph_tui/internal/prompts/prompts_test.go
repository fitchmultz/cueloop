package prompts

import (
	"strings"
	"testing"
)

func TestWorkerPromptsIncludeSafetySections(t *testing.T) {
	sections := []string{
		"PRE-FLIGHT SAFETY (DIRTY REPO)",
		"STOP/CANCEL SEMANTICS",
		"END-OF-TURN CHECKLIST",
	}
	runners := []Runner{RunnerCodex, RunnerOpencode}

	for _, runner := range runners {
		content, err := WorkerPrompt(runner)
		if err != nil {
			t.Fatalf("failed to load worker prompt for %s: %v", runner, err)
		}
		for _, section := range sections {
			if !strings.Contains(content, section) {
				t.Fatalf("worker prompt for %s missing section %q", runner, section)
			}
		}
	}
}

func TestSupervisorPromptIncludesRepairPriority(t *testing.T) {
	content, err := SupervisorPrompt()
	if err != nil {
		t.Fatalf("failed to load supervisor prompt: %v", err)
	}
	section := "MECHANICAL REPAIR PRIORITY (BEFORE QUARANTINE)"
	if !strings.Contains(content, section) {
		t.Fatalf("supervisor prompt missing section %q", section)
	}
}
