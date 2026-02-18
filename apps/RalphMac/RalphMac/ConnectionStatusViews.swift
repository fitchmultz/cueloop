/**
 ConnectionStatusViews

 Responsibilities:
 - Display offline/connection status indicators for CLI availability.
 - Provide inline banner when CLI is unavailable or workspace is inaccessible.
 - Provide smaller inline indicator for sidebars and toolbars.

 Does not handle:
 - Connection retry logic (delegated to parent via closures).
 - Health checking (handled by Workspace).

 Invariants/assumptions callers must respect:
 - Status is passed in as CLIHealthStatus.
 - Actions are provided via closures for retry/dismiss.
 */

import SwiftUI
import RalphCore

/// Inline banner when CLI is unavailable or workspace is inaccessible
struct OfflineStatusView: View {
    let status: CLIHealthStatus
    let onRetry: () -> Void
    let onDismiss: (() -> Void)?

    var body: some View {
        HStack(spacing: 12) {
            Image(systemName: iconName)
                .font(.system(size: 16, weight: .semibold))
                .foregroundStyle(iconColor)

            VStack(alignment: .leading, spacing: 2) {
                Text(title)
                    .font(.system(size: 13, weight: .semibold))
                    .foregroundStyle(.primary)

                if let subtitle = subtitle {
                    Text(subtitle)
                        .font(.system(size: 11))
                        .foregroundStyle(.secondary)
                        .lineLimit(1)
                }
            }

            Spacer()

            Button(action: onRetry) {
                Image(systemName: "arrow.clockwise")
                    .font(.system(size: 12, weight: .medium))
            }
            .buttonStyle(.borderless)
            .help("Retry connection")

            if let onDismiss = onDismiss {
                Button(action: onDismiss) {
                    Image(systemName: "xmark")
                        .font(.system(size: 10, weight: .medium))
                }
                .buttonStyle(.borderless)
                .help("Dismiss")
            }
        }
        .padding(.horizontal, 16)
        .padding(.vertical, 10)
        .background(backgroundView)
        .overlay(
            Rectangle()
                .frame(height: 1)
                .foregroundStyle(borderColor.opacity(0.3)),
            alignment: .bottom
        )
    }

    @ViewBuilder
    private var backgroundView: some View {
        RoundedRectangle(cornerRadius: 0)
            .fill(
                LinearGradient(
                    colors: [
                        backgroundColor.opacity(0.15),
                        backgroundColor.opacity(0.05)
                    ],
                    startPoint: .top,
                    endPoint: .bottom
                )
            )
    }

    private var iconName: String {
        switch status.availability {
        case .available:
            return "checkmark.circle.fill"
        case .unavailable(let reason):
            switch reason {
            case .cliNotFound, .cliNotExecutable:
                return "terminal.fill"
            case .workspaceInaccessible:
                return "folder.badge.questionmark"
            case .permissionDenied:
                return "lock.fill"
            case .timeout:
                return "clock.badge.exclamationmark.fill"
            case .unknown:
                return "exclamationmark.triangle.fill"
            }
        case .unknown:
            return "questionmark.circle.fill"
        }
    }

    private var iconColor: Color {
        switch status.availability {
        case .available:
            return .green
        case .unavailable(let reason):
            switch reason {
            case .cliNotFound, .cliNotExecutable:
                return .orange
            case .workspaceInaccessible, .permissionDenied:
                return .red
            case .timeout:
                return .yellow
            case .unknown:
                return .gray
            }
        case .unknown:
            return .gray
        }
    }

    private var backgroundColor: Color { iconColor }
    private var borderColor: Color { iconColor }

    private var title: String {
        switch status.availability {
        case .available:
            return "Connected"
        case .unavailable(let reason):
            switch reason {
            case .cliNotFound, .cliNotExecutable:
                return "Ralph CLI Unavailable"
            case .workspaceInaccessible:
                return "Workspace Inaccessible"
            case .permissionDenied:
                return "Permission Denied"
            case .timeout:
                return "Connection Timed Out"
            case .unknown:
                return "Connection Issue"
            }
        case .unknown:
            return "Checking Connection..."
        }
    }

    private var subtitle: String? {
        switch status.availability {
        case .available:
            return "All systems operational"
        case .unavailable(let reason):
            switch reason {
            case .cliNotFound:
                return "The ralph executable could not be found"
            case .cliNotExecutable:
                return "The ralph executable is not runnable"
            case .workspaceInaccessible:
                return "Cannot access the workspace directory"
            case .permissionDenied:
                return "Check file permissions for this workspace"
            case .timeout:
                return "The operation took too long to respond"
            case .unknown(let description):
                return description
            }
        case .unknown:
            return nil
        }
    }
}

/// Smaller inline indicator for use in sidebars/toolbars
struct ConnectionStatusIndicator: View {
    let isAvailable: Bool
    let onTap: () -> Void

    var body: some View {
        Button(action: onTap) {
            HStack(spacing: 6) {
                Circle()
                    .fill(isAvailable ? Color.green : Color.orange)
                    .frame(width: 8, height: 8)

                Text(isAvailable ? "Connected" : "Offline")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
        }
        .buttonStyle(.plain)
        .help(isAvailable ? "CLI is available" : "CLI is unavailable - click for details")
    }
}
