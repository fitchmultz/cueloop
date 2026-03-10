//! Workspace+GraphData
//!
//! Responsibilities:
//! - Load dependency graph data from the Ralph CLI.
//!
//! Does not handle:
//! - Graph layout or visualization.
//! - Queue task loading or mutations.
//! - Analytics loading.
//!
//! Invariants/assumptions callers must respect:
//! - Graph payloads must conform to `RalphGraphDocument`.
//! - Errors are surfaced through the workspace recovery state.

import Foundation

public extension Workspace {
    func loadGraphData(retryConfiguration: RetryConfiguration = .default) async {
        let repositoryContext = currentRepositoryContext()

        guard let client else {
            guard isCurrentRepositoryContext(repositoryContext) else { return }
            insightsState.graphDataErrorMessage = "CLI client not available."
            return
        }

        insightsState.graphDataLoading = true
        insightsState.graphDataErrorMessage = nil

        do {
            let helper = RetryHelper(configuration: retryConfiguration)
            let collected = try await helper.execute(
                operation: { [self] in
                    let result = try await client.runAndCollect(
                        arguments: ["--no-color", "queue", "graph", "--format", "json"],
                        currentDirectoryURL: identityState.workingDirectoryURL
                    )
                    if result.status.code != 0 {
                        throw result.toError()
                    }
                    return result
                },
                onProgress: { [weak self] attempt, maxAttempts, _ in
                    await MainActor.run { [weak self] in
                        self?.insightsState.graphDataErrorMessage =
                            "Retrying load graph (attempt \(attempt)/\(maxAttempts))..."
                    }
                }
            )

            guard collected.status.code == 0 else {
                guard isCurrentRepositoryContext(repositoryContext) else { return }
                insightsState.graphDataErrorMessage = collected.stderr.isEmpty
                    ? "Failed to load graph data (exit \(collected.status.code))."
                    : collected.stderr
                insightsState.graphDataLoading = false
                return
            }

            let graphData = try JSONDecoder().decode(
                RalphGraphDocument.self,
                from: Data(collected.stdout.utf8)
            )
            guard isCurrentRepositoryContext(repositoryContext) else { return }
            insightsState.graphData = graphData
        } catch {
            guard isCurrentRepositoryContext(repositoryContext) else { return }
            let recoveryError = RecoveryError.classify(
                error: error,
                operation: "loadGraphData",
                workspaceURL: identityState.workingDirectoryURL
            )
            insightsState.graphDataErrorMessage = recoveryError.message
            diagnosticsState.lastRecoveryError = recoveryError
            diagnosticsState.showErrorRecovery = true
        }

        guard isCurrentRepositoryContext(repositoryContext) else { return }
        insightsState.graphDataLoading = false
    }
}
