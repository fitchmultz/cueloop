// Package tui provides tests for specs view preview refresh behavior.
package tui

import "testing"

func TestSpecsPreviewRefreshSetsLoading(t *testing.T) {
	_, locs, cfg := newHermeticModel(t)
	view, err := newSpecsView(cfg, locs)
	if err != nil {
		t.Fatalf("newSpecsView failed: %v", err)
	}
	view.previewDirty = true

	cmd := view.refreshPreviewAsync()
	if cmd == nil {
		t.Fatalf("expected refreshPreviewAsync to return a command")
	}
	if !view.previewLoading {
		t.Fatalf("expected previewLoading to be true")
	}
	if view.previewDirty {
		t.Fatalf("expected previewDirty to be false")
	}
}
