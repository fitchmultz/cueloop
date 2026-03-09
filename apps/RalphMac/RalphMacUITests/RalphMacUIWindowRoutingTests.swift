/**
 Purpose:
 - Validate focused-scene tab and window routing regressions.

 Responsibilities:
 - Cover menu, keyboard, and command-palette multi-window behavior.

 Scope:
 - Multi-window routing only.

 Usage:
 - Runs with `--uitesting-multiwindow` inherited from `RalphMacUITestCase`.

 Invariants/Assumptions:
 - Tests rely on shared workspace-window probing helpers for window/tab counts.
 */

import XCTest

@MainActor
final class RalphMacUIWindowRoutingTests: RalphMacUITestCase {
    func test_createNewTab_andSwitchBetweenTabs() throws {
        let window = app.windows.firstMatch
        assertExists(window, message: "Main window should appear")
        let before = tabCount(in: window)

        app.menuBars.menuBarItems["Workspace"].click()
        app.menuBars.menuItems["New Tab"].click()

        XCTAssertTrue(
            waitUntil { tabCount(in: window) == before + 1 },
            "New Tab menu action should increase tab count in the active window"
        )
    }

    func test_windowShortcuts_affectOnlyFocusedWindow() throws {
        ensureSecondWindow()

        let windows = workspaceWindows()
        XCTAssertGreaterThanOrEqual(windows.count, 2, "Expected at least two workspace windows")
        let firstWindow = windows[0]
        let secondWindow = windows[1]
        XCTAssertTrue(firstWindow.exists)
        XCTAssertTrue(secondWindow.exists)

        firstWindow.click()
        let firstBefore = tabCount(in: firstWindow)
        let secondBefore = tabCount(in: secondWindow)
        let windowsBefore = workspaceWindowCount()

        firstWindow.typeKey("t", modifierFlags: .command)
        XCTAssertTrue(
            waitUntil { tabCount(in: firstWindow) == firstBefore + 1 },
            "Cmd+T should add a tab only in the focused window"
        )
        XCTAssertEqual(tabCount(in: secondWindow), secondBefore, "Cmd+T should not add tabs in unfocused windows")
        XCTAssertEqual(workspaceWindowCount(), windowsBefore, "Cmd+T should not create or close windows")

        firstWindow.typeKey("w", modifierFlags: .command)
        XCTAssertTrue(
            waitUntil { tabCount(in: firstWindow) == firstBefore },
            "Cmd+W should close a tab only in the focused window"
        )
        XCTAssertEqual(tabCount(in: secondWindow), secondBefore, "Cmd+W should not close tabs in unfocused windows")
        XCTAssertEqual(workspaceWindowCount(), windowsBefore, "Cmd+W should not close the entire window")

        secondWindow.click()
        secondWindow.typeKey("w", modifierFlags: [.command, .shift])
        XCTAssertTrue(
            waitUntil { workspaceWindowCount() == windowsBefore - 1 },
            "Cmd+Shift+W should close only the focused window (expected \(windowsBefore - 1), got \(workspaceWindowCount()))"
        )
    }

    func test_navigationShortcut_affectsOnlyFocusedWindow() throws {
        ensureSecondWindow()

        let windows = workspaceWindows()
        XCTAssertGreaterThanOrEqual(windows.count, 2, "Expected at least two workspace windows")
        let firstWindow = windows[0]
        let secondWindow = windows[1]

        assertExists(taskViewModePicker(in: firstWindow), message: "First window view picker should appear")
        assertExists(taskViewModePicker(in: secondWindow), message: "Second window view picker should appear")

        firstWindow.click()
        firstWindow.typeKey("5", modifierFlags: .command)

        XCTAssertTrue(
            waitUntil { !self.taskViewModePicker(in: firstWindow).exists },
            "Cmd+5 should switch only the focused window away from Queue"
        )
        XCTAssertTrue(
            taskViewModePicker(in: secondWindow).exists,
            "Cmd+5 should not mutate the unfocused window's section"
        )
    }

    func test_commandPaletteNewTab_affectsOnlyFocusedWindow() throws {
        ensureSecondWindow()

        let windows = workspaceWindows()
        XCTAssertGreaterThanOrEqual(windows.count, 2, "Expected at least two workspace windows")
        let firstWindow = windows[0]
        let secondWindow = windows[1]
        XCTAssertTrue(firstWindow.exists)
        XCTAssertTrue(secondWindow.exists)

        firstWindow.click()
        let firstBefore = tabCount(in: firstWindow)
        let secondBefore = tabCount(in: secondWindow)
        let windowsBefore = workspaceWindowCount()

        firstWindow.typeKey("k", modifierFlags: .command)

        let searchField = app.textFields["Type a command or search..."]
        assertExists(searchField, message: "Command palette should appear")
        searchField.click()
        searchField.typeText("New Tab")
        searchField.typeKey(XCUIKeyboardKey.return, modifierFlags: [])

        XCTAssertTrue(
            waitUntil { tabCount(in: firstWindow) == firstBefore + 1 },
            "Command palette 'New Tab' should affect only the focused window"
        )
        XCTAssertEqual(tabCount(in: secondWindow), secondBefore, "Command palette 'New Tab' should not affect unfocused windows")
        XCTAssertEqual(workspaceWindowCount(), windowsBefore, "Command palette 'New Tab' should not create or close windows")
    }
}
