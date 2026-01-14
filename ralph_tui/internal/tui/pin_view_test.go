// Package tui provides tests for pin view reload behavior.
package tui

import "testing"

func TestPinReloadAsyncSetsLoading(t *testing.T) {
	_, locs, cfg := newHermeticModel(t)
	view, err := newPinView(cfg, locs)
	if err != nil {
		t.Fatalf("newPinView failed: %v", err)
	}

	cmd := view.reloadAsync(true)
	if cmd == nil {
		t.Fatalf("expected reloadAsync to return a command")
	}
	if !view.loading {
		t.Fatalf("expected reloadAsync to set loading")
	}
}

func TestPinReloadAsyncQueuesWhenBusy(t *testing.T) {
	_, locs, cfg := newHermeticModel(t)
	view, err := newPinView(cfg, locs)
	if err != nil {
		t.Fatalf("newPinView failed: %v", err)
	}

	view.loading = true
	cmd := view.reloadAsync(true)
	if cmd != nil {
		t.Fatalf("expected nil command when already loading")
	}
	if !view.reloadAgain {
		t.Fatalf("expected reloadAgain to be set when already loading")
	}
}

func TestPinReloadAsyncClearsReloadAgainOnStart(t *testing.T) {
	_, locs, cfg := newHermeticModel(t)
	view, err := newPinView(cfg, locs)
	if err != nil {
		t.Fatalf("newPinView failed: %v", err)
	}

	view.reloadAgain = true
	cmd := view.reloadAsync(true)
	if cmd == nil {
		t.Fatalf("expected reloadAsync to return a command")
	}
	if view.reloadAgain {
		t.Fatalf("expected reloadAgain to clear when starting reload")
	}
}

func TestPinReloadAsyncClearsReloadAgainOnError(t *testing.T) {
	_, locs, cfg := newHermeticModel(t)
	view, err := newPinView(cfg, locs)
	if err != nil {
		t.Fatalf("newPinView failed: %v", err)
	}

	view.loading = true
	view.reloadAgain = true
	_ = view.Update(pinReloadMsg{err: errSentinel}, newTestKeyMap())
	if view.reloadAgain {
		t.Fatalf("expected reloadAgain to clear on reload error")
	}
	if view.loading {
		t.Fatalf("expected loading to clear on reload error")
	}
}
