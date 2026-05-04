// Purpose: Probe local @cursor/sdk candidates for `cueloop doctor` without importing workspace package code.
// Keep the JSON payload aligned with cursor_sdk_runner.mjs load diagnostics.
const fs = require('fs');
const path = require('path');
const { createRequire } = require('module');
const { pathToFileURL } = require('url');
const { execFileSync } = require('child_process');
const preferredVersion = '__CUELOOP_CURSOR_SDK_VERSION__';
const diagnostics = () => ({ preferred_sdk_version: preferredVersion, selected: null, attempted_sources: [], warnings: [], proceeded_best_effort: false, fatal_cause: null });
function addAttempt(d, a) { const n = { source: a.source, location: a.location || null, entrypoint: a.entrypoint || null, package_json: a.package_json || null, sdk_version: a.sdk_version || null, global_root: a.global_root || null, status: a.status, error: a.error ? String(a.error) : null }; d.attempted_sources.push(n); return n; }
function findCursorSdkPackageJson(entrypoint) {
  let current = path.resolve(entrypoint);
  if (!fs.existsSync(current)) return null;
  if (!fs.statSync(current).isDirectory()) current = path.dirname(current);
  while (true) {
    const candidate = path.join(current, 'package.json');
    if (fs.existsSync(candidate)) { try { if (JSON.parse(fs.readFileSync(candidate, 'utf8')).name === '@cursor/sdk') return candidate; } catch { return null; } }
    const parent = path.dirname(current); if (parent === current) return null; current = parent;
  }
}
function metadata(entrypoint) { const packageJsonPath = findCursorSdkPackageJson(entrypoint); if (!packageJsonPath) return { packageJsonPath: null, version: null }; const pkg = JSON.parse(fs.readFileSync(packageJsonPath, 'utf8')); return { packageJsonPath, version: pkg.version || null }; }
function versionWarning(meta) { if (!meta.version) return `Unable to find @cursor/sdk package version; CueLoop's preferred/tested version is ${preferredVersion}; proceeding best-effort`; if (meta.version !== preferredVersion) return `@cursor/sdk ${meta.version} differs from CueLoop's preferred/tested ${preferredVersion}; proceeding best-effort`; return null; }
function globalRoots() { const roots = []; if (process.env.CUELOOP_CURSOR_SDK_GLOBAL_ROOT) return [path.resolve(process.env.CUELOOP_CURSOR_SDK_GLOBAL_ROOT)]; try { const root = execFileSync('npm', ['root', '-g'], { encoding: 'utf8', stdio: ['ignore', 'pipe', 'ignore'] }).trim(); if (root) roots.push(root); } catch {} return [...new Set(roots.map((root) => path.resolve(root)))]; }
function resolveFromGlobalRoot(root) { const packageJson = path.join(root, '@cursor', 'sdk', 'package.json'); if (fs.existsSync(packageJson)) return createRequire(packageJson).resolve('./'); return createRequire(path.join(root, 'package.json')).resolve('@cursor/sdk', { paths: [root] }); }
function normalizeSdkModule(moduleNamespace) { const candidates = [moduleNamespace, moduleNamespace && moduleNamespace.default, moduleNamespace && moduleNamespace.default && moduleNamespace.default.default]; if (!candidates.find((candidate) => candidate && candidate.Agent)) { const error = new Error('Loaded @cursor/sdk module does not expose required export Agent'); error.fatal_cause = 'incompatible_api'; throw error; } }
function importStatus(error) { return error && (error.fatal_cause === 'incompatible_api' || String(error.message || error).includes('does not expose')) ? 'incompatible_api' : 'import_failed'; }
function markWarning(d, meta) { const warning = versionWarning(meta); if (warning) { d.warnings.push(warning); d.proceeded_best_effort = true; } }
async function selectCandidate() {
  const d = diagnostics(); const configured = process.env.CUELOOP_CURSOR_SDK_MODULE_PATH;
  if (configured) {
    const entrypoint = path.resolve(configured); const attempt = addAttempt(d, { source: 'env', location: configured, entrypoint, status: fs.existsSync(entrypoint) ? 'resolved' : 'invalid_path', error: fs.existsSync(entrypoint) ? null : 'path does not exist' });
    try { const meta = metadata(entrypoint); attempt.package_json = meta.packageJsonPath; attempt.sdk_version = meta.version; d.selected = { source: 'env', entrypoint, package_json: meta.packageJsonPath, sdk_version: meta.version, global_root: null }; normalizeSdkModule(await import(pathToFileURL(entrypoint).href)); attempt.status = 'selected'; markWarning(d, meta); return d; }
    catch (error) { attempt.status = attempt.status === 'invalid_path' ? 'invalid_path' : importStatus(error); attempt.error = String(error && error.message || error); d.fatal_cause = 'invalid_module_path'; throw Object.assign(new Error(`CUELOOP_CURSOR_SDK_MODULE_PATH is set but unusable: ${entrypoint}: ${attempt.error}`), { diagnostics: d }); }
  }
  addAttempt(d, { source: 'env', location: 'CUELOOP_CURSOR_SDK_MODULE_PATH (unset)', status: 'not_found', error: 'environment variable is not set' });
  const workspace = addAttempt(d, { source: 'workspace', location: process.cwd(), status: 'not_found' });
  try { const resolved = path.resolve(createRequire(path.join(process.cwd(), 'package.json')).resolve('@cursor/sdk', { paths: [process.cwd()] })); const meta = metadata(resolved); Object.assign(workspace, { entrypoint: resolved, package_json: meta.packageJsonPath, sdk_version: meta.version, status: 'selected' }); d.selected = { source: 'workspace', entrypoint: resolved, package_json: meta.packageJsonPath, sdk_version: meta.version, global_root: null }; markWarning(d, meta); return d; }
  catch (error) { workspace.error = String(error && error.message || error); }
  const roots = globalRoots(); if (roots.length === 0) addAttempt(d, { source: 'global', location: 'global npm roots', status: 'not_found', error: 'no global npm root discovered' });
  for (const root of roots) {
    const attempt = addAttempt(d, { source: 'global', location: root, global_root: root, status: 'not_found' }); let resolved;
    try { resolved = path.resolve(resolveFromGlobalRoot(root)); const meta = metadata(resolved); Object.assign(attempt, { entrypoint: resolved, package_json: meta.packageJsonPath, sdk_version: meta.version, status: 'resolved' }); d.selected = { source: 'global', entrypoint: resolved, package_json: meta.packageJsonPath, sdk_version: meta.version, global_root: root }; normalizeSdkModule(await import(pathToFileURL(resolved).href)); attempt.status = 'selected'; markWarning(d, meta); return d; }
    catch (error) { attempt.status = resolved ? importStatus(error) : 'not_found'; attempt.error = String(error && error.message || error); if (resolved) { d.fatal_cause = importStatus(error) === 'incompatible_api' ? 'incompatible_api' : 'import_failed'; throw Object.assign(new Error(`Global @cursor/sdk from ${root} is unusable: ${attempt.error}`), { diagnostics: d }); } }
  }
  d.fatal_cause = 'missing_sdk'; throw Object.assign(new Error('@cursor/sdk package was not found; tried CUELOOP_CURSOR_SDK_MODULE_PATH, workspace, and global npm roots'), { diagnostics: d });
}
selectCandidate().then((d) => process.stdout.write(JSON.stringify(d))).catch((error) => { const payload = error.diagnostics || { fatal_cause: 'missing_sdk', message: String(error && error.message || error) }; payload.message = String(error && error.message || error); console.error(JSON.stringify(payload)); process.exit(1); });
