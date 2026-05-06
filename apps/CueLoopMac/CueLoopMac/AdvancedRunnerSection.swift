/**
 AdvancedRunnerSection

 Purpose:
 - Provide Advanced Runner content column with command list and filters.

 Responsibilities:
 - Provide Advanced Runner content column with command list and filters.
 - Provide Advanced Runner detail column with argument configuration and command preview.
 - Support searching, filtering hidden commands/args, and building CLI arguments.

 Does not handle:
 - Direct CLI execution (delegated to Workspace).
 - Command palette functionality (see CommandPaletteView).

 Usage:
 - Used by the CueLoopMac app or CueLoopCore tests through its owning feature surface.

 Invariants/assumptions callers must respect:
 - Workspace is injected via @ObservedObject.
 - Commands are loaded via workspace.loadCLISpec().
 - Argument state is managed by Workspace.
 */

import SwiftUI
import CueLoopCore

@MainActor
struct AdvancedRunnerContentColumn: View {
    @ObservedObject var workspace: Workspace
    @ObservedObject private var commandState: WorkspaceCommandState
    let navTitle: (String) -> String

    init(workspace: Workspace, navTitle: @escaping (String) -> String) {
        self.workspace = workspace
        self._commandState = ObservedObject(wrappedValue: workspace.commandState)
        self.navTitle = navTitle
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            headerSection()
                .padding(.top, 16)
                .padding(.bottom, 16)
                .padding(.leading, 32)
                .padding(.trailing, 16)

            Divider()

            commandList()
        }
        .task { @MainActor in
            await loadCLISpecIfNeeded()
        }
        .onChange(of: workspace.identityState.retargetRevision) { _, _ in
            Task { @MainActor in
                await loadCLISpecIfNeeded()
            }
        }
        .onChange(of: commandState.advancedSelectedCommandID) { _, _ in
            workspace.resetAdvancedInputs()
        }
    }

    private func loadCLISpecIfNeeded() async {
        guard commandState.cliSpec == nil else { return }
        if commandState.cliSpecIsLoading {
            for _ in 0..<20 {
                guard !Task.isCancelled else { return }
                guard commandState.cliSpec == nil else { return }
                guard commandState.cliSpecIsLoading else { break }

                try? await Task.sleep(for: .milliseconds(100))
            }
        }
        guard commandState.cliSpec == nil else { return }
        guard !commandState.cliSpecIsLoading else { return }
        await workspace.loadCLISpec()
    }

    @ViewBuilder
    private func headerSection() -> some View {
        VStack(alignment: .leading, spacing: 12) {
            WorkingDirectoryHeader(workspace: workspace)

            advancedOptionsBar()

            if let err = commandState.cliSpecErrorMessage {
                Text(err)
                    .foregroundStyle(.red)
                    .font(.system(.caption))
                    .padding(.vertical, 4)
            }
        }
    }

    @ViewBuilder
    private func advancedOptionsBar() -> some View {
        ViewThatFits(in: .horizontal) {
            HStack(spacing: 12) {
                advancedOptionToggles()
                Spacer(minLength: 8)
                cliSpecLoadingIndicator()
                reloadButton()
            }

            VStack(alignment: .leading, spacing: 10) {
                HStack(spacing: 12) {
                    advancedOptionToggles()
                    Spacer(minLength: 0)
                }

                HStack(spacing: 8) {
                    cliSpecLoadingIndicator()
                    reloadButton()
                    Spacer(minLength: 0)
                }
            }
        }
    }

    @ViewBuilder
    private func advancedOptionToggles() -> some View {
        HStack(spacing: 12) {
            Toggle("No Color", isOn: Binding(
                get: { commandState.advancedIncludeNoColor },
                set: { commandState.advancedIncludeNoColor = $0 }
            ))
            .toggleStyle(.switch)
            .fixedSize(horizontal: true, vertical: false)

            Toggle("Show Hidden", isOn: Binding(
                get: { commandState.advancedShowHiddenCommands },
                set: { commandState.advancedShowHiddenCommands = $0 }
            ))
            .toggleStyle(.switch)
            .fixedSize(horizontal: true, vertical: false)

            Toggle("Hidden Args", isOn: Binding(
                get: { commandState.advancedShowHiddenArgs },
                set: { commandState.advancedShowHiddenArgs = $0 }
            ))
            .toggleStyle(.switch)
            .fixedSize(horizontal: true, vertical: false)
        }
    }

    @ViewBuilder
    private func cliSpecLoadingIndicator() -> some View {
        if commandState.cliSpecIsLoading {
            Image(systemName: "arrow.triangle.2.circlepath")
                .font(.caption)
                .foregroundStyle(.secondary)
                .symbolEffect(.rotate, isActive: true)
                .frame(width: 14, height: 14)
                .accessibilityLabel("Loading command specification")
        }
    }

    private func reloadButton() -> some View {
        Button(action: {
            Task { @MainActor in
                await workspace.loadCLISpec()
            }
        }) {
            Label("Reload", systemImage: "arrow.clockwise")
        }
        .buttonStyle(GlassButtonStyle())
        .fixedSize(horizontal: true, vertical: false)
    }

    @ViewBuilder
    private func commandList() -> some View {
        let commands = filteredCommands()

        List(
            commands,
            selection: Binding(
                get: { commandState.advancedSelectedCommandID },
                set: { commandState.advancedSelectedCommandID = $0 }
            )
        ) { cmd in
            VStack(alignment: .leading, spacing: 2) {
                Text(cmd.displayPath)
                    .font(.system(.body, design: .monospaced))
                if let about = cmd.about, !about.isEmpty {
                    Text(about)
                        .font(.system(.caption))
                        .foregroundStyle(.secondary)
                        .lineLimit(1)
                }
            }
            .tag(cmd.id)
            .listRowInsets(EdgeInsets(top: 4, leading: 32, bottom: 4, trailing: 8))
        }
        .listStyle(.plain)
        .searchable(
            text: Binding(
                get: { commandState.advancedSearchText },
                set: { commandState.advancedSearchText = $0 }
            ),
            placement: .toolbar
        )
        .navigationTitle(navTitle("Advanced Runner"))
    }

    private func filteredCommands() -> [CueLoopCLICommandSpec] {
        let commands = workspace.advancedCommands()
        let query = commandState.advancedSearchText.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !query.isEmpty else { return commands }

        return commands.filter { cmd in
            cmd.displayPath.localizedCaseInsensitiveContains(query)
                || (cmd.about?.localizedCaseInsensitiveContains(query) ?? false)
        }
    }
}

@MainActor
struct AdvancedRunnerDetailColumn: View {
    @ObservedObject var workspace: Workspace
    let navTitle: (String) -> String

    var body: some View {
        if let cmd = workspace.selectedAdvancedCommand() {
            commandDetailView(cmd: cmd)
        } else {
            EmptyAdvancedRunnerDetailView()
        }
    }

    @ViewBuilder
    private func commandDetailView(cmd: CueLoopCLICommandSpec) -> some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 20) {
                commandHeader(cmd: cmd)

                let args = cmd.args.filter { workspace.commandState.advancedShowHiddenArgs || !$0.hidden }
                let (positional, options) = splitArgs(args)

                if !positional.isEmpty {
                    positionalArgsSection(args: positional)
                }

                if !options.isEmpty {
                    optionsSection(args: options)
                }

                commandPreviewSection(cmd: cmd)

                ConsoleView(workspace: workspace)
            }
            .padding(20)
        }
        .background(.clear)
        .navigationTitle(navTitle(cmd.name))
    }

    @ViewBuilder
    private func commandHeader(cmd: CueLoopCLICommandSpec) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            Text(cmd.displayPath)
                .font(.system(.title3, design: .monospaced))
            if let about = cmd.about, !about.isEmpty {
                Text(about)
                    .foregroundStyle(.secondary)
            }
        }
    }

    @ViewBuilder
    private func positionalArgsSection(args: [CueLoopCLIArgSpec]) -> some View {
        GlassGroupBox(title: "Positionals") {
            VStack(alignment: .leading, spacing: 10) {
                ForEach(args, id: \.id) { arg in
                    AdvancedArgRow(workspace: workspace, arg: arg)
                }
            }
        }
    }

    @ViewBuilder
    private func optionsSection(args: [CueLoopCLIArgSpec]) -> some View {
        GlassGroupBox(title: "Options") {
            VStack(alignment: .leading, spacing: 10) {
                ForEach(args, id: \.id) { arg in
                    AdvancedArgRow(workspace: workspace, arg: arg)
                }
            }
        }
    }

    @ViewBuilder
    private func commandPreviewSection(cmd: CueLoopCLICommandSpec) -> some View {
        GlassGroupBox(title: "Command") {
            VStack(alignment: .leading, spacing: 8) {
                let argv = workspace.buildAdvancedArguments()
                Text(shellPreview(argv: argv))
                    .font(.system(.caption, design: .monospaced))
                    .foregroundStyle(.secondary)
                    .textSelection(.enabled)
                    .frame(maxWidth: .infinity, alignment: .leading)

                HStack {
                    Button("Run") {
                        let argv = workspace.buildAdvancedArguments()
                        if !argv.isEmpty {
                            workspace.run(arguments: argv)
                        }
                    }
                    .disabled(workspace.runState.isExecutionActive)
                    .buttonStyle(GlassButtonStyle())

                    if workspace.runState.isExecutionActive {
                        Button(action: { workspace.cancel() }) {
                            Label("Stop", systemImage: "stop.circle.fill")
                                .foregroundStyle(.red)
                        }
                        .buttonStyle(.borderless)
                    }

                    Spacer()

                    ExitStatusBadge(workspace: workspace)
                }
            }
        }
    }

    private func splitArgs(_ args: [CueLoopCLIArgSpec]) -> ([CueLoopCLIArgSpec], [CueLoopCLIArgSpec]) {
        let positionals = args
            .filter(\.positional)
            .sorted { ($0.index ?? Int.max) < ($1.index ?? Int.max) }
        let options = args
            .filter { !$0.positional }
            .sorted { $0.id < $1.id }
        return (positionals, options)
    }

    private func shellPreview(argv: [String]) -> String {
        guard !argv.isEmpty else { return "" }
        return (["cueloop"] + argv).map(shellEscape).joined(separator: " ")
    }

    private func shellEscape(_ s: String) -> String {
        let allowed = CharacterSet.alphanumerics
            .union(CharacterSet(charactersIn: "._/-=:"))
        if s.unicodeScalars.allSatisfy({ allowed.contains($0) }) {
            return s
        }
        return "'" + s.replacingOccurrences(of: "'", with: "'\"'\"'") + "'"
    }
}

@MainActor
struct AdvancedArgRow: View {
    @ObservedObject var workspace: Workspace
    let arg: CueLoopCLIArgSpec

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            HStack(alignment: .firstTextBaseline) {
                Text(argDisplayName)
                    .font(.system(.body, design: .monospaced))
                    .foregroundStyle(arg.required ? Color.primary : Color.secondary)

                if arg.required {
                    Text("*")
                        .foregroundStyle(.red)
                }

                Spacer()

                argInputControl
            }

            if let help = arg.help, !help.isEmpty {
                Text(help)
                    .font(.system(.caption))
                    .foregroundStyle(.secondary)
            }
        }
    }

    @ViewBuilder
    private var argInputControl: some View {
        if arg.isCountFlag {
            Stepper(
                value: Binding(
                    get: { workspace.commandState.advancedCountValues[arg.id] ?? 0 },
                    set: { workspace.commandState.advancedCountValues[arg.id] = $0 }
                ),
                in: 0...20
            ) {
                Text("\(workspace.commandState.advancedCountValues[arg.id] ?? 0)")
                    .font(.system(.caption, design: .monospaced))
                    .foregroundStyle(.secondary)
            }
            .frame(maxWidth: 220)
        } else if arg.isBooleanFlag {
            Toggle(
                "",
                isOn: Binding(
                    get: { workspace.commandState.advancedBoolValues[arg.id] ?? false },
                    set: { workspace.commandState.advancedBoolValues[arg.id] = $0 }
                )
            )
            .labelsHidden()
            .toggleStyle(.switch)
        } else if arg.takesValue {
            if arg.allowsMultipleValues {
                TextEditor(
                    text: Binding(
                        get: { workspace.commandState.advancedMultiValues[arg.id] ?? "" },
                        set: { workspace.commandState.advancedMultiValues[arg.id] = $0 }
                    )
                )
                .font(.system(.caption, design: .monospaced))
                .frame(minHeight: 48, maxHeight: 88)
            } else {
                TextField(
                    "",
                    text: Binding(
                        get: { workspace.commandState.advancedSingleValues[arg.id] ?? "" },
                        set: { workspace.commandState.advancedSingleValues[arg.id] = $0 }
                    )
                )
                .textFieldStyle(.roundedBorder)
                .font(.system(.body, design: .monospaced))
                .frame(maxWidth: 360)
            }
        }
    }

    private var argDisplayName: String {
        if arg.positional {
            let idx = arg.index.map { "#\($0)" } ?? ""
            return "<\(arg.id)>\(idx.isEmpty ? "" : " \(idx)")"
        }

        var parts: [String] = []
        if let long = arg.long {
            parts.append("--\(long)")
        }
        if let short = arg.short, !short.isEmpty {
            parts.append("-\(short)")
        }
        if parts.isEmpty {
            return arg.id
        }
        return parts.joined(separator: " ")
    }
}

@MainActor
struct EmptyAdvancedRunnerDetailView: View {
    var body: some View {
        VStack(spacing: 16) {
            Image(systemName: "terminal.fill")
                .font(.system(size: 48))
                .foregroundStyle(.secondary)

            Text("No Command Selected")
                .font(.headline)

            Text("Select a command from the list to configure and run it.")
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
struct ExitStatusBadge: View {
    @ObservedObject var workspace: Workspace

    var body: some View {
        if let status = workspace.runState.lastExitStatus {
            Text("Exit: \(status.code) [\(status.reason.rawValue)]")
                .font(.system(.caption, design: .monospaced))
                .foregroundStyle(status.code == 0 ? Color.secondary : Color.red)
        }
    }
}

@MainActor
struct GlassGroupBox<Content: View>: View {
    let title: String
    @ViewBuilder let content: () -> Content

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(title)
                .font(.system(.caption, weight: .semibold))
                .foregroundStyle(.secondary)
                .padding(.horizontal, 12)

            content()
                .padding(12)
                .frame(maxWidth: .infinity, alignment: .leading)
                .underPageBackground(cornerRadius: 10, isEmphasized: false)
        }
    }
}

private extension CueLoopCLICommandSpec {
    var displayPath: String {
        let segments = Array(path.dropFirst())
        if segments.isEmpty {
            return name
        }
        return segments.joined(separator: " ")
    }
}
