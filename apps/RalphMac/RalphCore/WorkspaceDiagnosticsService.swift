/**
 WorkspaceDiagnosticsService

 Responsibilities:
 - Execute workspace-scoped diagnostics commands used by recovery UI.
 - Load recent Ralph logs through the shared logger using async-friendly APIs.
 - Keep diagnostics and recovery command orchestration out of SwiftUI views.
 - Format continuation documents into human-readable recovery summaries.

 Does not handle:
 - SwiftUI sheet presentation or button state.
 - Error classification.
 - Opening Finder, links, or pasteboard integration.

 Invariants/assumptions callers must respect:
 - Diagnostics run against a live `Workspace` configured on the main actor.
 - Queue validation requires a Ralph queue file in the workspace.
 - Log export may be unavailable on older macOS runtimes.
 */

import Foundation

@MainActor
public enum WorkspaceDiagnosticsService {
    public static func queueValidationOutput(for workspace: Workspace) async -> String {
        guard workspace.hasRalphQueueFile else {
            return "Queue validation skipped\n\nNo `.ralph/queue.jsonc` found in \(workspace.identityState.workingDirectoryURL.path).\nRun `ralph init --non-interactive` in this directory first."
        }

        do {
            let document = try await workspace.validateQueueContinuation()
            return formatQueueValidation(document)
        } catch {
            return "Failed to run queue validation: \(error.localizedDescription)"
        }
    }

    public static func queueRepairPreviewOutput(for workspace: Workspace) async -> String {
        do {
            let document = try await workspace.repairQueueContinuation(dryRun: true)
            return formatContinuationDocument(
                headline: document.continuation.headline,
                detail: document.continuation.detail,
                blocking: document.effectiveBlocking,
                nextSteps: document.continuation.nextSteps,
                body: document.report.prettyPrintedString ?? "No repair report payload was returned."
            )
        } catch {
            return "Failed to preview queue repair: \(error.localizedDescription)"
        }
    }

    public static func queueRestorePreviewOutput(for workspace: Workspace) async -> String {
        do {
            let document = try await workspace.restoreQueueContinuation(dryRun: true)
            let body = document.result?.prettyPrintedString ?? "No restore preview payload was returned."
            return formatContinuationDocument(
                headline: document.continuation.headline,
                detail: document.continuation.detail,
                blocking: document.effectiveBlocking,
                nextSteps: document.continuation.nextSteps,
                body: body
            )
        } catch {
            return "Failed to preview queue restore: \(error.localizedDescription)"
        }
    }

    public static func recentLogs(hours: Int = 2) async -> String {
        guard RalphLogger.shared.canExportLogs else {
            return "Log export requires macOS 12.0+"
        }

        do {
            return try await RalphLogger.shared.exportLogs(hours: hours)
        } catch {
            return "Failed to export logs: \(error.localizedDescription)"
        }
    }

    private static func formatQueueValidation(_ document: MachineQueueValidateDocument) -> String {
        var sections: [String] = [document.continuation.headline, "", document.continuation.detail]

        if let blocking = document.effectiveBlocking {
            sections.append("")
            sections.append("Operator state: \(blocking.status.rawValue)")
            sections.append(blocking.message)
            if !blocking.detail.isEmpty {
                sections.append(blocking.detail)
            }
        }

        if !document.warnings.isEmpty {
            sections.append("")
            sections.append("Warnings:")
            sections.append(contentsOf: document.warnings.map { "- [\($0.taskID)] \($0.message)" })
        }

        if !document.continuation.nextSteps.isEmpty {
            sections.append("")
            sections.append("Next:")
            sections.append(
                contentsOf: document.continuation.nextSteps.enumerated().map { index, step in
                    "\(index + 1). \(step.command) — \(step.detail)"
                }
            )
        }

        return sections.joined(separator: "\n")
    }

    private static func formatContinuationDocument(
        headline: String,
        detail: String,
        blocking: WorkspaceRunnerController.MachineBlockingState?,
        nextSteps: [WorkspaceContinuationAction],
        body: String
    ) -> String {
        var sections: [String] = [headline, "", detail]

        if let blocking {
            sections.append("")
            sections.append("Operator state: \(blocking.status.rawValue)")
            sections.append(blocking.message)
            if !blocking.detail.isEmpty {
                sections.append(blocking.detail)
            }
        }

        if !body.isEmpty {
            sections.append("")
            sections.append(body)
        }

        if !nextSteps.isEmpty {
            sections.append("")
            sections.append("Next:")
            sections.append(
                contentsOf: nextSteps.enumerated().map { index, step in
                    "\(index + 1). \(step.command) — \(step.detail)"
                }
            )
        }

        return sections.joined(separator: "\n")
    }
}

private extension RalphJSONValue {
    var prettyPrintedString: String? {
        guard let data = try? JSONEncoder().encode(self),
              let object = try? JSONSerialization.jsonObject(with: data),
              let prettyData = try? JSONSerialization.data(withJSONObject: object, options: [.prettyPrinted]),
              let string = String(data: prettyData, encoding: .utf8)
        else {
            return nil
        }
        return string
    }
}
