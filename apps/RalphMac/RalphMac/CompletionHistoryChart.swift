/**
 CompletionHistoryChart

 Responsibilities:
 - Render a line chart showing both tasks created and completed over time.
 */

import SwiftUI
import Charts
import RalphCore

struct CompletionHistoryChart: View {
    let history: HistoryReport?
    
    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text("Task Activity")
                .font(.headline)
                .padding(.horizontal)
                .padding(.top)
            
            if let history = history, !history.days.isEmpty {
                Chart {
                    ForEach(history.days, id: \.date) { day in
                        LineMark(
                            x: .value("Date", formatDate(day.date)),
                            y: .value("Created", day.created.count)
                        )
                        .foregroundStyle(.blue)
                        .lineStyle(StrokeStyle(lineWidth: 2))
                        
                        LineMark(
                            x: .value("Date", formatDate(day.date)),
                            y: .value("Completed", day.completed.count)
                        )
                        .foregroundStyle(.green)
                        .lineStyle(StrokeStyle(lineWidth: 2))
                    }
                }
                .chartXAxis {
                    AxisMarks { value in
                        AxisValueLabel {
                            if let dateStr = value.as(String.self) {
                                Text(dateStr)
                                    .font(.caption)
                            }
                        }
                    }
                }
                .chartYAxis {
                    AxisMarks(position: .leading)
                }
                .chartLegend(position: .top, alignment: .trailing)
                .padding()
            } else {
                emptyStateView(message: "No history data available")
            }
        }
    }
    
    private func formatDate(_ dateString: String) -> String {
        let components = dateString.split(separator: "-")
        if components.count == 3 {
            return "\(components[1])-\(components[2])"
        }
        return dateString
    }
    
    @ViewBuilder
    private func emptyStateView(message: String) -> some View {
        VStack {
            Spacer()
            Text(message)
                .foregroundStyle(.secondary)
            Spacer()
        }
        .frame(maxWidth: .infinity)
    }
}
