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

async function loadCursorSdk(cwd) {
  const configuredPath = process.env.CUELOOP_CURSOR_SDK_MODULE_PATH;
  if (configuredPath) {
    try {
      return await importCursorSdkFromPath(configuredPath, "env");
    } catch (configuredError) {
      throw sdkLoadError(configuredError, null, null, null, "CUELOOP_CURSOR_SDK_MODULE_PATH is set but unusable");
    }
  }

  const attempts = [];
  const requireFromCwd = createRequire(path.join(cwd, "package.json"));
  let workspaceResolved = null;
  try {
    workspaceResolved = requireFromCwd.resolve("@cursor/sdk", { paths: [cwd] });
  } catch (cwdResolveError) {
    attempts.push(["workspace", cwdResolveError]);
  }
  if (workspaceResolved) {
    try {
      return await importCursorSdkFromPath(workspaceResolved, "workspace");
    } catch (cwdImportError) {
      throw sdkLoadError(cwdImportError, null, cwdImportError, null, "Workspace @cursor/sdk is unusable");
    }
  }

  for (const globalRoot of await discoverGlobalNpmRoots()) {
    let globalResolved = null;
    try {
      globalResolved = resolveCursorSdkFromGlobalRoot(globalRoot);
    } catch (globalResolveError) {
      attempts.push([`global:${globalRoot}`, globalResolveError]);
      continue;
    }
    try {
      return await importCursorSdkFromPath(globalResolved, "global");
    } catch (globalImportError) {
      throw sdkLoadError(globalImportError, null, null, globalImportError, `Global @cursor/sdk from ${globalRoot} is unusable`);
    }
  }

  throw sdkLoadError(attempts[0]?.[1], null, attempts[0]?.[1], attempts.at(-1)?.[1]);
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

async function importCursorSdkFromPath(entrypoint, source) {
  const resolvedEntrypoint = path.resolve(entrypoint);
  const metadata = cursorSdkMetadata(resolvedEntrypoint);
  const sdk = normalizeCursorSdkModule(await import(pathToFileURL(resolvedEntrypoint).href));
  const warning = cursorSdkVersionWarning(metadata);
  if (warning) {
    emit({
      type: "system",
      subtype: "cursor_sdk_warning",
      warning,
      sdk_version: metadata.version,
      preferred_sdk_version: EXPECTED_CURSOR_SDK_VERSION,
      sdk_entrypoint: resolvedEntrypoint,
      sdk_source: source,
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
    const error = new Error("Loaded @cursor/sdk module does not expose Agent");
    error.name = "CursorSdkLoadError";
    throw error;
  }
  return sdk;
}

function sdkLoadError(primaryError, directError, cwdError, globalError, prefix = "Unable to load @cursor/sdk") {
  const causeMessage = primaryError?.message ? `: ${primaryError.message}` : "";
  const error = new Error(
    `${prefix}${causeMessage}. Install @cursor/sdk in the target workspace (preferred/tested: \`npm install --save-exact @cursor/sdk@${EXPECTED_CURSOR_SDK_VERSION}\`), install it globally, or set CUELOOP_CURSOR_SDK_MODULE_PATH to the SDK entrypoint.`,
  );
  error.name = "CursorSdkLoadError";
  error.cause = {
    primary: String(primaryError?.message ?? primaryError),
    direct: String(directError?.message ?? directError),
    cwd: String(cwdError?.message ?? cwdError),
    global: String(globalError?.message ?? globalError),
  };
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
