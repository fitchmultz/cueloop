/**
 TagBreakdownChart

 Responsibilities:
 - Render a pie/donut chart showing task distribution by tags.
 - Uses SwiftUI Charts for visualization.
 */

import SwiftUI
import Charts
import RalphCore

struct TagBreakdownChart: View {
    let tagBreakdown: [TagBreakdown]
    
    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text("Tag Distribution")
                .font(.headline)
                .padding(.horizontal)
                .padding(.top)
            
            if !tagBreakdown.isEmpty {
                HStack(spacing: 20) {
                    // Donut Chart
                    Chart(tagBreakdown, id: \.tag) { item in
                        SectorMark(
                            angle: .value("Count", item.count),
                            innerRadius: .ratio(0.5),
                            angularInset: 1.0
                        )
                        .foregroundStyle(by: .value("Tag", item.tag))
                    }
                    .frame(width: 200, height: 200)
                    .chartLegend(position: .trailing, alignment: .center)
                    
                    // Legend/List
                    VStack(alignment: .leading, spacing: 8) {
                        ForEach(tagBreakdown.prefix(5), id: \.tag) { item in
                            HStack {
                                Circle()
                                    .frame(width: 8, height: 8)
                                Text(item.tag)
                                    .font(.caption)
                                Spacer()
                                Text("\(item.count)")
                                    .font(.caption)
                                    .foregroundStyle(.secondary)
                            }
                        }
                    }
                    .frame(width: 120)
                }
                .padding()
                .frame(maxWidth: .infinity)
            } else {
                emptyStateView(message: "No tagged tasks")
            }
        }
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
