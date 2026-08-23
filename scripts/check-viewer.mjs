#!/usr/bin/env node

import fs from "node:fs"
import net from "node:net"
import os from "node:os"
import path from "node:path"
import { spawn } from "node:child_process"
import { fileURLToPath } from "node:url"

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..")
const fixture = path.join(
  repoRoot,
  "tests",
  "fixtures",
  "viewer",
  "models",
  "enclosure",
  "counterbore.step",
)
const mechanicalFitFixture = path.join(repoRoot, "examples", "linear-actuator-good")
const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), "burr-viewer-check-"))
const modelDirectory = path.join(tempRoot, "models", "enclosure")
const modelPath = path.join(modelDirectory, "counterbore.step")
const checkedModelDirectory = path.join(tempRoot, "models", "z-check")
const ignoredDirectory = path.join(tempRoot, "notes")
const unconfiguredModelDirectory = path.join(tempRoot, "archive")
const burrPackDirectory = path.join(tempRoot, ".burr", "packs")
const port = await availablePort()
const baseUrl = `http://127.0.0.1:${port}`

fs.mkdirSync(modelDirectory, { recursive: true })
fs.mkdirSync(checkedModelDirectory, { recursive: true })
fs.mkdirSync(ignoredDirectory, { recursive: true })
fs.mkdirSync(unconfiguredModelDirectory, { recursive: true })
fs.mkdirSync(burrPackDirectory, { recursive: true })
fs.copyFileSync(fixture, modelPath)
for (const file of ["source.py", "actuator.step", "burr-design-data.json"]) {
  fs.copyFileSync(path.join(mechanicalFitFixture, file), path.join(checkedModelDirectory, file))
}
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
    "[[packs]]",
    'id = "builtin:mechanical-fit"',
    "",
    "[[packs]]",
    'path = "packs/product-fit.toml"',
    "",
  ].join("\n"),
)
fs.writeFileSync(
  path.join(burrPackDirectory, "product-fit.toml"),
  [
    'schema_version = "burr.pack.v1"',
    'id = "project:product-fit"',
    'version = "0.1.0"',
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

  const project = await getJson("/api/project")
  expectEqual(project.schema_version, "burr.project-state.v1", "project state schema")
  expectEqual(project.configured, true, "configured project state")
  expectEqual(project.config_path, ".burr/config.toml", "portable config path")
  expectEqual(project.model_paths?.length, 1, "configured model root count")
  expectEqual(project.model_paths?.[0], "models", "configured model root")
  expectEqual(project.packs?.length, 2, "resolved pack count")
  expectEqual(project.packs?.[0]?.id, "builtin:mechanical-fit", "built-in pack id")
  expectEqual(project.packs?.[0]?.source, "builtin", "built-in pack source")
  expectEqual(project.packs?.[1]?.id, "project:product-fit", "local pack id")
  expectEqual(project.packs?.[1]?.source, "local", "local pack source")
  expectEqual(
    project.packs?.[1]?.path,
    ".burr/packs/product-fit.toml",
    "portable local pack path",
  )

  const checks = await getJson("/api/checks")
  expectEqual(checks.schema_version, "burr.check-results.v1", "check result schema")
  expectEqual(
    checks.capability_catalog?.join(","),
    "mesh,brep,assembly,declared_intent",
    "shared capability catalog",
  )
  expectEqual(checks.outcome, "incomplete", "aggregate check outcome")
  expectEqual(checks.packs?.length, 2, "executed pack count")
  const mechanicalFit = checks.packs?.[0]
  expectEqual(mechanicalFit?.id, "builtin:mechanical-fit", "executed built-in pack")
  expectEqual(mechanicalFit?.outcome, "pass", "mechanical-fit pack outcome")
  expectEqual(
    mechanicalFit?.required_capabilities?.join(","),
    "declared_intent",
    "mechanical-fit required capability",
  )
  expectEqual(
    mechanicalFit?.available_capabilities?.join(","),
    "declared_intent",
    "mechanical-fit available capability",
  )
  expectEqual(mechanicalFit?.targets?.length, 1, "mechanical-fit target count")
  expectEqual(
    mechanicalFit?.targets?.[0]?.source_path,
    "models/z-check/burr-design-data.json",
    "portable checked source path",
  )
  expectEqual(mechanicalFit?.summary?.targets_passed, 1, "passed target count")
  const localPack = checks.packs?.[1]
  expectEqual(localPack?.outcome, "incomplete", "unimplemented local pack outcome")
  expectEqual(
    localPack?.findings?.[0]?.code,
    "local_pack_runtime_unavailable",
    "local pack incomplete reason",
  )
  expectEqual(
    fs.existsSync(path.join(checkedModelDirectory, "burr-receipt.json")),
    false,
    "interactive checks do not write receipts",
  )

  const shellResponse = await fetch(baseUrl)
  expectEqual(shellResponse.status, 200, "workbench shell status")
  const shellHtml = await shellResponse.text()
  expectIncludes(shellHtml, 'role="tablist"', "project tools tab list")
  expectIncludes(shellHtml, 'id="checks-panel"', "visual checks panel")
  expectIncludes(shellHtml, 'fetch("/api/checks"', "checks API client")
  expectIncludes(shellHtml, "item.dataset.findingCode", "structured finding rows")

  const initialTree = await getJson("/api/tree")
  expectEqual(initialTree.files?.length, 2, "filtered model count")
  expectEqual(initialTree.files?.[0]?.path, "models/enclosure/counterbore.step", "model path")
  expectEqual(initialTree.files?.[0]?.format, "STEP", "model format")
  const initialVersion = initialTree.files[0].version

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
    `${baseUrl}/viewer?path=${encodeURIComponent(initialTree.files[0].path)}&theme=dark`,
  )
  expectEqual(viewerResponse.status, 200, "viewer response status")
  const viewerHtml = await viewerResponse.text()
  expectIncludes(viewerHtml, '<canvas id="gl-canvas">', "Look WebGL canvas")
  expectIncludes(viewerHtml, "STEP B-REP", "STEP format badge")
  expectIncludes(viewerHtml, "counterbore.step", "model label")
  expectIncludes(viewerHtml, 'data-burr-theme="dark"', "dark viewer theme marker")
  expectIncludes(viewerHtml, "background-color: #0c0d10", "dark viewer surface")

  const lightViewerResponse = await fetch(
    `${baseUrl}/viewer?path=${encodeURIComponent(initialTree.files[0].path)}&theme=light`,
  )
  expectEqual(lightViewerResponse.status, 200, "light viewer response status")
  const lightViewerHtml = await lightViewerResponse.text()
  expectIncludes(lightViewerHtml, 'data-burr-theme="light"', "light viewer theme marker")
  expectIncludes(lightViewerHtml, "background-color: #c9ced0", "light viewer surface")
  expectEqual(lightViewerHtml === viewerHtml, false, "theme-specific viewer output")

  const traversal = await fetch(`${baseUrl}/viewer?path=..%2FCargo.toml`)
  expectEqual(traversal.status, 422, "path traversal status")
  const unsupported = await fetch(
    `${baseUrl}/viewer?path=${encodeURIComponent("notes/readme.txt")}`,
  )
  expectEqual(unsupported.status, 422, "unsupported file status")

  fs.appendFileSync(modelPath, "\n")
  const updatedTree = await waitForVersionChange(initialVersion)
  const updatedVersion = updatedTree.files[0].version
  if (updatedVersion === initialVersion) {
    throw new Error("viewer watcher did not detect the changed STEP file")
  }

  const refreshedViewer = await fetch(
    `${baseUrl}/viewer?path=${encodeURIComponent(updatedTree.files[0].path)}&v=${encodeURIComponent(updatedVersion)}`,
  )
  expectEqual(refreshedViewer.status, 200, "refreshed viewer status")
  expectIncludes(await refreshedViewer.text(), "STEP B-REP", "refreshed Look viewer")
  expectIncludes(stdout, `OPEN ${baseUrl}/`, "printed viewer URL")
  expectIncludes(stdout, "CONFIGURED PACKS 2", "resolved pack startup summary")
  expectIncludes(stdout, "CHECKS INCOMPLETE", "check startup summary")

  console.log(
    `viewer proof passed (checks panel wired, mechanical-fit executed read-only, incomplete local pack stayed explicit, model scope enforced, Look rendered, watcher refreshed, traversal rejected)`,
  )
} catch (error) {
  if (stdout) process.stderr.write(`viewer stdout:\n${stdout}`)
  if (stderr) process.stderr.write(`viewer stderr:\n${stderr}`)
  throw error
} finally {
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

async function waitForServer() {
  for (let attempt = 0; attempt < 240; attempt += 1) {
    if (child.exitCode !== null) {
      throw new Error(`viewer exited before becoming ready with code ${child.exitCode}`)
    }
    try {
      const response = await fetch(`${baseUrl}/api/health`)
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
    if (tree.files?.[0]?.version !== initialVersion) return tree
    await delay(100)
  }
  throw new Error("viewer did not observe the changed model within 4 seconds")
}

async function getJson(route) {
  const response = await fetch(`${baseUrl}${route}`, { cache: "no-store" })
  if (!response.ok) throw new Error(`${route} returned ${response.status}`)
  return response.json()
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
