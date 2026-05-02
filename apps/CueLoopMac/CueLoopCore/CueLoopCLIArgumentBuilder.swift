/**
 CueLoopCLIArgumentBuilder

 Purpose:
 - Translate typed CLI selections into argv arrays for subprocess execution.

 Responsibilities:
 - Translate typed CLI selections into argv arrays for subprocess execution.
 - Keep positional and option token rendering consistent with the emitted CLI spec.

 Does not handle:
 - Interactive prompting or validation of clap semantics.
 - Command execution.

 Usage:
 - Used by the CueLoopMac app or CueLoopCore tests through its owning feature surface.

 Invariants/assumptions callers must respect:
 - `command.path` includes the root executable name as its first segment.
 - Selection dictionaries are keyed by `CueLoopCLIArgSpec.id`.
 */

import Foundation

public enum CueLoopCLIArgValue: Equatable, Sendable, Hashable {
    case flag(Bool)
    case count(Int)
    case values([String])
}

public enum CueLoopCLIArgumentBuilder {
    public static func buildArguments(
        command: CueLoopCLICommandSpec,
        selections: [String: CueLoopCLIArgValue],
        globalArguments: [String] = []
    ) -> [String] {
        var argv: [String] = []
        argv.append(contentsOf: globalArguments)
        argv.append(contentsOf: command.path.dropFirst())

        let (positionals, options): ([CueLoopCLIArgSpec], [CueLoopCLIArgSpec]) = command.args.reduce(into: ([], [])) { acc, arg in
            if arg.positional {
                acc.0.append(arg)
            } else {
                acc.1.append(arg)
            }
        }

        for arg in options {
            guard let value = selections[arg.id] else { continue }
            argv.append(contentsOf: buildOptionTokens(arg: arg, value: value))
        }

        let sortedPositionals = positionals.sorted { (a, b) in
            (a.index ?? Int.max) < (b.index ?? Int.max)
        }
        for arg in sortedPositionals {
            guard let value = selections[arg.id] else { continue }
            argv.append(contentsOf: buildPositionalTokens(arg: arg, value: value))
        }

        return argv
    }

    private static func buildPositionalTokens(arg: CueLoopCLIArgSpec, value: CueLoopCLIArgValue) -> [String] {
        guard arg.positional else { return [] }
        switch value {
        case .values(let values):
            return values
        case .flag, .count:
            return []
        }
    }

    private static func buildOptionTokens(arg: CueLoopCLIArgSpec, value: CueLoopCLIArgValue) -> [String] {
        guard !arg.positional else { return [] }
        guard let token = arg.preferredToken else {
            return []
        }

        switch value {
        case .flag(let present):
            return present ? [token] : []
        case .count(let n):
            guard n > 0 else { return [] }
            return Array(repeating: token, count: n)
        case .values(let values):
            let normalized = values.filter { !$0.isEmpty }
            guard !normalized.isEmpty else { return [] }

            if arg.numArgsMax == nil || (arg.numArgsMax ?? 0) > 1 {
                return [token] + normalized
            }

            if arg.action.contains("Append") {
                var out: [String] = []
                out.reserveCapacity(normalized.count * 2)
                for value in normalized {
                    out.append(token)
                    out.append(value)
                }
                return out
            }

            return [token, normalized[0]]
        }
    }
}
