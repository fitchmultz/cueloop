/**
 WorkspaceRunControlWatcherHealthTests

 Purpose:
 - Validate queue watcher health is reflected in workspace operational summaries.

 Responsibilities:
 - Validate queue watcher health is reflected in workspace operational summaries.

 Does not handle:
 - Run invocation, blocking/resume state, parallel status, or loop/cancel behavior.

 Usage:
 - Used by the CueLoopMac app or CueLoopCore tests through its owning feature surface.

 Invariants/assumptions callers must respect:
 - Tests construct in-memory workspaces without mock CLI fixtures.
 */

import XCTest
@testable import CueLoopCore

@MainActor
final class WorkspaceRunControlWatcherHealthTests: WorkspacePerformanceTestCase {
    func test_updateWatcherHealth_startingSurfacesInformationalIssue() {
        let workspace = makeWorkspace(label: "watcher-health-starting")

        workspace.updateWatcherHealth(
            QueueWatcherHealth(
                state: .starting(attempt: 1),
                workingDirectoryURL: workspace.workingDirectoryURL
            )
        )

        XCTAssertEqual(workspace.operationalSummary.severity, .info)
        XCTAssertEqual(workspace.operationalIssues.first?.source, .watcher)
        XCTAssertEqual(workspace.operationalIssues.first?.title, "Queue watcher starting")
        XCTAssertEqual(workspace.operationalIssues.first?.severity, .info)
    }

    func test_updateWatcherHealth_degradedWithScheduledRetryIsInformational() {
        let workspace = makeWorkspace(label: "watcher-health-retrying")
        let nextRetryAt = Date(timeIntervalSince1970: 1_900_000_000)

        workspace.updateWatcherHealth(
            QueueWatcherHealth(
                state: .degraded(
                    reason: "Failed to start FSEvent stream",
                    retryCount: 1,
                    nextRetryAt: nextRetryAt
                ),
                workingDirectoryURL: workspace.workingDirectoryURL
            )
        )

        XCTAssertEqual(workspace.operationalSummary.severity, .info)
        XCTAssertEqual(workspace.operationalIssues.first?.source, .watcher)
        XCTAssertEqual(workspace.operationalIssues.first?.title, "Queue watcher retrying")
        XCTAssertEqual(workspace.operationalIssues.first?.severity, .info)
        XCTAssertEqual(
            workspace.operationalIssues.first?.recoverySuggestion,
            "CueLoop is retrying queue-file observation automatically."
        )
    }

    func test_updateWatcherHealth_degradedWithoutScheduledRetryWarns() {
        let workspace = makeWorkspace(label: "watcher-health-degraded")

        workspace.updateWatcherHealth(
            QueueWatcherHealth(
                state: .degraded(
                    reason: "Queue watcher timer failed",
                    retryCount: 1,
                    nextRetryAt: nil
                ),
                workingDirectoryURL: workspace.workingDirectoryURL
            )
        )

        XCTAssertEqual(workspace.operationalSummary.severity, .warning)
        XCTAssertEqual(workspace.operationalIssues.first?.source, .watcher)
        XCTAssertEqual(workspace.operationalIssues.first?.title, "Queue watcher degraded")
        XCTAssertEqual(workspace.operationalIssues.first?.severity, .warning)
    }

    func test_updateWatcherHealth_failedSurfacesError() {
        let workspace = makeWorkspace(label: "watcher-health-failed")

        workspace.updateWatcherHealth(
            QueueWatcherHealth(
                state: .failed(reason: "stream bootstrap failed", attempts: 3),
                workingDirectoryURL: workspace.workingDirectoryURL
            )
        )

        XCTAssertEqual(workspace.operationalSummary.severity, .error)
        XCTAssertEqual(workspace.operationalIssues.first?.source, .watcher)
        XCTAssertEqual(workspace.operationalIssues.first?.title, "Queue watcher failed")
        XCTAssertEqual(workspace.operationalIssues.first?.severity, .error)
    }

    private func makeWorkspace(label: String) -> Workspace {
        Workspace(
            workingDirectoryURL: CueLoopCoreTestSupport.workspaceURL(label: label)
        )
    }
}
