// Purpose:
// - Bridge CueLoop's Cursor runner integration to the local @cursor/sdk package.
//
// Responsibilities:
// - Read one JSON runner request from stdin and invoke Cursor SDK run/resume APIs.
// - Normalize Cursor SDK stream events into CueLoop's newline-delimited JSON protocol.
// - Resolve @cursor/sdk from an explicit module path, the workspace, or global npm roots.
//
// Non-scope:
// - Selecting CueLoop tasks, resolving runner configuration, or managing subprocess lifetimes.
// - Persisting queue/done state or interpreting CueLoop phase semantics beyond request flags.
//
// Usage:
// - Embedded by CueLoop and executed as `node <helper>.mjs` with a serialized request on stdin.
//
// Invariants/Assumptions:
// - The configured Node runtime is supported by @cursor/sdk.
// - Caller-provided request fields are trusted only after local validation.

import fs from "node:fs";
import { createRequire } from "node:module";
import path from "node:path";
import process from "node:process";
import { pathToFileURL } from "node:url";

const DEFAULT_SETTING_SOURCES = ["project", "user", "plugins"];
const EXPECTED_CURSOR_SDK_VERSION = "1.0.12";

function emit(event) {
  process.stdout.write(`${JSON.stringify(event)}\n`);
}

function readStdin() {
  return new Promise((resolve, reject) => {
    let data = "";
    process.stdin.setEncoding("utf8");
    process.stdin.on("data", (chunk) => {
      data += chunk;
    });
    process.stdin.on("end", () => resolve(data));
    process.stdin.on("error", reject);
  });
}

async function readRequest() {
  const raw = await readStdin();
  if (raw.trim().length === 0) {
    throw new Error("missing Cursor SDK runner request on stdin");
  }
  return JSON.parse(raw);
}

function newCursorSdkDiagnostics() {
  return {
    preferred_sdk_version: EXPECTED_CURSOR_SDK_VERSION,
    selected: null,
    attempted_sources: [],
    warnings: [],
    proceeded_best_effort: false,
    fatal_cause: null,
  };
}

function addAttempt(diagnostics, attempt) {
  const normalized = {
    source: attempt.source,
    location: attempt.location ?? null,
    entrypoint: attempt.entrypoint ?? null,
    package_json: attempt.package_json ?? null,
    sdk_version: attempt.sdk_version ?? null,
    global_root: attempt.global_root ?? null,
    status: attempt.status,
    error: attempt.error ? String(attempt.error) : null,
  };
  diagnostics.attempted_sources.push(normalized);
  return normalized;
}

async function loadCursorSdk(cwd) {
  const diagnostics = newCursorSdkDiagnostics();
  const configuredPath = process.env.CUELOOP_CURSOR_SDK_MODULE_PATH;
  if (configuredPath) {
    const entrypoint = path.resolve(configuredPath);
    const attempt = addAttempt(diagnostics, {
      source: "env",
      location: configuredPath,
      entrypoint,
      status: fs.existsSync(entrypoint) ? "resolved" : "invalid_path",
      error: fs.existsSync(entrypoint) ? null : "path does not exist",
    });
    try {
      return await importCursorSdkFromPath(entrypoint, "env", diagnostics, attempt);
    } catch (configuredError) {
      attempt.status = attempt.status === "invalid_path" ? "invalid_path" : statusForImportError(configuredError);
      attempt.error = String(configuredError?.message ?? configuredError);
      throw sdkLoadError(configuredError, diagnostics, "invalid_module_path", `CUELOOP_CURSOR_SDK_MODULE_PATH is set but unusable: ${entrypoint}`);
    }
  } else {
    addAttempt(diagnostics, {
      source: "env",
      location: "CUELOOP_CURSOR_SDK_MODULE_PATH (unset)",
      status: "not_found",
      error: "environment variable is not set",
    });
  }

  const requireFromCwd = createRequire(path.join(cwd, "package.json"));
  let workspaceResolved = null;
  const workspaceAttempt = addAttempt(diagnostics, {
    source: "workspace",
    location: cwd,
    status: "not_found",
  });
  try {
    workspaceResolved = requireFromCwd.resolve("@cursor/sdk", { paths: [cwd] });
    workspaceAttempt.entrypoint = path.resolve(workspaceResolved);
    workspaceAttempt.status = "resolved";
  } catch (cwdResolveError) {
    workspaceAttempt.error = String(cwdResolveError?.message ?? cwdResolveError);
  }
  if (workspaceResolved) {
    try {
      return await importCursorSdkFromPath(workspaceResolved, "workspace", diagnostics, workspaceAttempt);
    } catch (cwdImportError) {
      workspaceAttempt.status = statusForImportError(cwdImportError);
      workspaceAttempt.error = String(cwdImportError?.message ?? cwdImportError);
      throw sdkLoadError(cwdImportError, diagnostics, statusForFatal(cwdImportError), "Workspace @cursor/sdk is unusable");
    }
  }

  const roots = await discoverGlobalNpmRoots();
  if (roots.length === 0) {
    addAttempt(diagnostics, {
      source: "global",
      location: "global npm roots",
      status: "not_found",
      error: "no global npm root discovered",
    });
  }
  for (const globalRoot of roots) {
    let globalResolved = null;
    const globalAttempt = addAttempt(diagnostics, {
      source: "global",
      location: globalRoot,
      global_root: globalRoot,
      status: "not_found",
    });
    try {
      globalResolved = resolveCursorSdkFromGlobalRoot(globalRoot);
      globalAttempt.entrypoint = path.resolve(globalResolved);
      globalAttempt.status = "resolved";
    } catch (globalResolveError) {
      globalAttempt.error = String(globalResolveError?.message ?? globalResolveError);
      continue;
    }
    try {
      return await importCursorSdkFromPath(globalResolved, "global", diagnostics, globalAttempt, { globalRoot });
    } catch (globalImportError) {
      globalAttempt.status = statusForImportError(globalImportError);
      globalAttempt.error = String(globalImportError?.message ?? globalImportError);
      throw sdkLoadError(globalImportError, diagnostics, statusForFatal(globalImportError), `Global @cursor/sdk from ${globalRoot} is unusable`);
    }
  }

  throw sdkLoadError(null, diagnostics, "missing_sdk", "Unable to load @cursor/sdk: package was not found");
}

async function discoverGlobalNpmRoots() {
  const roots = [];
  if (process.env.CUELOOP_CURSOR_SDK_GLOBAL_ROOT) {
    return [path.resolve(process.env.CUELOOP_CURSOR_SDK_GLOBAL_ROOT)];
  }
  try {
    const { execFileSync } = await import("node:child_process");
    const root = execFileSync("npm", ["root", "-g"], { encoding: "utf8", stdio: ["ignore", "pipe", "ignore"] }).trim();
    if (root) roots.push(root);
  } catch {}
  return [...new Set(roots.map((root) => path.resolve(root)))];
}

function resolveCursorSdkFromGlobalRoot(globalRoot) {
  const packageJson = path.join(globalRoot, "@cursor", "sdk", "package.json");
  if (fs.existsSync(packageJson)) {
    return createRequire(packageJson).resolve("./");
  }
  const requireFromGlobalRoot = createRequire(path.join(globalRoot, "package.json"));
  return requireFromGlobalRoot.resolve("@cursor/sdk", { paths: [globalRoot] });
}

async function importCursorSdkFromPath(entrypoint, source, diagnostics, attempt, extra = {}) {
  const resolvedEntrypoint = path.resolve(entrypoint);
  const metadata = cursorSdkMetadata(resolvedEntrypoint);
  if (attempt) {
    attempt.entrypoint = resolvedEntrypoint;
    attempt.package_json = metadata.packageJsonPath;
    attempt.sdk_version = metadata.version;
  }
  const selected = {
    source,
    entrypoint: resolvedEntrypoint,
    package_json: metadata.packageJsonPath,
    sdk_version: metadata.version,
    global_root: extra.globalRoot ?? null,
  };
  diagnostics.selected = selected;
  const sdk = normalizeCursorSdkModule(await import(pathToFileURL(resolvedEntrypoint).href));
  if (attempt) {
    attempt.status = "selected";
  }
  const warning = cursorSdkVersionWarning(metadata);
  if (warning) {
    diagnostics.warnings.push(warning);
    diagnostics.proceeded_best_effort = true;
    emit({
      type: "system",
      subtype: "cursor_sdk_warning",
      warning,
      sdk_version: metadata.version,
      preferred_sdk_version: EXPECTED_CURSOR_SDK_VERSION,
      sdk_entrypoint: resolvedEntrypoint,
      sdk_source: source,
      proceeded_best_effort: true,
      attempted_sources: diagnostics.attempted_sources,
      diagnostics,
    });
  }
  return sdk;
}

function cursorSdkMetadata(entrypoint) {
  const packageJsonPath = findCursorSdkPackageJson(entrypoint);
  if (!packageJsonPath) {
    return { packageJsonPath: null, version: null };
  }
  const pkg = JSON.parse(fs.readFileSync(packageJsonPath, "utf8"));
  return { packageJsonPath, version: pkg.version ?? null };
}

function cursorSdkVersionWarning(metadata) {
  if (!metadata.version) {
    return `Unable to find @cursor/sdk package version; CueLoop's preferred/tested version is ${EXPECTED_CURSOR_SDK_VERSION}; proceeding best-effort`;
  }
  if (metadata.version !== EXPECTED_CURSOR_SDK_VERSION) {
    return `@cursor/sdk ${metadata.version} differs from CueLoop's preferred/tested ${EXPECTED_CURSOR_SDK_VERSION}; proceeding best-effort`;
  }
  return null;
}

function findCursorSdkPackageJson(entrypoint) {
  let current = path.resolve(entrypoint);
  if (!fs.existsSync(current)) {
    return null;
  }
  if (!fs.statSync(current).isDirectory()) {
    current = path.dirname(current);
  }

  while (true) {
    const candidate = path.join(current, "package.json");
    if (fs.existsSync(candidate)) {
      try {
        const pkg = JSON.parse(fs.readFileSync(candidate, "utf8"));
        if (pkg.name === "@cursor/sdk") {
          return candidate;
        }
      } catch {
        return null;
      }
    }
    const parent = path.dirname(current);
    if (parent === current) {
      return null;
    }
    current = parent;
  }
}

function normalizeCursorSdkModule(moduleNamespace) {
  const candidates = [
    moduleNamespace,
    moduleNamespace?.default,
    moduleNamespace?.default?.default,
  ];
  const sdk = candidates.find((candidate) => candidate?.Agent);
  if (!sdk) {
    const error = new Error("Loaded @cursor/sdk module does not expose required export Agent");
    error.name = "CursorSdkLoadError";
    error.fatalCause = "incompatible_api";
    throw error;
  }
  return sdk;
}

function statusForImportError(error) {
  if (error?.fatalCause === "incompatible_api" || String(error?.message ?? error).includes("does not expose")) {
    return "incompatible_api";
  }
  return "import_failed";
}

function statusForFatal(error) {
  return statusForImportError(error) === "incompatible_api" ? "incompatible_api" : "import_failed";
}

function attemptedSourcesSummary(diagnostics) {
  return diagnostics.attempted_sources
    .map((attempt) => `${attempt.source} ${attempt.location ?? "unknown"}${attempt.entrypoint ? ` -> ${attempt.entrypoint}` : ""} [${attempt.status}]`)
    .join("; ");
}

function sdkLoadError(primaryError, diagnostics, fatalCause, prefix = "Unable to load @cursor/sdk") {
  diagnostics.fatal_cause = fatalCause;
  const causeMessage = primaryError?.message ? `: ${primaryError.message}` : "";
  const tried = attemptedSourcesSummary(diagnostics);
  const error = new Error(
    `${prefix}${causeMessage}. Tried locations: ${tried}. Preferred/tested version: ${EXPECTED_CURSOR_SDK_VERSION}. Install @cursor/sdk in the target workspace, install it globally, or set CUELOOP_CURSOR_SDK_MODULE_PATH to a trusted SDK entrypoint.`,
  );
  error.name = "CursorSdkLoadError";
  error.fatal_cause = fatalCause;
  error.diagnostics = diagnostics;
  return error;
}

function requireString(value, name) {
  if (typeof value !== "string" || value.trim().length === 0) {
    throw new Error(`invalid Cursor SDK runner request: ${name} must be a non-empty string`);
  }
  return value;
}

function modelSelection(request) {
  const id = requireString(request.model, "model");
  const model = { id };
  if (Array.isArray(request.model_params) && request.model_params.length > 0) {
    model.params = request.model_params;
  }
  return model;
}

function settingSources(request) {
  if (!Array.isArray(request.setting_sources) || request.setting_sources.length === 0) {
    return DEFAULT_SETTING_SOURCES;
  }
  return request.setting_sources.filter((source) => typeof source === "string" && source.length > 0);
}

function localOptions(request) {
  return {
    cwd: requireString(request.cwd, "cwd"),
    settingSources: settingSources(request),
    sandboxOptions: { enabled: Boolean(request.sandbox_enabled) },
  };
}

function normalizeTextBlocks(content) {
  if (!Array.isArray(content)) {
    return [];
  }
  return content
    .filter((block) => block && typeof block === "object")
    .map((block) => {
      if (block.type === "text" && typeof block.text === "string") {
        return { type: "text", text: block.text };
      }
      if (block.type === "tool_use") {
        return {
          type: "tool_use",
          id: typeof block.id === "string" ? block.id : "",
          name: typeof block.name === "string" ? block.name : "tool",
          input: block.input,
        };
      }
      return null;
    })
    .filter(Boolean);
}

function assistantTextFromEvent(event) {
  if (event?.type !== "assistant" || !Array.isArray(event.message?.content)) {
    return "";
  }
  return event.message.content
    .filter((block) => block?.type === "text" && typeof block.text === "string")
    .map((block) => block.text)
    .join("");
}

function normalizeEvent(event, agentId, runId) {
  const base = {
    ...event,
    session_id: agentId,
    agent_id: event.agent_id ?? agentId,
    run_id: event.run_id ?? runId,
  };

  switch (event.type) {
    case "assistant":
      return {
        ...base,
        message: {
          role: "assistant",
          content: normalizeTextBlocks(event.message?.content),
        },
      };
    case "tool_call":
      return {
        ...base,
        subtype: event.status,
        tool_call: {
          function: {
            name: event.name ?? "tool",
            args: event.args,
            result: event.result,
          },
        },
      };
    case "thinking":
    case "status":
    case "task":
    case "request":
    case "system":
    case "user":
      return base;
    default:
      return {
        type: "event",
        original_type: event.type,
        session_id: agentId,
        agent_id: agentId,
        run_id: runId,
      };
  }
}

function normalizeError(error, context) {
  return {
    type: "error",
    subtype: "cursor_sdk",
    session_id: context.agentId,
    agent_id: context.agentId,
    run_id: context.runId,
    name: error?.name ?? "Error",
    code: error?.code ?? error?.protoErrorCode,
    is_retryable: Boolean(error?.isRetryable),
    message: String(error?.message ?? error),
    fatal_cause: error?.fatal_cause ?? error?.fatalCause,
    diagnostics: error?.diagnostics,
  };
}

function emitError(error, context = {}) {
  const normalized = normalizeError(error, context);
  emit(normalized);
  process.stderr.write(`${JSON.stringify(normalized)}\n`);
}

async function disposeAgent(agent) {
  if (agent?.[Symbol.asyncDispose]) {
    await agent[Symbol.asyncDispose]();
  } else if (agent?.close) {
    await agent.close();
  }
}

async function run() {
  let agent;
  let runHandle;
  try {
    const request = await readRequest();
    const cwd = requireString(request.cwd, "cwd");
    const { Agent } = await loadCursorSdk(cwd);
    const apiKey = process.env.CURSOR_API_KEY;
    if (!apiKey) {
      throw new Error("CURSOR_API_KEY is required for Cursor SDK local runner execution");
    }

    const options = {
      apiKey,
      model: modelSelection(request),
      local: localOptions(request),
    };

    if (request.operation === "resume") {
      agent = await Agent.resume(requireString(request.agent_id, "agent_id"), options);
    } else if (request.operation === "run") {
      agent = await Agent.create(options);
    } else {
      throw new Error("invalid Cursor SDK runner request: operation must be run or resume");
    }

    emit({
      type: "system",
      subtype: "init",
      session_id: agent.agentId,
      agent_id: agent.agentId,
      runner: "cursor-sdk",
      model: options.model,
    });

    const sendOptions = { model: options.model };
    if (request.force) {
      sendOptions.local = { force: true };
    }

    runHandle = await agent.send(requireString(request.message, "message"), sendOptions);

    let assistantText = "";
    for await (const event of runHandle.stream()) {
      const normalizedEvent = normalizeEvent(event, agent.agentId, runHandle.id);
      assistantText += assistantTextFromEvent(normalizedEvent);
      emit(normalizedEvent);
    }

    const result = await runHandle.wait();
    const finalResult = typeof result.result === "string" && result.result.length > 0
      ? result.result
      : assistantText;
    emit({
      type: "result",
      subtype: result.status,
      is_error: result.status !== "finished",
      result: finalResult,
      session_id: agent.agentId,
      agent_id: agent.agentId,
      run_id: result.id ?? runHandle.id,
      status: result.status,
      duration_ms: result.durationMs,
      model: result.model,
      git: result.git,
    });

    if (result.status !== "finished") {
      process.exitCode = 1;
    }
  } finally {
    await disposeAgent(agent);
  }
}

try {
  await run();
} catch (error) {
  emitError(error);
  process.exitCode = 1;
}
