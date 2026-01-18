package tui

import tea "github.com/charmbracelet/bubbletea"

func isHardQuitKey(msg tea.KeyMsg) bool {
	return msg.Type == tea.KeyCtrlC
}
