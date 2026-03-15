/**
 WorkspaceRunnerController+Execution

 Responsibilities:
 - Schedule workspace run tasks and own subprocess finalization.
 - Resolve next-task selection and loop continuation for app-launched runs.
 - Keep cancellation and repository-retarget cleanup centralized outside the facade file.

 Does not handle:
 - Machine output decoding details.
 - Queue watching.
 */

import Foundation

@MainActor
extension WorkspaceRunnerController {
    func hasPendingRunWork(for workspace: Workspace) -> Bool {
        runTask != nil || activeRun != nil || workspace.runState.isRunning
    }

    func scheduleRunTask(
        preservingConsole: Bool,
        operation: @escaping @MainActor (Workspace, Workspace.RepositoryContext) async -> [String]?
    ) {
        guard let workspace, !workspace.isShutDown else { return }

        runTaskRevision &+= 1
        let revision = runTaskRevision
        let repositoryContext = workspace.currentRepositoryContext()
        runTask = Task { @MainActor [weak self] in
            guard let self else { return }
            defer { self.finishRunTask(revision) }
            await self.executeRunTask(
                revision: revision,
                repositoryContext: repositoryContext,
                preservingConsole: preservingConsole,
                operation: operation
            )
        }
    }

    func executeRunTask(
        revision: UInt64,
        repositoryContext: Workspace.RepositoryContext,
        preservingConsole: Bool,
        operation: @escaping @MainActor (Workspace, Workspace.RepositoryContext) async -> [String]?
    ) async {
        guard
            let workspace,
            !Task.isCancelled,
            !workspace.isShutDown,
            runTaskRevision == revision,
            workspace.isCurrentRepositoryContext(repositoryContext)
        else {
            return
        }

        guard let arguments = await operation(workspace, repositoryContext) else {
            return
        }

        guard
            !Task.isCancelled,
            !workspace.isShutDown,
            runTaskRevision == revision,
            workspace.isCurrentRepositoryContext(repositoryContext)
        else {
            return
        }

        guard let client = workspace.client else {
            workspace.runState.errorMessage = "CLI client not available."
            return
        }

        do {
            let run = try client.start(
                arguments: arguments,
                currentDirectoryURL: workspace.identityState.workingDirectoryURL
            )
            activeRun = run

            guard
                !Task.isCancelled,
                !workspace.isShutDown,
                runTaskRevision == revision,
                workspace.isCurrentRepositoryContext(repositoryContext)
            else {
                activeRun = nil
                await run.cancel()
                return
            }

            workspace.runState.prepareForNewRun(preservingConsole: preservingConsole)
            var machineDecoder = MachineRunOutputDecoder()
            let usesMachineRunEvents = Self.isMachineRunCommand(arguments)

            for await event in run.events {
                if Task.isCancelled || runTaskRevision != revision {
                    await run.cancel()
                    continue
                }
                guard workspace.isCurrentRepositoryContext(repositoryContext), activeRun === run else { continue }
                if usesMachineRunEvents, event.stream == .stdout {
                    for item in machineDecoder.append(event.text) {
                        applyMachineRunOutputItem(item, workspace: workspace)
                    }
                } else {
                    appendConsoleText(event.text, workspace: workspace)
                }
            }

            if usesMachineRunEvents,
               !Task.isCancelled,
               runTaskRevision == revision,
               workspace.isCurrentRepositoryContext(repositoryContext),
               activeRun === run {
                for item in machineDecoder.finish() {
                    applyMachineRunOutputItem(item, workspace: workspace)
                }
            }

            let status = await run.waitUntilExit()
            guard !Task.isCancelled, runTaskRevision == revision else { return }
            finalizeRun(
                status: status,
                run: run,
                repositoryContext: repositoryContext,
                workspace: workspace
            )
        } catch is CancellationError {
            return
        } catch {
            guard workspace.isCurrentRepositoryContext(repositoryContext), runTaskRevision == revision else { return }
            let recoveryError = RecoveryError.classify(
                error: error,
                operation: "run",
                workspaceURL: workspace.identityState.workingDirectoryURL
            )
            workspace.runState.errorMessage = recoveryError.message
            workspace.diagnosticsState.lastRecoveryError = recoveryError
            workspace.diagnosticsState.showErrorRecovery = true
            workspace.runState.isRunning = false
            activeRun = nil
            runCancellationTask = nil
            cancelRequested = false
            workspace.resetExecutionState()
        }
    }

    func finishRunTask(_ revision: UInt64) {
        guard runTaskRevision == revision else { return }
        runTask = nil
    }

    func cancelPendingRunTask() {
        runTask?.cancel()
        runTask = nil
        runTaskRevision &+= 1
        if let workspace, activeRun == nil {
            workspace.runState.isRunning = false
            workspace.resetExecutionState()
        }
    }

    func scheduleRunCancellation(_ run: RalphCLIRun) {
        runCancellationTask?.cancel()
        runCancellationTask = Task { @MainActor [weak self] in
            await run.cancel()
            guard let self, self.activeRun == nil else { return }
            self.runCancellationTask = nil
        }
    }

    func finalizeRun(
        status: RalphCLIExitStatus,
        run: RalphCLIRun,
        repositoryContext: Workspace.RepositoryContext,
        workspace: Workspace
    ) {
        guard workspace.isCurrentRepositoryContext(repositoryContext), activeRun === run else { return }
        workspace.runState.lastExitStatus = status
        workspace.runState.isRunning = false

        if let startTime = workspace.runState.executionStartTime {
            let record = Workspace.ExecutionRecord(
                id: UUID(),
                taskID: workspace.runState.currentTaskID,
                startTime: startTime,
                endTime: Date(),
                exitCode: cancelRequested ? nil : Int(status.code),
                wasCancelled: cancelRequested
            )
            workspace.addToHistory(record)
        }

        let shouldContinueLoop = workspace.runState.isLoopMode
            && !workspace.runState.stopAfterCurrent
            && !cancelRequested
            && status.code == 0

        if status.code != 0 {
            workspace.runState.isLoopMode = false
        }

        activeRun = nil
        runCancellationTask = nil
        cancelRequested = false
        workspace.resetExecutionState()

        if shouldContinueLoop {
            scheduleLoopContinuation()
        }
    }

    func scheduleLoopContinuation() {
        loopContinuationTask?.cancel()
        loopContinuationTask = Task { @MainActor [weak self] in
            guard let self, let workspace = self.workspace else { return }
            loopContinuationTask = nil
            guard workspace.runState.isLoopMode, !workspace.runState.stopAfterCurrent else { return }
            runNextTask(forceDirtyRepo: loopForceDirtyRepo, preservingConsole: true)
        }
    }

    func resolveNextRunnableTaskID(repositoryContext: Workspace.RepositoryContext) async -> String? {
        guard let workspace else { return nil }
        guard let client = workspace.client else { return workspace.nextTask()?.id }

        do {
            let snapshot = try await workspace.decodeMachineRepositoryJSON(
                MachineQueueReadDocument.self,
                client: client,
                machineArguments: ["queue", "read"],
                currentDirectoryURL: repositoryContext.workingDirectoryURL,
                retryConfiguration: .minimal
            )
            guard !Task.isCancelled, workspace.isCurrentRepositoryContext(repositoryContext) else {
                return nil
            }
            workspace.updateResolvedPaths(snapshot.paths)
            if let id = snapshot.nextRunnableTaskID {
                return id
            }
        } catch is CancellationError {
            return nil
        } catch {
            RalphLogger.shared.debug(
                "Failed to resolve runnable task ID: \(error)",
                category: .workspace
            )
        }

        guard workspace.isCurrentRepositoryContext(repositoryContext) else {
            return nil
        }
        return workspace.nextTask()?.id
    }

    nonisolated static func isMachineRunCommand(_ arguments: [String]) -> Bool {
        let filtered = arguments.filter { $0 != "--no-color" }
        return filtered.starts(with: ["machine", "run"])
    }

    nonisolated static func validateMachineConfigResolveVersion(_ version: Int) throws {
        guard version == supportedMachineConfigResolveVersion else {
            throw NSError(
                domain: "RalphMachineContract",
                code: 2,
                userInfo: [
                    NSLocalizedDescriptionKey:
                        "Unsupported machine config resolve version \(version). RalphMac requires version \(supportedMachineConfigResolveVersion)."
                ]
            )
        }
    }
}
