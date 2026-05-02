/**
 Purpose:
 - Cover navigation, search, and keyboard interaction regressions in a single workspace window.

 Responsibilities:
 - Validate decompose access, sidebar traversal, search filtering, and list/kanban keyboard flows.

 Scope:
 - Single-window navigation and keyboard coverage only.

 Usage:
 - Runs against the shared seeded queue and task helpers from `CueLoopMacUITestCase`.

 Invariants/Assumptions:
 - Search and keyboard assertions rely on deterministic UI-condition helpers, not fixed sleeps.
 */

import XCTest

@MainActor
final class CueLoopMacUINavigationAndKeyboardTests: CueLoopMacUITestCase {
    func test_openTaskDecomposeSheet_fromTaskMenu() throws {
        app.menuBars.menuBarItems["Task"].click()
        app.menuBars.menuItems["Decompose Task..."].click()

        let sheet = app.sheets.firstMatch
        assertExists(sheet, message: "Task decompose sheet should appear")
        XCTAssertTrue(sheet.descendants(matching: .textField).matching(identifier: AccessibilityID.taskDecomposeRequestField).firstMatch.exists)
        XCTAssertTrue(sheet.descendants(matching: .button).matching(identifier: AccessibilityID.taskDecomposePreviewButton).firstMatch.exists)
        XCTAssertTrue(sheet.descendants(matching: .button).matching(identifier: AccessibilityID.taskDecomposeWriteButton).firstMatch.exists)
    }

    func test_navigateThroughAllSidebarSections() throws {
        let sidebar = currentWorkspaceWindow().outlines["Main navigation"]
        assertExists(sidebar, message: "Main navigation sidebar should exist")

        let sections = ["Queue", "Quick Actions", "Run Control", "Advanced Runner", "Analytics"]
        for section in sections {
            let sectionItem = sidebar.staticTexts[section]
            XCTAssertTrue(sectionItem.exists, "\(section) should exist in sidebar")
            sectionItem.click()
            assertEventually("\(section) should remain visible after navigation") {
                sectionItem.exists
            }
        }
    }

    func test_taskSearchFunctionality() throws {
        let searchField = taskSearchField
        assertExists(searchField, message: "Task search field should exist")
        let taskList = requireTaskList()

        searchField.click()
        searchField.typeText("Search Test")

        let matchingText = taskText("UI Fixture Search Test", in: taskList)
        let nonMatchingText = taskText("UI Fixture Alpha", in: taskList)
        assertEventually("Search should filter the task list to the matching fixture task") {
            matchingText.exists && !nonMatchingText.exists
        }

        let clearButton = currentWorkspaceWindow().buttons["Clear search"]
        if clearButton.exists {
            clearButton.click()
            assertEventually("Clearing search should restore the full seeded task list") {
                matchingText.exists && nonMatchingText.exists
            }
        }
    }

    func test_taskListKeyboardNavigation() throws {
        _ = createTask(titlePrefix: "Keyboard Flow Task")
        openFirstTaskDetails()

        currentWorkspaceWindow().typeKey(XCUIKeyboardKey.downArrow, modifierFlags: [])
        currentWorkspaceWindow().typeKey(XCUIKeyboardKey.upArrow, modifierFlags: [])
        currentWorkspaceWindow().typeKey(XCUIKeyboardKey.return, modifierFlags: [])
        assertExists(taskDetailTitleField, message: "Task detail title field should appear after opening details")
    }

    func test_kanbanBoardKeyboardNavigation() throws {
        _ = createTask(titlePrefix: "Keyboard Flow Task")

        assertEventually("Task view mode picker should appear") { taskViewModePicker().exists }
        selectTaskViewMode("Kanban")

        let kanbanBoard = currentWorkspaceWindow().scrollViews["Kanban board"]
        assertExists(kanbanBoard, message: "Kanban board should appear")

        let firstCard = kanbanBoard.buttons.firstMatch
        assertExists(firstCard, message: "Expected at least one kanban card")
        firstCard.click()

        currentWorkspaceWindow().typeKey(XCUIKeyboardKey.rightArrow, modifierFlags: [])
        currentWorkspaceWindow().typeKey(XCUIKeyboardKey.leftArrow, modifierFlags: [])
        currentWorkspaceWindow().typeKey(XCUIKeyboardKey.downArrow, modifierFlags: [])
    }
}
