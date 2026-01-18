// Package tui provides override regression tests for config reload behavior.
package tui

import (
	"path/filepath"
	"testing"

	"github.com/mitchfultz/ralph/ralph_tui/internal/config"
)

func TestReloadPreservesCLIAndSessionOverrides(t *testing.T) {
	_, locs, _ := newHermeticModel(t)

	repoConfigPath := filepath.Join(locs.RepoRoot, ".ralph", "ralph.json")
	locs.RepoConfigPath = repoConfigPath

	initialRefresh := 12
	repoPartial := config.PartialConfig{
		UI: &config.UIPartial{
			RefreshSeconds: &initialRefresh,
		},
	}
	if err := config.SavePartial(repoConfigPath, repoPartial, config.SaveOptions{RelativeRoot: locs.RepoRoot}); err != nil {
		t.Fatalf("save repo partial: %v", err)
	}

	cliTheme := "cli-theme"
	cliOverrides := config.PartialConfig{
		UI: &config.UIPartial{
			Theme: &cliTheme,
		},
	}
	sessionLevel := "error"
	sessionOverrides := config.PartialConfig{
		Logging: &config.LoggingPartial{
			Level: &sessionLevel,
		},
	}

	cfg, err := config.LoadFromLocations(config.LoadOptions{
		Locations:        locs,
		CLIOverrides:     cliOverrides,
		SessionOverrides: sessionOverrides,
	})
	if err != nil {
		t.Fatalf("load config: %v", err)
	}

	m := newModel(cfg, locs, StartOptions{
		CLIOverrides:     cliOverrides,
		SessionOverrides: sessionOverrides,
	})

	updatedRefresh := 30
	updatedPartial := config.PartialConfig{
		UI: &config.UIPartial{
			RefreshSeconds: &updatedRefresh,
		},
	}
	if err := config.SavePartial(repoConfigPath, updatedPartial, config.SaveOptions{RelativeRoot: locs.RepoRoot}); err != nil {
		t.Fatalf("update repo partial: %v", err)
	}

	msg := m.reloadConfigCmd()()
	updated, _ := m.Update(msg)
	next := updated.(model)

	if next.cfg.UI.Theme != cliTheme {
		t.Fatalf("expected cli override theme %q, got %q", cliTheme, next.cfg.UI.Theme)
	}
	if next.cfg.Logging.Level != sessionLevel {
		t.Fatalf("expected session override log level %q, got %q", sessionLevel, next.cfg.Logging.Level)
	}
	if next.cfg.UI.RefreshSeconds != updatedRefresh {
		t.Fatalf("expected refreshed repo value %d, got %d", updatedRefresh, next.cfg.UI.RefreshSeconds)
	}
}

func TestConfigEditorSessionLayerIncludesCLIAndSessionOverrides(t *testing.T) {
	_, locs, _ := newHermeticModel(t)

	cliTheme := "cli-theme"
	cliOverrides := config.PartialConfig{
		UI: &config.UIPartial{
			Theme: &cliTheme,
		},
	}
	sessionLevel := "error"
	sessionOverrides := config.PartialConfig{
		Logging: &config.LoggingPartial{
			Level: &sessionLevel,
		},
	}

	editor, err := newConfigEditor(locs, cliOverrides, sessionOverrides)
	if err != nil {
		t.Fatalf("new config editor: %v", err)
	}

	if err := editor.resetLayer(layerSession); err != nil {
		t.Fatalf("reset session layer: %v", err)
	}

	if editor.data.UITheme != cliTheme {
		t.Fatalf("expected session layer theme %q, got %q", cliTheme, editor.data.UITheme)
	}
	if editor.data.LogLevel != sessionLevel {
		t.Fatalf("expected session layer log level %q, got %q", sessionLevel, editor.data.LogLevel)
	}
}

func TestConfigEditorSessionLayerSourcesReflectOverrides(t *testing.T) {
	_, locs, _ := newHermeticModel(t)

	cliTheme := "cli-theme"
	cliOverrides := config.PartialConfig{
		UI: &config.UIPartial{
			Theme: &cliTheme,
		},
	}
	sessionLevel := "error"
	sessionOverrides := config.PartialConfig{
		Logging: &config.LoggingPartial{
			Level: &sessionLevel,
		},
	}

	editor, err := newConfigEditor(locs, cliOverrides, sessionOverrides)
	if err != nil {
		t.Fatalf("new config editor: %v", err)
	}

	if err := editor.resetLayer(layerSession); err != nil {
		t.Fatalf("reset session layer: %v", err)
	}

	if got := editor.sourceForKey(fieldUITheme); got != config.SourceCLI {
		t.Fatalf("expected ui.theme source cli, got %q", got)
	}
	if got := editor.sourceForKey(fieldLogLevel); got != config.SourceSession {
		t.Fatalf("expected logging.level source session, got %q", got)
	}
}
