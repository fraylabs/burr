#!/usr/bin/env node

import fs from "node:fs"
import http from "node:http"
import net from "node:net"
import os from "node:os"
import path from "node:path"
import { execFileSync, spawn } from "node:child_process"
import { fileURLToPath } from "node:url"

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..")
const cargoTargetDirectory = JSON.parse(
  execFileSync("cargo", ["metadata", "--no-deps", "--format-version", "1"], {
    cwd: repoRoot,
    encoding: "utf8",
  }),
).target_directory
const burrExecutable = process.platform === "win32" ? "burr.exe" : "burr"
const fixture = path.join(
  repoRoot,
  "tests",
  "fixtures",
  "viewer",
  "models",
  "enclosure",
  "counterbore.step",
)
const interferenceFixtures = path.join(repoRoot, "tests", "fixtures", "interference")
const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), "burr-viewer-check-"))
const cacheDirectory = path.join(tempRoot, "cache")
const modelDirectory = path.join(tempRoot, "models", "enclosure")
const modelPath = path.join(modelDirectory, "counterbore.step")
const assemblyDirectory = path.join(tempRoot, "models", "assemblies")
const ignoredDirectory = path.join(tempRoot, "notes")
const unconfiguredModelDirectory = path.join(tempRoot, "archive")
const port = await availablePort()
const baseUrl = `http://127.0.0.1:${port}`
let replayChild = null
let replayClosed = null

fs.mkdirSync(modelDirectory, { recursive: true })
fs.mkdirSync(assemblyDirectory, { recursive: true })
fs.mkdirSync(ignoredDirectory, { recursive: true })
fs.mkdirSync(unconfiguredModelDirectory, { recursive: true })
fs.mkdirSync(path.join(tempRoot, ".burr"), { recursive: true })
fs.copyFileSync(fixture, modelPath)
for (const name of ["contained.step", "intersecting.step", "separated.step", "touching.step"]) {
  fs.copyFileSync(path.join(interferenceFixtures, name), path.join(assemblyDirectory, name))
}
fs.copyFileSync(
  path.join(interferenceFixtures, "separated.step"),
  path.join(assemblyDirectory, "separated-folded.step"),
)
fs.copyFileSync(fixture, path.join(unconfiguredModelDirectory, "not-configured.step"))
fs.writeFileSync(path.join(ignoredDirectory, "readme.txt"), "not a model\n")
fs.writeFileSync(
  path.join(tempRoot, ".burr", "config.toml"),
  [
    'schema_version = "burr.project.v1"',
    "",
    "[project]",
    'models = ["models"]',
    "",
    "[[motions]]",
    'id = "fold"',
    'label = "Fold assembly"',
    'from = "models/assemblies/separated.step"',
    'from_label = "Deployed"',
    'to = "models/assemblies/separated-folded.step"',
    'to_label = "Folded"',
    "duration_ms = 800",
    "",
  ].join("\n"),
)

let stdout = ""
let stderr = ""
const child = spawn(
  "cargo",
  ["run", "--locked", "--quiet", "--manifest-path", path.join(repoRoot, "Cargo.toml"), "--", "."],
  {
    cwd: tempRoot,
    env: {
      ...process.env,
      BURR_CACHE_DIR: cacheDirectory,
      BURR_VIEWER_NO_OPEN: "1",
      BURR_VIEWER_PORT: String(port),
    },
    stdio: ["ignore", "pipe", "pipe"],
  },
)
const childClosed = new Promise((resolve) => child.once("close", resolve))
child.stdout.on("data", (chunk) => {
  stdout += chunk
})
child.stderr.on("data", (chunk) => {
  stderr += chunk
})

try {
  await waitForServer()

  expectEqual(
    await requestStatusWithHost("attacker.example"),
    403,
    "DNS-rebinding Host rejection",
  )

  const project = await getJson("/api/project")
  expectEqual(project.schema_version, "burr.project-state.v1", "project state schema")
  expectEqual(project.configured, true, "configured project state")
  expectEqual(project.config_path, ".burr/config.toml", "portable config path")
  expectEqual(project.model_paths?.length, 1, "configured model root count")
  expectEqual(project.model_paths?.[0], "models", "configured model root")
  expectEqual(project.motions?.length, 1, "configured motion count")
  expectEqual(project.motions?.[0]?.id, "fold", "configured motion id")
  expectEqual(
    Object.keys(project).sort().join(","),
    "config_path,configured,model_paths,motions,root,schema_version",
    "closed project state",
  )

  const initialTree = await getJson("/api/tree")
  expectEqual(initialTree.files?.length, 6, "filtered model count")
  expectEqual(initialTree.motions?.[0]?.from_label, "Deployed", "tree motion start label")
  expectEqual(initialTree.motions?.[0]?.to_label, "Folded", "tree motion end label")
  const counterbore = model(initialTree, "models/enclosure/counterbore.step")
  expectEqual(counterbore.format, "STEP", "model format")
  const initialVersion = counterbore.version

  const shellResponse = await fetch(`${baseUrl}/`)
  expectEqual(shellResponse.status, 200, "workbench shell status")
  const shellHtml = await shellResponse.text()
  expectIncludes(shellHtml, 'id="checks-tab"', "checks tab")
  expectIncludes(shellHtml, 'id="checks-panel"', "checks panel")
  expectIncludes(shellHtml, 'data-render-mode="x-ray"', "X-ray control")
  expectIncludes(shellHtml, 'data-render-mode="solid"', "solid control")
  expectIncludes(shellHtml, 'id="snapshot-button"', "snapshot control")
  expectIncludes(shellHtml, 'id="motion-controls"', "motion controls")
  expectIncludes(shellHtml, 'id="motion-progress"', "motion scrubber")
  expectIncludes(shellHtml, 'id="loading-stage"', "progressive loading stage")
  expectIncludes(shellHtml, 'aria-live="polite"', "accessible loading announcements")
  expectIncludes(shellHtml, "/api/load-status?", "load-status polling")
  expectIncludes(shellHtml, "new AbortController()", "superseded load cancellation")
  expectIncludes(shellHtml, 'type === "burr:viewer-ready"', "event-specific viewer readiness")
  expectIncludes(
    shellHtml,
    "Checks begin after the model is visible.",
    "model-first check scheduling",
  )
  expectIncludes(shellHtml, "Open the Checks tab", "on-demand check guidance")
  expectIncludes(shellHtml, 'type: "burr:toggle-motion"', "motion playback dispatch")
  expectIncludes(shellHtml, "snapshotFilename(state.selectedPath)", "snapshot filename generation")
  expectIncludes(shellHtml, 'type: "burr:export-snapshot"', "snapshot request dispatch")
  expectIncludes(shellHtml, "burr:snapshot-exported", "snapshot completion feedback")

  const singleReport = await getJson(
    `/api/checks?path=${encodeURIComponent(counterbore.path)}`,
  )
  expectEqual(singleReport.schema_version, "burr.checks.v1", "check report schema")
  expectEqual(singleReport.check_id, "assembly-interference", "geometry-native check id")
  expectEqual(singleReport.outcome, "incomplete", "single-part check outcome")
  expectEqual(singleReport.incomplete_reasons?.[0]?.code, "assembly_required", "single-part reason")

  const separated = model(initialTree, "models/assemblies/separated.step")
  const separatedReport = await getJson(
    `/api/checks?path=${encodeURIComponent(separated.path)}`,
  )
  expectEqual(separatedReport.outcome, "pass", "separated assembly outcome")
  expectEqual(separatedReport.checked_pair_count, 1, "separated pair count")

  const touching = model(initialTree, "models/assemblies/touching.step")
  const touchingReport = await getJson(`/api/checks?path=${encodeURIComponent(touching.path)}`)
  expectEqual(touchingReport.outcome, "pass", "face-touching assembly outcome")

  const intersecting = model(initialTree, "models/assemblies/intersecting.step")
  const intersectingReport = await getJson(
    `/api/checks?path=${encodeURIComponent(intersecting.path)}`,
  )
  expectEqual(intersectingReport.outcome, "fail", "intersecting assembly outcome")
  expectEqual(intersectingReport.findings?.length, 1, "interference finding count")
  expectEqual(
    intersectingReport.findings?.[0]?.witness?.kind,
    "surface_crossing",
    "interference witness kind",
  )

  const contained = model(initialTree, "models/assemblies/contained.step")
  const containedReport = await getJson(`/api/checks?path=${encodeURIComponent(contained.path)}`)
  expectEqual(containedReport.outcome, "fail", "contained assembly outcome")
  expectEqual(
    containedReport.findings?.[0]?.witness?.kind,
    "containment",
    "containment witness kind",
  )

  const logoResponse = await fetch(`${baseUrl}/assets/burr-logo.png`)
  expectEqual(logoResponse.status, 200, "logo response status")
  expectEqual(logoResponse.headers.get("content-type"), "image/png", "logo content type")
  const logoBytes = new Uint8Array(await logoResponse.arrayBuffer())
  expectEqual(logoBytes.length > 100_000, true, "logo asset size")
  expectEqual(
    [...logoBytes.slice(0, 8)].join(","),
    "137,80,78,71,13,10,26,10",
    "logo PNG signature",
  )

  const viewerResponse = await fetch(
    `${baseUrl}/viewer?path=${encodeURIComponent(counterbore.path)}&theme=dark&load=proof-generated`,
  )
  expectEqual(viewerResponse.status, 200, "viewer response status")
  const viewerHtml = await viewerResponse.text()
  expectIncludes(viewerHtml, '<canvas id="gl-canvas">', "Look WebGL canvas")
  expectIncludes(viewerHtml, "STEP B-REP", "STEP format badge")
  expectIncludes(viewerHtml, "counterbore.step", "model label")
  expectIncludes(viewerHtml, 'data-burr-theme="dark"', "dark viewer theme marker")
  expectIncludes(viewerHtml, "background-color: #0c0d10", "dark viewer surface")
  expectIncludes(
    viewerHtml,
    'name="burr-render-modes" content="x-ray,solid"',
    "viewer render modes",
  )
  expectIncludes(viewerHtml, 'let burrRenderMode = "x-ray"', "default render mode")
  expectIncludes(viewerHtml, "fragColor = vec4(col, uOpacity);", "transparent shader output")
  expectIncludes(viewerHtml, 'name="burr-snapshot-export" content="png"', "PNG export marker")
  expectIncludes(viewerHtml, "burr:export-snapshot", "snapshot request listener")
  expectIncludes(viewerHtml, "canvas.toBlob", "canvas PNG export")
  expectIncludes(viewerHtml, 'type: "burr:viewer-ready"', "specific load completion message")
  const generatedStatus = await getJson("/api/load-status?id=proof-generated")
  expectEqual(generatedStatus.schema_version, "burr.load-status.v1", "load status schema")
  expectEqual(generatedStatus.state, "ready", "generated viewer ready state")
  expectEqual(generatedStatus.cache, "generated", "generated viewer cache outcome")

  const memoryViewerResponse = await fetch(
    `${baseUrl}/viewer?path=${encodeURIComponent(counterbore.path)}&theme=dark&load=proof-memory`,
  )
  expectEqual(memoryViewerResponse.status, 200, "memory-cached viewer response status")
  const memoryViewerHtml = await memoryViewerResponse.text()
  expectEqual(
    viewerWithoutLoadIdentity(memoryViewerHtml),
    viewerWithoutLoadIdentity(viewerHtml),
    "memory-cached viewer output",
  )
  const memoryStatus = await getJson("/api/load-status?id=proof-memory")
  expectEqual(memoryStatus.cache, "memory", "same-process viewer cache outcome")

  const replayPort = await availablePort()
  const replayBaseUrl = `http://127.0.0.1:${replayPort}`
  replayChild = spawn(path.join(cargoTargetDirectory, "debug", burrExecutable), ["."], {
    cwd: tempRoot,
    env: {
      ...process.env,
      BURR_CACHE_DIR: cacheDirectory,
      BURR_VIEWER_NO_OPEN: "1",
      BURR_VIEWER_PORT: String(replayPort),
    },
    stdio: "ignore",
  })
  replayClosed = new Promise((resolve) => replayChild.once("close", resolve))
  await waitForServerAt(replayChild, replayBaseUrl)
  const diskViewerResponse = await fetch(
    `${replayBaseUrl}/viewer?path=${encodeURIComponent(counterbore.path)}&theme=dark&load=proof-disk`,
  )
  expectEqual(diskViewerResponse.status, 200, "disk-cached viewer response status")
  expectEqual(
    viewerWithoutLoadIdentity(await diskViewerResponse.text()),
    viewerWithoutLoadIdentity(viewerHtml),
    "disk-cached viewer output",
  )
  const diskStatus = await getJsonAt(replayBaseUrl, "/api/load-status?id=proof-disk")
  expectEqual(diskStatus.cache, "disk", "cross-process viewer cache outcome")
  replayChild.kill("SIGTERM")
  await Promise.race([replayClosed, delay(2_000)])
  replayChild = null
  replayClosed = null

  const motionViewerResponse = await fetch(
    `${baseUrl}/viewer?path=${encodeURIComponent("models/assemblies/separated.step")}&motion=fold`,
  )
  expectEqual(motionViewerResponse.status, 200, "motion viewer response status")
  const motionViewerHtml = await motionViewerResponse.text()
  expectIncludes(
    motionViewerHtml,
    'name="burr-motion" content="rigid-poses"',
    "rigid motion marker",
  )
  expectIncludes(motionViewerHtml, "uBurrInstanceTransforms[32]", "motion transform shader")
  expectIncludes(motionViewerHtml, "burr:set-motion-progress", "motion scrub listener")
  expectIncludes(motionViewerHtml, "burr:motion-state", "motion playback state")

  const unknownMotion = await fetch(
    `${baseUrl}/viewer?path=${encodeURIComponent("models/assemblies/separated.step")}&motion=unknown`,
  )
  expectEqual(unknownMotion.status, 422, "unknown motion status")

  const lightViewerResponse = await fetch(
    `${baseUrl}/viewer?path=${encodeURIComponent(counterbore.path)}&theme=light`,
  )
  expectEqual(lightViewerResponse.status, 200, "light viewer response status")
  const lightViewerHtml = await lightViewerResponse.text()
  expectIncludes(lightViewerHtml, 'data-burr-theme="light"', "light viewer theme marker")
  expectIncludes(lightViewerHtml, "background-color: #c9ced0", "light viewer surface")
  expectEqual(lightViewerHtml === viewerHtml, false, "theme-specific viewer output")

  const focusedComponents = intersectingReport.findings[0].components.map(
    (component) => component.occurrence_index,
  )
  const focusedViewerResponse = await fetch(
    `${baseUrl}/viewer?path=${encodeURIComponent(intersecting.path)}&theme=dark&focus=${focusedComponents.join(",")}`,
  )
  expectEqual(focusedViewerResponse.status, 200, "focused viewer response status")
  const focusedViewerHtml = await focusedViewerResponse.text()
  expectIncludes(
    focusedViewerHtml,
    `name="burr-highlighted-components" content="${focusedComponents.join(",")}"`,
    "component highlight marker",
  )
  expectEqual(
    focusedViewerHtml ===
      (await (
        await fetch(`${baseUrl}/viewer?path=${encodeURIComponent(intersecting.path)}&theme=dark`)
      ).text()),
    false,
    "focused viewer colors differ",
  )

  const invalidFocus = await fetch(
    `${baseUrl}/viewer?path=${encodeURIComponent(intersecting.path)}&focus=0,0`,
  )
  expectEqual(invalidFocus.status, 400, "invalid component focus status")

  const traversal = await fetch(`${baseUrl}/viewer?path=..%2FCargo.toml`)
  expectEqual(traversal.status, 422, "path traversal status")
  const unsupported = await fetch(
    `${baseUrl}/viewer?path=${encodeURIComponent("notes/readme.txt")}`,
  )
  expectEqual(unsupported.status, 422, "unsupported file status")
  const checkTraversal = await fetch(`${baseUrl}/api/checks?path=..%2FCargo.toml`)
  expectEqual(checkTraversal.status, 422, "check path traversal status")

  fs.appendFileSync(modelPath, "\n")
  const updatedTree = await waitForVersionChange(initialVersion)
  const updatedVersion = model(updatedTree, counterbore.path).version
  if (updatedVersion === initialVersion) {
    throw new Error("viewer watcher did not detect the changed STEP file")
  }

  const refreshedViewer = await fetch(
    `${baseUrl}/viewer?path=${encodeURIComponent(counterbore.path)}&v=${encodeURIComponent(updatedVersion)}&load=proof-invalidated`,
  )
  expectEqual(refreshedViewer.status, 200, "refreshed viewer status")
  expectIncludes(await refreshedViewer.text(), "STEP B-REP", "refreshed Look viewer")
  const invalidatedStatus = await getJson("/api/load-status?id=proof-invalidated")
  expectEqual(invalidatedStatus.cache, "generated", "changed source invalidates viewer cache")
  const refreshedReport = await getJson(
    `/api/checks?path=${encodeURIComponent(counterbore.path)}`,
  )
  expectEqual(refreshedReport.model_version, updatedVersion, "refreshed check version")
  expectIncludes(stdout, `OPEN ${baseUrl}/`, "printed viewer URL")

  console.log(
    `viewer proof passed (loopback Host enforced, model scope enforced, progressive load status exposed, cross-process viewer cache reused and invalidated, named STEP motion rendered, STEP interference pass/fail/incomplete proven, X-ray rendering and PNG export available, components highlighted, watcher refreshed, traversal rejected)`,
  )
} catch (error) {
  if (stdout) process.stderr.write(`viewer stdout:\n${stdout}`)
  if (stderr) process.stderr.write(`viewer stderr:\n${stderr}`)
  throw error
} finally {
  if (replayChild) {
    replayChild.kill("SIGTERM")
    await Promise.race([replayClosed, delay(2_000)])
  }
  child.kill("SIGTERM")
  await Promise.race([childClosed, delay(2_000)])
  fs.rmSync(tempRoot, { recursive: true, force: true })
}

async function availablePort() {
  return new Promise((resolve, reject) => {
    const server = net.createServer()
    server.once("error", reject)
    server.listen(0, "127.0.0.1", () => {
      const address = server.address()
      const selected = typeof address === "object" && address ? address.port : null
      server.close((error) => {
        if (error) reject(error)
        else if (selected === null) reject(new Error("could not allocate viewer test port"))
        else resolve(selected)
      })
    })
  })
}

async function requestStatusWithHost(host) {
  return new Promise((resolve, reject) => {
    const request = http.request(
      {
        hostname: "127.0.0.1",
        port,
        path: "/api/project",
        headers: { Host: host },
      },
      (response) => {
        response.resume()
        response.once("end", () => resolve(response.statusCode))
      },
    )
    request.once("error", reject)
    request.end()
  })
}

async function waitForServer() {
  return waitForServerAt(child, baseUrl)
}

async function waitForServerAt(process, url) {
  for (let attempt = 0; attempt < 240; attempt += 1) {
    if (process.exitCode !== null) {
      throw new Error(`viewer exited before becoming ready with code ${process.exitCode}`)
    }
    try {
      const response = await fetch(`${url}/api/health`)
      if (response.ok) return
    } catch {
      // The initial Rust build may still be running.
    }
    await delay(250)
  }
  throw new Error("viewer did not become ready within 60 seconds")
}

async function waitForVersionChange(initialVersion) {
  for (let attempt = 0; attempt < 40; attempt += 1) {
    const tree = await getJson("/api/tree")
    const current = model(tree, "models/enclosure/counterbore.step")
    if (current.version !== initialVersion) return tree
    await delay(100)
  }
  throw new Error("viewer did not observe the changed model within 4 seconds")
}

function model(tree, wantedPath) {
  const found = tree.files?.find((file) => file.path === wantedPath)
  if (!found) throw new Error(`model tree did not contain ${wantedPath}`)
  return found
}

async function getJson(route) {
  return getJsonAt(baseUrl, route)
}

async function getJsonAt(url, route) {
  const response = await fetch(`${url}${route}`, { cache: "no-store" })
  if (!response.ok) throw new Error(`${route} returned ${response.status}`)
  return response.json()
}

function viewerWithoutLoadIdentity(html) {
  const start = html.indexOf("<!--burr-load-id-start-->")
  const endMarker = "<!--burr-load-id-end-->"
  const end = html.indexOf(endMarker, start)
  if (start < 0 || end < 0) throw new Error("viewer did not contain its load identity")
  return `${html.slice(0, start)}${html.slice(end + endMarker.length)}`
}

function expectEqual(actual, expected, label) {
  if (actual !== expected) {
    throw new Error(`${label}: expected ${JSON.stringify(expected)}, got ${JSON.stringify(actual)}`)
  }
}

function expectIncludes(actual, expected, label) {
  if (!actual.includes(expected)) {
    throw new Error(`${label}: expected ${JSON.stringify(expected)}`)
  }
}

function delay(milliseconds) {
  return new Promise((resolve) => setTimeout(resolve, milliseconds))
}
