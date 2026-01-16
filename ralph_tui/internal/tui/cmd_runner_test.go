// Package tui provides a small command runner for Bubble Tea command tests.
package tui

import (
	"context"
	"reflect"
	"sync"
	"testing"
	"time"

	tea "github.com/charmbracelet/bubbletea"
)

const cmdRunTimeout = 5 * time.Second

func driveCmdsUntil(t *testing.T, m model, cmd tea.Cmd, done func(model) bool) model {
	t.Helper()
	deadline := time.Now().Add(cmdRunTimeout)
	cmdQueue := []tea.Cmd{cmd}
	for len(cmdQueue) > 0 {
		if done(m) {
			return m
		}
		if time.Now().After(deadline) {
			t.Fatalf("timed out waiting for commands to complete")
		}
		next := cmdQueue[0]
		cmdQueue = cmdQueue[1:]
		for _, msg := range runCmd(t, next) {
			updated, nextCmd := m.Update(msg)
			m = unwrapModel(t, updated)
			if nextCmd != nil {
				cmdQueue = append(cmdQueue, nextCmd)
			}
			if done(m) {
				return m
			}
		}
	}
	if !done(m) {
		t.Fatalf("commands exhausted before reaching expected state")
	}
	return m
}

func runCmd(t *testing.T, cmd tea.Cmd) []tea.Msg {
	t.Helper()
	if cmd == nil {
		return nil
	}
	return flattenMsg(t, cmd())
}

func flattenMsg(t *testing.T, msg tea.Msg) []tea.Msg {
	t.Helper()
	switch next := msg.(type) {
	case nil:
		return nil
	case tea.BatchMsg:
		return runBatchCmds(t, []tea.Cmd(next))
	default:
		if cmds, ok := cmdSliceFromMsg(msg); ok {
			return runSequenceCmds(t, cmds)
		}
		return []tea.Msg{msg}
	}
}

func runBatchCmds(t *testing.T, cmds []tea.Cmd) []tea.Msg {
	t.Helper()
	if len(cmds) == 0 {
		return nil
	}
	ctx, cancel := context.WithTimeout(context.Background(), cmdRunTimeout)
	defer cancel()

	msgCh := make(chan tea.Msg, len(cmds))
	var wg sync.WaitGroup
	for _, cmd := range cmds {
		if cmd == nil {
			continue
		}
		wg.Add(1)
		go func(c tea.Cmd) {
			defer wg.Done()
			msgCh <- c()
		}(cmd)
	}
	complete := make(chan struct{})
	go func() {
		wg.Wait()
		close(complete)
	}()

	select {
	case <-complete:
	case <-ctx.Done():
		t.Fatalf("timed out running batched commands")
	}
	close(msgCh)

	messages := make([]tea.Msg, 0, len(cmds))
	for msg := range msgCh {
		messages = append(messages, flattenMsg(t, msg)...)
	}
	return messages
}

func runSequenceCmds(t *testing.T, cmds []tea.Cmd) []tea.Msg {
	t.Helper()
	messages := make([]tea.Msg, 0, len(cmds))
	for _, cmd := range cmds {
		messages = append(messages, runCmd(t, cmd)...)
	}
	return messages
}

func cmdSliceFromMsg(msg tea.Msg) ([]tea.Cmd, bool) {
	value := reflect.ValueOf(msg)
	if !value.IsValid() || value.Kind() != reflect.Slice {
		return nil, false
	}
	cmdType := reflect.TypeOf((tea.Cmd)(nil))
	if value.Type().Elem() != cmdType {
		return nil, false
	}
	cmds := make([]tea.Cmd, value.Len())
	for i := 0; i < value.Len(); i++ {
		cmd, ok := value.Index(i).Interface().(tea.Cmd)
		if !ok {
			return nil, false
		}
		cmds[i] = cmd
	}
	return cmds, true
}

func unwrapModel(t *testing.T, updated tea.Model) model {
	t.Helper()
	switch next := updated.(type) {
	case model:
		return next
	case *model:
		return *next
	default:
		t.Fatalf("unexpected model type %T", updated)
	}
	return model{}
}
