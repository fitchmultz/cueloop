/**
 RalphMacApp+Support

 Responsibilities:
 - Provide app-level support actions such as log export, crash-report export, and alerts.

 Does not handle:
 - URL routing.
 - Window/bootstrap lifecycle.

 Invariants/assumptions callers must respect:
 - AppKit save panels and alerts must run on the main actor.
 */

import AppKit
import Foundation
import SwiftUI
import RalphCore
import UniformTypeIdentifiers

extension RalphMacApp {
    func exportLogs() {
        guard RalphLogger.shared.canExportLogs else {
            showAlert(title: "Not Available", message: "Log export requires macOS 12 or later.")
            return
        }

        Task { @MainActor in
            do {
                let logContent = try await RalphLogger.shared.exportLogs(hours: 24)
                let savePanel = NSSavePanel()
                savePanel.nameFieldStringValue = "ralph-logs-\(Date().formatted(.iso8601.dateSeparator(.dash).timeSeparator(.omitted))).txt"
                savePanel.allowedContentTypes = [.plainText]

                let result = await savePanel.begin()
                if result == .OK, let url = savePanel.url {
                    do {
                        try logContent.write(to: url, atomically: true, encoding: .utf8)
                    } catch {
                        showAlert(title: "Export Failed", message: "Could not save logs: \(error.localizedDescription)")
                    }
                }
            } catch {
                showAlert(title: "Export Failed", message: "Could not retrieve logs: \(error.localizedDescription)")
            }
        }
    }

    func showCrashReports() {
        let reports = CrashReporter.shared.getAllReports()
        if reports.isEmpty {
            showAlert(title: "No Crash Reports", message: "No crash reports found.")
            return
        }

        let content = CrashReporter.shared.exportAllReports()

        Task { @MainActor in
            let savePanel = NSSavePanel()
            savePanel.nameFieldStringValue = "ralph-crash-reports-\(Date().formatted(.iso8601.dateSeparator(.dash))).txt"
            savePanel.allowedContentTypes = [.plainText]

            let result = await savePanel.begin()
            if result == .OK, let url = savePanel.url {
                do {
                    try content.write(to: url, atomically: true, encoding: .utf8)
                } catch {
                    showAlert(title: "Export Failed", message: "Could not save crash reports: \(error.localizedDescription)")
                }
            }
        }
    }

    func showAlert(title: String, message: String) {
        let alert = NSAlert()
        alert.messageText = title
        alert.informativeText = message
        alert.alertStyle = .informational
        alert.runModal()
    }
}

@MainActor
final class MainWindowService {
    static let shared = MainWindowService()

    private var openMainWindowHandler: (() -> Void)?

    private init() {}

    func register(openWindow: OpenWindowAction) {
        openMainWindowHandler = { openWindow(id: "main") }
    }

    @discardableResult
    func revealOrOpenPrimaryWindow() -> Bool {
        if let window = workspaceWindows().first {
            window.collectionBehavior.insert(.moveToActiveSpace)
            window.makeKeyAndOrderFront(nil)
            NSApp.activate(ignoringOtherApps: true)
            return true
        }

        guard let openMainWindowHandler else { return false }
        openMainWindowHandler()
        return true
    }
    private func workspaceWindows() -> [NSWindow] {
        NSApp.windows
            .filter { $0.identifier?.rawValue.contains("AppWindow") == true }
            .sorted { $0.windowNumber < $1.windowNumber }
    }
}

struct MainWindowOpenActionRegistrar: View {
    @Environment(\.openWindow) private var openWindow

    var body: some View {
        Color.clear
            .frame(width: 0, height: 0)
            .allowsHitTesting(false)
            .task {
                MainWindowService.shared.register(openWindow: openWindow)
            }
    }
}
