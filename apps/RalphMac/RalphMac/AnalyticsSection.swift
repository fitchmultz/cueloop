/**
 AnalyticsSection

 Responsibilities:
 - Provide Analytics detail column with productivity summary and milestones.
 - Display analytics data from workspace when available.

 Does not handle:
 - Data fetching or refresh (delegated to Workspace and AnalyticsDashboardView).
 - Chart rendering (see individual chart views like VelocityChartView).

 Invariants/assumptions callers must respect:
 - Workspace is injected via @ObservedObject.
 - Analytics data is populated by parent views.
 */

import SwiftUI
import RalphCore

@MainActor
struct AnalyticsDetailColumn: View {
    @ObservedObject var workspace: Workspace
    let navTitle: (String) -> String

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 20) {
                if let report = workspace.analyticsData.productivitySummary {
                    productivitySummarySection(report: report)

                    if !report.milestones.isEmpty {
                        milestonesSection(report: report)
                    }
                } else {
                    EmptyAnalyticsView()
                }
            }
            .padding(20)
        }
        .background(.clear)
        .navigationTitle(navTitle("Analytics"))
    }

    @ViewBuilder
    private func productivitySummarySection(report: ProductivitySummaryReport) -> some View {
        GlassGroupBox(title: "Productivity Summary") {
            VStack(alignment: .leading, spacing: 8) {
                AnalyticsDetailRow(label: "Total Completed", value: "\(report.totalCompleted)")
                AnalyticsDetailRow(label: "Current Streak", value: "\(report.currentStreak) days")
                AnalyticsDetailRow(label: "Longest Streak", value: "\(report.longestStreak) days")

                if let nextMilestone = report.nextMilestone {
                    AnalyticsDetailRow(label: "Next Milestone", value: "\(nextMilestone) tasks")
                }
            }
        }
    }

    @ViewBuilder
    private func milestonesSection(report: ProductivitySummaryReport) -> some View {
        GlassGroupBox(title: "Milestones Achieved") {
            VStack(alignment: .leading, spacing: 6) {
                ForEach(Array(report.milestones.prefix(5).enumerated()), id: \.offset) { _, milestone in
                    HStack {
                        Image(systemName: milestone.celebrated ? "checkmark.circle.fill" : "circle")
                            .foregroundStyle(milestone.celebrated ? .green : .secondary)
                        Text("\(milestone.threshold) tasks")
                        Spacer()
                        Text(String(milestone.achievedAt.prefix(10)))
                            .font(.caption)
                            .foregroundStyle(.secondary)
                    }
                    .font(.caption)
                }
            }
        }
    }
}

@MainActor
struct EmptyAnalyticsView: View {
    var body: some View {
        VStack(spacing: 16) {
            Image(systemName: "chart.bar")
                .font(.system(size: 48))
                .foregroundStyle(.secondary)

            Text("No Analytics Data")
                .font(.headline)

            Text("Select a time range and refresh to load analytics.")
                .font(.subheadline)
                .foregroundStyle(.secondary)
                .multilineTextAlignment(.center)
                .frame(maxWidth: 300)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background(.clear)
    }
}

@MainActor
struct AnalyticsDetailRow: View {
    let label: String
    let value: String

    var body: some View {
        HStack {
            Text(label)
                .foregroundStyle(.secondary)
            Spacer()
            Text(value)
                .font(.system(.body, design: .monospaced))
        }
    }
}
