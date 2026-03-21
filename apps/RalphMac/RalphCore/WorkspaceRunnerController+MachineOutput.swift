/**
 WorkspaceRunnerController+MachineOutput

 Responsibilities:
 - Decode machine-run envelopes and summaries emitted by the CLI.
 - Apply structured run-event updates to workspace run state and console output.
 - Keep machine-contract helpers separate from runner lifecycle orchestration.

 Does not handle:
 - Process start/stop scheduling.
 - Queue watching or workspace retarget lifecycle.
 */

import Foundation

@MainActor
extension WorkspaceRunnerController {
    func appendConsoleText(_ text: String, workspace: Workspace) {
        workspace.runState.outputBuffer.append(text)
        workspace.runState.output = workspace.runState.outputBuffer.content
        workspace.consumeStreamTextChunk(text)
    }

    func applyMachineRunOutputItem(_ item: MachineRunOutputDecoder.Item, workspace: Workspace) {
        switch item {
        case .event(let event):
            switch event.kind {
            case .runStarted:
                workspace.runState.currentTaskID = event.taskID ?? workspace.runState.currentTaskID
                if let document = event.payload?.decode(MachineConfigResolveDocument.self, for: "config") {
                    workspace.updateResolvedPaths(document.paths)
                    workspace.runState.resumeState = document.resumePreview?.asWorkspaceResumeState()
                }
            case .taskSelected:
                workspace.runState.currentTaskID = event.taskID ?? workspace.runState.currentTaskID
            case .phaseEntered:
                workspace.runState.currentPhase = Workspace.ExecutionPhase(machineValue: event.phase)
            case .phaseCompleted:
                if workspace.runState.currentPhase == Workspace.ExecutionPhase(machineValue: event.phase) {
                    workspace.runState.currentPhase = nil
                }
            case .resumeDecision:
                if let decision = decodeResumeDecision(from: event.payload) {
                    workspace.runState.resumeState = decision.asWorkspaceResumeState()
                    appendResumeDecision(decision, workspace: workspace)
                } else if let message = event.message, !message.isEmpty {
                    appendConsoleText("\(message)\n", workspace: workspace)
                }
            case .runnerOutput:
                if let text = event.payload?.string(for: "text") {
                    appendConsoleText(text, workspace: workspace)
                }
            case .queueSnapshot:
                if let paths = event.payload?.decode(MachineQueuePaths.self, for: "paths") {
                    workspace.updateResolvedPaths(paths)
                }
            case .configResolved:
                if let document = event.payload?.decode(MachineConfigResolveDocument.self, for: "config") {
                    workspace.updateResolvedPaths(document.paths)
                    workspace.runState.resumeState = document.resumePreview?.asWorkspaceResumeState()
                }
            case .warning:
                if let message = event.message, !message.isEmpty {
                    appendConsoleText("[warning] \(message)\n", workspace: workspace)
                }
            case .runFinished:
                break
            }
        case .summary(let summary):
            if let taskID = summary.taskID {
                workspace.runState.currentTaskID = taskID
            }
        case .rawText(let text):
            appendConsoleText(text, workspace: workspace)
        }
    }

    private func decodeResumeDecision(from payload: RalphJSONValue?) -> MachineResumeDecision? {
        let decoder = JSONDecoder()
        decoder.dateDecodingStrategy = .iso8601
        guard let payload else { return nil }
        guard let data = try? JSONEncoder().encode(payload) else { return nil }
        return try? decoder.decode(MachineResumeDecision.self, from: data)
    }

    private func appendResumeDecision(_ decision: MachineResumeDecision, workspace: Workspace) {
        var lines = [decision.message]
        if !decision.detail.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
            lines.append("  \(decision.detail)")
        }
        appendConsoleText(lines.joined(separator: "\n") + "\n", workspace: workspace)
    }
}

extension WorkspaceRunnerController {
    struct MachineRunEventEnvelope: Decodable, Sendable {
        let version: Int
        let kind: Kind
        let taskID: String?
        let phase: String?
        let message: String?
        let payload: RalphJSONValue?

        enum Kind: String, Decodable, Sendable {
            case runStarted = "run_started"
            case queueSnapshot = "queue_snapshot"
            case configResolved = "config_resolved"
            case resumeDecision = "resume_decision"
            case taskSelected = "task_selected"
            case phaseEntered = "phase_entered"
            case phaseCompleted = "phase_completed"
            case runnerOutput = "runner_output"
            case warning
            case runFinished = "run_finished"
        }

        enum CodingKeys: String, CodingKey {
            case version
            case kind
            case taskID = "task_id"
            case phase
            case message
            case payload
        }
    }

    struct MachineRunSummaryDocument: Decodable, Sendable {
        let version: Int
        let taskID: String?
        let exitCode: Int
        let outcome: String

        enum CodingKeys: String, CodingKey {
            case version
            case taskID = "task_id"
            case exitCode = "exit_code"
            case outcome
        }
    }

    struct MachineRunOutputDecoder {
        enum Item {
            case event(MachineRunEventEnvelope)
            case summary(MachineRunSummaryDocument)
            case rawText(String)
        }

        private var buffered = ""

        mutating func append(_ chunk: String) -> [Item] {
            buffered.append(chunk)
            return drainCompleteLines()
        }

        mutating func finish() -> [Item] {
            defer { buffered.removeAll(keepingCapacity: false) }
            guard !buffered.isEmpty else { return [] }
            return decodeLine(buffered)
        }

        private mutating func drainCompleteLines() -> [Item] {
            var items: [Item] = []
            while let newlineIndex = buffered.firstIndex(of: "\n") {
                let line = String(buffered[..<newlineIndex])
                buffered.removeSubrange(...newlineIndex)
                items.append(contentsOf: decodeLine(line))
            }
            return items
        }

        private func decodeLine(_ line: String) -> [Item] {
            let trimmed = line.trimmingCharacters(in: .whitespacesAndNewlines)
            guard !trimmed.isEmpty else { return [] }
            let data = Data(trimmed.utf8)
            let decoder = JSONDecoder()

            if let event = try? decoder.decode(MachineRunEventEnvelope.self, from: data) {
                return [.event(event)]
            }
            if let summary = try? decoder.decode(MachineRunSummaryDocument.self, from: data) {
                return [.summary(summary)]
            }
            return [.rawText(line + "\n")]
        }
    }
}

private extension Workspace.ExecutionPhase {
    init?(machineValue: String?) {
        switch machineValue {
        case "plan":
            self = .plan
        case "implement":
            self = .implement
        case "review":
            self = .review
        default:
            return nil
        }
    }
}

private extension RalphJSONValue {
    func string(for key: String) -> String? {
        guard case .object(let object) = self, let value = object[key] else { return nil }
        return value.stringValue
    }

    func decode<T: Decodable>(_ type: T.Type, for key: String) -> T? {
        guard case .object(let object) = self, let value = object[key] else { return nil }
        let decoder = JSONDecoder()
        decoder.dateDecodingStrategy = .iso8601
        guard let data = try? JSONEncoder().encode(value) else { return nil }
        return try? decoder.decode(type, from: data)
    }
}
