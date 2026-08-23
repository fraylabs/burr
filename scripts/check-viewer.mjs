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
const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), "burr-viewer-check-"))
const modelDirectory = path.join(tempRoot, "models", "enclosure")
const modelPath = path.join(modelDirectory, "counterbore.step")
const ignoredDirectory = path.join(tempRoot, "notes")
const unconfiguredModelDirectory = path.join(tempRoot, "archive")
const port = await availablePort()
const baseUrl = `http://127.0.0.1:${port}`

fs.mkdirSync(modelDirectory, { recursive: true })
fs.mkdirSync(ignoredDirectory, { recursive: true })
fs.mkdirSync(unconfiguredModelDirectory, { recursive: true })
fs.mkdirSync(path.join(tempRoot, ".burr"), { recursive: true })
fs.copyFileSync(fixture, modelPath)
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
  expectEqual("packs" in project, false, "project state has no premature pack contract")

  const initialTree = await getJson("/api/tree")
  expectEqual(initialTree.files?.length, 1, "filtered model count")
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

  console.log(
    `viewer proof passed (simple model scope enforced, dark/light Look HTML rendered, watcher refreshed, traversal rejected)`,
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
