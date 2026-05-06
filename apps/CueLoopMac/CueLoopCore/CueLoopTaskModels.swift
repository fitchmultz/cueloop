/**
 CueLoopTaskModels

 Purpose:
 - Define task, agent-override, queue-document, and queue machine-document models shared across the app.

 Responsibilities:
 - Define task, agent-override, queue-document, and queue machine-document models shared across the app.
 - Normalize task-level execution overrides into canonical forms.

 Does not handle:
 - Graph visualization or analytics aggregation.
 - Workspace mutations or persistence side effects.

 Usage:
 - Used by the CueLoopMac app or CueLoopCore tests through its owning feature surface.

 Invariants/assumptions callers must respect:
 - Queue payloads decode from the current queue document object shape.
 - Task-agent normalization is the single cutover surface for override cleanup.
 */

public import Foundation

public enum CueLoopTaskKind: String, Codable, Sendable, Equatable, CaseIterable {
    case workItem = "work_item"
    case group = "group"

    public var displayName: String {
        switch self {
        case .workItem: return "Work item"
        case .group: return "Group"
        }
    }

    public var isExecutable: Bool {
        self == .workItem
    }
}

public enum CueLoopTaskStatus: String, Codable, Sendable, Equatable, CaseIterable {
    case draft = "draft"
    case todo = "todo"
    case doing = "doing"
    case done = "done"
    case rejected = "rejected"

    public var displayName: String {
        switch self {
        case .draft: return "Draft"
        case .todo: return "Todo"
        case .doing: return "Doing"
        case .done: return "Done"
        case .rejected: return "Rejected"
        }
    }
}

public enum CueLoopTaskPriority: String, Codable, Sendable, Equatable, CaseIterable {
    case critical = "critical"
    case high = "high"
    case medium = "medium"
    case low = "low"

    public var displayName: String {
        switch self {
        case .critical: return "Critical"
        case .high: return "High"
        case .medium: return "Medium"
        case .low: return "Low"
        }
    }

    public var sortOrder: Int {
        switch self {
        case .critical: return 4
        case .high: return 3
        case .medium: return 2
        case .low: return 1
        }
    }
}

public struct CueLoopTaskPhaseOverride: Codable, Sendable, Equatable {
    public var runner: String?
    public var model: String?
    public var reasoningEffort: String?

    private enum CodingKeys: String, CodingKey {
        case runner
        case model
        case reasoningEffort = "reasoning_effort"
    }

    public init(
        runner: String? = nil,
        model: String? = nil,
        reasoningEffort: String? = nil
    ) {
        self.runner = runner
        self.model = model
        self.reasoningEffort = reasoningEffort
    }

    public var isEmpty: Bool {
        (runner?.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty ?? true)
            && (model?.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty ?? true)
            && (reasoningEffort?.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty ?? true)
    }
}

public struct CueLoopTaskPhaseOverrides: Codable, Sendable, Equatable {
    public var phase1: CueLoopTaskPhaseOverride?
    public var phase2: CueLoopTaskPhaseOverride?
    public var phase3: CueLoopTaskPhaseOverride?

    public init(
        phase1: CueLoopTaskPhaseOverride? = nil,
        phase2: CueLoopTaskPhaseOverride? = nil,
        phase3: CueLoopTaskPhaseOverride? = nil
    ) {
        self.phase1 = phase1
        self.phase2 = phase2
        self.phase3 = phase3
    }

    public var isEmpty: Bool {
        (phase1?.isEmpty ?? true) && (phase2?.isEmpty ?? true) && (phase3?.isEmpty ?? true)
    }
}

public struct CueLoopTaskAgent: Codable, Sendable, Equatable {
    public var runner: String?
    public var model: String?
    public var modelEffort: String?
    public var phases: Int?
    public var iterations: Int?
    public var followupReasoningEffort: String?
    public var runnerCLI: CueLoopJSONValue?
    public var phaseOverrides: CueLoopTaskPhaseOverrides?

    private enum CodingKeys: String, CodingKey {
        case runner
        case model
        case modelEffort = "model_effort"
        case phases
        case iterations
        case followupReasoningEffort = "followup_reasoning_effort"
        case runnerCLI = "runner_cli"
        case phaseOverrides = "phase_overrides"
    }

    public init(
        runner: String? = nil,
        model: String? = nil,
        modelEffort: String? = nil,
        phases: Int? = nil,
        iterations: Int? = nil,
        followupReasoningEffort: String? = nil,
        runnerCLI: CueLoopJSONValue? = nil,
        phaseOverrides: CueLoopTaskPhaseOverrides? = nil
    ) {
        self.runner = runner
        self.model = model
        self.modelEffort = modelEffort
        self.phases = phases
        self.iterations = iterations
        self.followupReasoningEffort = followupReasoningEffort
        self.runnerCLI = runnerCLI
        self.phaseOverrides = phaseOverrides
    }

    public var isEmpty: Bool {
        let runnerEmpty = runner?.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty ?? true
        let modelEmpty = model?.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty ?? true
        let modelEffortEmpty = modelEffort?.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty ?? true
        let followupEmpty = followupReasoningEffort?.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty ?? true
        return runnerEmpty
            && modelEmpty
            && modelEffortEmpty
            && phases == nil
            && iterations == nil
            && followupEmpty
            && runnerCLI == nil
            && (phaseOverrides?.isEmpty ?? true)
    }
}

public extension CueLoopTaskAgent {
    static func normalizedOverride(_ agent: CueLoopTaskAgent?) -> CueLoopTaskAgent? {
        guard var normalized = agent else { return nil }

        normalized.runner = normalizeOptionalString(normalized.runner)
        normalized.model = normalizeOptionalString(normalized.model)
        normalized.modelEffort = normalizeOptionalString(normalized.modelEffort)
        if normalized.modelEffort?.lowercased() == "default" {
            normalized.modelEffort = nil
        }
        normalized.followupReasoningEffort = normalizeOptionalString(normalized.followupReasoningEffort)

        if let phases = normalized.phases, !(1...3).contains(phases) {
            normalized.phases = nil
        }
        if let iterations = normalized.iterations, iterations < 1 {
            normalized.iterations = nil
        }

        if var phaseOverrides = normalized.phaseOverrides {
            phaseOverrides.phase1 = normalizePhaseOverride(phaseOverrides.phase1)
            phaseOverrides.phase2 = normalizePhaseOverride(phaseOverrides.phase2)
            phaseOverrides.phase3 = normalizePhaseOverride(phaseOverrides.phase3)
            normalized.phaseOverrides = phaseOverrides.isEmpty ? nil : phaseOverrides
        }

        return normalized.isEmpty ? nil : normalized
    }
}

public enum CueLoopTaskExecutionPreset: String, CaseIterable, Sendable, Identifiable {
    case codexDeep
    case codexBalanced
    case kimiFast
    case hybridCodexKimi
    case inheritFromConfig

    public var id: String { rawValue }

    public var displayName: String {
        switch self {
        case .codexDeep:
            return "Deep"
        case .codexBalanced:
            return "Balanced"
        case .kimiFast:
            return "Fast"
        case .hybridCodexKimi:
            return "Phased"
        case .inheritFromConfig:
            return "Inherit Config"
        }
    }

    public var description: String {
        switch self {
        case .codexDeep:
            return "High reasoning with the configured runner/model."
        case .codexBalanced:
            return "Medium reasoning and a 2-phase flow using config."
        case .kimiFast:
            return "Low reasoning and a 1-phase flow using config."
        case .hybridCodexKimi:
            return "Phase-specific effort tuning using the configured runner/model."
        case .inheritFromConfig:
            return "Remove task overrides and use .cueloop/config.jsonc."
        }
    }

    public var agentOverride: CueLoopTaskAgent? {
        switch self {
        case .codexDeep:
            return CueLoopTaskAgent(
                modelEffort: "high",
                phases: 3,
                iterations: 1
            )
        case .codexBalanced:
            return CueLoopTaskAgent(
                modelEffort: "medium",
                phases: 2,
                iterations: 1
            )
        case .kimiFast:
            return CueLoopTaskAgent(
                modelEffort: "low",
                phases: 1,
                iterations: 1
            )
        case .hybridCodexKimi:
            return CueLoopTaskAgent(
                phases: 3,
                iterations: 1,
                phaseOverrides: CueLoopTaskPhaseOverrides(
                    phase1: CueLoopTaskPhaseOverride(reasoningEffort: "high"),
                    phase2: CueLoopTaskPhaseOverride(reasoningEffort: "medium"),
                    phase3: CueLoopTaskPhaseOverride(reasoningEffort: "medium")
                )
            )
        case .inheritFromConfig:
            return nil
        }
    }

    public static func matchingPreset(for agent: CueLoopTaskAgent?) -> CueLoopTaskExecutionPreset? {
        let normalizedAgent = CueLoopTaskAgent.normalizedOverride(agent)
        for preset in Self.allCases where preset != .inheritFromConfig {
            if CueLoopTaskAgent.normalizedOverride(preset.agentOverride) == normalizedAgent {
                return preset
            }
        }
        if normalizedAgent == nil {
            return .inheritFromConfig
        }
        return nil
    }
}

public struct CueLoopTask: Codable, Sendable, Equatable, Identifiable {
    public let id: String
    public var status: CueLoopTaskStatus
    public var kind: CueLoopTaskKind
    public var title: String
    public var description: String?
    public var priority: CueLoopTaskPriority
    public var tags: [String]
    public var scope: [String]?
    public var evidence: [String]?
    public var plan: [String]?
    public var notes: [String]?
    public var request: String?
    public var agent: CueLoopTaskAgent?
    public var createdAt: Date?
    public var updatedAt: Date?
    public var startedAt: Date?
    public var completedAt: Date?
    public var estimatedMinutes: Int?
    public var actualMinutes: Int?
    public var scheduledStart: Date?
    public var dependsOn: [String]?
    public var blocks: [String]?
    public var relatesTo: [String]?
    public var duplicates: String?
    public var customFields: [String: String]?
    public var parentID: String?

    private enum CodingKeys: String, CodingKey {
        case id, status, kind, title, description, priority, tags, scope, evidence, plan, notes
        case request, agent, dependsOn = "depends_on", blocks, relatesTo = "relates_to"
        case createdAt = "created_at"
        case updatedAt = "updated_at"
        case startedAt = "started_at"
        case completedAt = "completed_at"
        case estimatedMinutes = "estimated_minutes"
        case actualMinutes = "actual_minutes"
        case scheduledStart = "scheduled_start"
        case duplicates
        case customFields = "custom_fields"
        case parentID = "parent_id"
    }

    public init(
        id: String,
        status: CueLoopTaskStatus,
        kind: CueLoopTaskKind = .workItem,
        title: String,
        description: String? = nil,
        priority: CueLoopTaskPriority,
        tags: [String] = [],
        scope: [String]? = nil,
        evidence: [String]? = nil,
        plan: [String]? = nil,
        notes: [String]? = nil,
        request: String? = nil,
        agent: CueLoopTaskAgent? = nil,
        createdAt: Date? = nil,
        updatedAt: Date? = nil,
        startedAt: Date? = nil,
        completedAt: Date? = nil,
        estimatedMinutes: Int? = nil,
        actualMinutes: Int? = nil,
        scheduledStart: Date? = nil,
        dependsOn: [String]? = nil,
        blocks: [String]? = nil,
        relatesTo: [String]? = nil,
        duplicates: String? = nil,
        customFields: [String: String]? = nil,
        parentID: String? = nil
    ) {
        self.id = id
        self.status = status
        self.kind = kind
        self.title = title
        self.description = description
        self.priority = priority
        self.tags = tags
        self.scope = scope
        self.evidence = evidence
        self.plan = plan
        self.notes = notes
        self.request = request
        self.agent = agent
        self.createdAt = createdAt
        self.updatedAt = updatedAt
        self.startedAt = startedAt
        self.completedAt = completedAt
        self.estimatedMinutes = estimatedMinutes
        self.actualMinutes = actualMinutes
        self.scheduledStart = scheduledStart
        self.dependsOn = dependsOn
        self.blocks = blocks
        self.relatesTo = relatesTo
        self.duplicates = duplicates
        self.customFields = customFields
        self.parentID = parentID
    }

    public init(from decoder: any Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        self.id = try container.decode(String.self, forKey: .id)
        self.status = try container.decode(CueLoopTaskStatus.self, forKey: .status)
        self.kind = try container.decodeIfPresent(CueLoopTaskKind.self, forKey: .kind) ?? .workItem
        self.title = try container.decode(String.self, forKey: .title)
        self.description = try container.decodeIfPresent(String.self, forKey: .description)
        self.priority = try container.decode(CueLoopTaskPriority.self, forKey: .priority)
        self.tags = try container.decodeIfPresent([String].self, forKey: .tags) ?? []
        self.scope = try container.decodeIfPresent([String].self, forKey: .scope)
        self.evidence = try container.decodeIfPresent([String].self, forKey: .evidence)
        self.plan = try container.decodeIfPresent([String].self, forKey: .plan)
        self.notes = try container.decodeIfPresent([String].self, forKey: .notes)
        self.request = try container.decodeIfPresent(String.self, forKey: .request)
        self.agent = try container.decodeIfPresent(CueLoopTaskAgent.self, forKey: .agent)
        self.createdAt = try container.decodeIfPresent(Date.self, forKey: .createdAt)
        self.updatedAt = try container.decodeIfPresent(Date.self, forKey: .updatedAt)
        self.startedAt = try container.decodeIfPresent(Date.self, forKey: .startedAt)
        self.completedAt = try container.decodeIfPresent(Date.self, forKey: .completedAt)
        self.estimatedMinutes = try container.decodeIfPresent(Int.self, forKey: .estimatedMinutes)
        self.actualMinutes = try container.decodeIfPresent(Int.self, forKey: .actualMinutes)
        self.scheduledStart = try container.decodeIfPresent(Date.self, forKey: .scheduledStart)
        self.dependsOn = try container.decodeIfPresent([String].self, forKey: .dependsOn)
        self.blocks = try container.decodeIfPresent([String].self, forKey: .blocks)
        self.relatesTo = try container.decodeIfPresent([String].self, forKey: .relatesTo)
        self.duplicates = try container.decodeIfPresent(String.self, forKey: .duplicates)
        self.customFields = try container.decodeIfPresent([String: String].self, forKey: .customFields)
        self.parentID = try container.decodeIfPresent(String.self, forKey: .parentID)
    }
}

/// Represents the top-level on-disk queue document under `.cueloop/queue.jsonc`.
public struct CueLoopTaskQueueDocument: Codable, Sendable, Equatable {
    public let version: Int
    public let tasks: [CueLoopTask]

    private enum CodingKeys: String, CodingKey {
        case version
        case tasks
    }

    public init(version: Int = 1, tasks: [CueLoopTask]) {
        self.version = version
        self.tasks = tasks
    }

    public init(from decoder: any Decoder) throws {
        if let keyed = try? decoder.container(keyedBy: CodingKeys.self),
           keyed.contains(.tasks) {
            self.version = try keyed.decodeIfPresent(Int.self, forKey: .version) ?? 1
            self.tasks = try keyed.decode([CueLoopTask].self, forKey: .tasks)
            return
        }

        throw DecodingError.typeMismatch(
            CueLoopTaskQueueDocument.self,
            DecodingError.Context(
                codingPath: decoder.codingPath,
                debugDescription: "Expected queue document object with tasks key"
            )
        )
    }
}

public struct MachineQueueReadDocument: Codable, Sendable, Equatable, VersionedMachineDocument {
    public static let expectedVersion = CueLoopMachineContract.queueReadVersion
    public static let documentName = "machine queue read"

    public let version: Int
    public let paths: MachineQueuePaths
    public let active: CueLoopTaskQueueDocument
    public let done: CueLoopTaskQueueDocument
    public let nextRunnableTaskID: String?
    public let runnability: CueLoopJSONValue

    private enum CodingKeys: String, CodingKey {
        case version
        case paths
        case active
        case done
        case nextRunnableTaskID = "next_runnable_task_id"
        case runnability
    }
}

private func normalizeOptionalString(_ value: String?) -> String? {
    guard let value else { return nil }
    let trimmed = value.trimmingCharacters(in: .whitespacesAndNewlines)
    return trimmed.isEmpty ? nil : trimmed
}

private func normalizePhaseOverride(_ overrideValue: CueLoopTaskPhaseOverride?) -> CueLoopTaskPhaseOverride? {
    guard var normalized = overrideValue else { return nil }
    normalized.runner = normalizeOptionalString(normalized.runner)
    normalized.model = normalizeOptionalString(normalized.model)
    normalized.reasoningEffort = normalizeOptionalString(normalized.reasoningEffort)
    return normalized.isEmpty ? nil : normalized
}
