// Package tui provides shared sizing helpers for viewports.
// Entrypoint: resizeViewportToFit.
package tui

import (
	"strings"

	"github.com/charmbracelet/bubbles/viewport"
	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/huh"
	"github.com/charmbracelet/lipgloss"
	"github.com/charmbracelet/x/ansi"
)

var paddedViewportStyle = lipgloss.NewStyle().Padding(0, 1)

type wrappedBlock struct {
	Text            string
	MinRows         int
	BlankLinesAfter int
}

// resizeViewportToFit sets vp.Style and sizes the viewport to fit inside the provided outer bounds.
func resizeViewportToFit(vp *viewport.Model, outerW, outerH int, style lipgloss.Style) {
	if vp == nil {
		return
	}
	vp.Style = style
	frameW, frameH := style.GetFrameSize()
	vp.Width = max(0, outerW-frameW)
	vp.Height = max(0, outerH-frameH)
}

// terminalWrappedHeight returns how many terminal rows text will occupy at the given width.
func terminalWrappedHeight(text string, width int) int {
	if text == "" {
		return 0
	}
	text = strings.TrimRight(text, "\n")
	if text == "" {
		return 0
	}
	if width <= 0 {
		return 0
	}
	lines := strings.Split(text, "\n")
	height := 0
	for _, line := range lines {
		lineWidth := ansi.StringWidth(line)
		if lineWidth == 0 {
			height++
			continue
		}
		rows := (lineWidth + width - 1) / width
		if rows < 1 {
			rows = 1
		}
		height += rows
	}
	return height
}

// chromeHeight computes the total height of stacked blocks, including explicit blank lines.
func chromeHeight(width int, blocks ...wrappedBlock) int {
	total := 0
	for _, block := range blocks {
		height := terminalWrappedHeight(block.Text, width)
		if height < block.MinRows {
			height = block.MinRows
		}
		if height < 0 {
			height = 0
		}
		total += height
		if block.BlankLinesAfter > 0 {
			total += block.BlankLinesAfter
		}
	}
	return total
}

// remainingHeight returns the non-negative remainder after subtracting used from total.
func remainingHeight(total int, used int) int {
	return max(0, total-used)
}

// splitTwoPaneHeight divides available height into top/bottom ratios without overshooting bounds.
func splitTwoPaneHeight(available int, topNumerator int, denominator int) (int, int) {
	if available <= 0 {
		return 0, 0
	}
	if denominator <= 0 {
		denominator = 1
	}
	if topNumerator < 0 {
		topNumerator = 0
	}
	top := available * topNumerator / denominator
	if top < 0 {
		top = 0
	}
	if top > available {
		top = available
	}
	bottom := available - top
	if available >= 2 {
		if top == 0 {
			top = 1
			bottom = available - 1
		} else if bottom == 0 {
			bottom = 1
			top = available - 1
		}
	}
	return top, bottom
}

// resizeHuhFormToFit sizes a Huh form without enforcing a minimum larger than available space.
func resizeHuhFormToFit(form *huh.Form, width int, height int) *huh.Form {
	if form == nil {
		return nil
	}
	if width < 0 {
		width = 0
	}
	if height < 0 {
		height = 0
	}
	form = form.WithWidth(width)
	form = form.WithHeight(height)
	model, _ := form.Update(tea.WindowSizeMsg{Width: width, Height: height})
	if updated, ok := model.(*huh.Form); ok {
		form = updated
	}
	return form
}
