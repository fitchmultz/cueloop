/**
 Purpose:
 - Cover app-state regressions that depend on the shared seeded workspace and relaunch behavior.

 Responsibilities:
 - Verify the seeded fixture tasks render on launch.
 - Validate CLI-side workspace mutations appear after an app relaunch.

 Scope:
 - Workspace state and launch/relaunch visibility only.

 Usage:
 - Runs against the isolated UI-test workspace created by `RalphMacUITestCase`.

 Invariants/Assumptions:
 - Tests mutate only the disposable workspace created for the current test run.
 */

import XCTest

@MainActor
final class RalphMacUIAppStateTests: RalphMacUITestCase {
    func test_seededWorkspaceDisplaysFixtureTasks() throws {
        let taskList = requireTaskList()
        let alphaTask = taskText("UI Fixture Alpha", in: taskList)
        let searchTask = taskText("UI Fixture Search Test", in: taskList)

        assertExists(alphaTask, message: "Seeded alpha fixture task should appear on launch")
        assertExists(searchTask, message: "Seeded search fixture task should appear on launch")
    }

    func test_relaunchReflectsCliWorkspaceMutation() throws {
        guard let uiTestWorkspaceURL else {
            XCTFail("Expected a UI test workspace URL")
            return
        }

        let importURL = uiTestWorkspaceURL.appendingPathComponent("ui-app-state-import.json")
        let newTitle = "UI Relaunch Imported Task"
        let payload = #"""
        [
          {
            "id": "RQ-0099",
            "status": "todo",
            "title": "\#(newTitle)",
            "priority": "medium",
            "created_at": "2026-03-05T01:00:00Z",
            "updated_at": "2026-03-05T01:00:00Z"
          }
        ]
        """#
        try payload.write(to: importURL, atomically: true, encoding: .utf8)
        defer { XCTAssertNoThrow(try removeItemIfExists(importURL)) }

        try runRalph(
            arguments: ["queue", "import", "--format", "json", "--input", importURL.path],
            currentDirectoryURL: uiTestWorkspaceURL
        )
        XCTAssertTrue(
            try uiTestWorkspaceTasks().contains(where: { $0.title == newTitle }),
            "CLI import should update the disposable workspace before relaunch"
        )

        relaunchApp()
        let searchField = taskSearchField
        assertExists(searchField, message: "Task search field should appear after relaunch")
        searchField.click()
        searchField.typeText(newTitle)

        let taskList = requireTaskList()
        assertExists(
            taskText(newTitle, in: taskList),
            timeout: 8,
            message: "Relaunch should surface CLI-imported workspace tasks through the refreshed search index"
        )
    }

    func test_urlOpenBootstrapWorkspaceImmediatelyShowsNewWorkspaceQueue() throws {
        let placeholderWorkspaceURL = try makeAdditionalUITestWorkspace()
        let targetWorkspaceURL = try makeAdditionalUITestWorkspace()
        defer { XCTAssertNoThrow(try removeItemIfExists(placeholderWorkspaceURL)) }
        defer { XCTAssertNoThrow(try removeItemIfExists(targetWorkspaceURL)) }

        let importURL = targetWorkspaceURL.appendingPathComponent("ui-url-open-import.json")
        let newTitle = "UI URL Open Imported Task"
        let payload = #"""
        [
          {
            "id": "RQ-0200",
            "status": "todo",
            "title": "\#(newTitle)",
            "priority": "high",
            "created_at": "2026-03-07T01:00:00Z",
            "updated_at": "2026-03-07T01:00:00Z"
          }
        ]
        """#
        try payload.write(to: importURL, atomically: true, encoding: .utf8)
        defer { XCTAssertNoThrow(try removeItemIfExists(importURL)) }

        try runRalph(
            arguments: ["queue", "import", "--format", "json", "--input", importURL.path],
            currentDirectoryURL: targetWorkspaceURL
        )

        stopTimelineCapture()
        app.terminate()
        let relaunchedApp = XCUIApplication()
        relaunchedApp.launchArguments = ["--uitesting"]
        relaunchedApp.launchEnvironment[LaunchEnvironment.uiTestWorkspacePath] =
            placeholderWorkspaceURL.path
        if let ralphExecutableURL {
            relaunchedApp.launchEnvironment[LaunchEnvironment.ralphBinPath] = ralphExecutableURL.path
        }
        app = relaunchedApp
        app.launch()
        app.activate()
        startTimelineCaptureIfNeeded()
        _ = currentWorkspaceWindow()

        try openWorkspaceURLInApp(targetWorkspaceURL)

        let searchField = taskSearchField
        assertExists(searchField, timeout: 8, message: "Task search field should appear after URL-open retarget")
        let taskList = requireTaskList(timeout: 8)
        assertExists(
            taskText(newTitle, in: taskList),
            timeout: 8,
            message: "Bootstrap URL-open should immediately show the new workspace queue without a manual refresh"
        )
    }
}
