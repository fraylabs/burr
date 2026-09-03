#!/usr/bin/env node

import fs from "node:fs"
import net from "node:net"
import os from "node:os"
import path from "node:path"
import { spawn } from "node:child_process"
import { performance } from "node:perf_hooks"
import { fileURLToPath } from "node:url"

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..")
const burr = path.join(repoRoot, "target", "release", process.platform === "win32" ? "burr.exe" : "burr")
const projects = process.argv.slice(2).map((project) => path.resolve(project))

if (projects.length === 0) {
  throw new Error(
    "provide one or more extracted Burr outcome projects; first run `cargo build --release --locked`",
  )
}
if (!fs.existsSync(burr)) {
  throw new Error(`release binary not found at ${burr}; run cargo build --release --locked`)
}
for (const project of projects) {
  if (!fs.statSync(project).isDirectory()) throw new Error(`not a project directory: ${project}`)
}

const results = []
for (const project of projects) {
  results.push(await measureProject(project))
}

console.log(
  JSON.stringify(
    {
      schema_version: "burr.outcome-performance.v1",
      burr_version: await binaryVersion(),
      measured_at: new Date().toISOString(),
      platform: `${process.platform}-${process.arch}`,
      results,
    },
    null,
    2,
  ),
)

async function measureProject(project) {
  const cacheRoot = fs.mkdtempSync(path.join(os.tmpdir(), "burr-outcome-cache-"))
  let server = null
  let replay = null
  try {
    server = await startServer(project, cacheRoot)
    const selection = await selectRepresentativeModel(server.baseUrl)
    const cold = await fetchViewer(server.baseUrl, selection, "cold")
    const warm = await fetchViewer(server.baseUrl, selection, "warm")
    if (cold.cache !== "generated") {
      throw new Error(`${path.basename(project)} cold load expected generation, got ${cold.cache}`)
    }
    if (warm.cache !== "memory") {
      throw new Error(`${path.basename(project)} warm load expected memory cache, got ${warm.cache}`)
    }
    await stopServer(server)
    server = null

    replay = await startServer(project, cacheRoot)
    const restart = await fetchViewer(replay.baseUrl, selection, "restart")
    if (restart.cache !== "disk") {
      throw new Error(`${path.basename(project)} restart expected disk cache, got ${restart.cache}`)
    }

    return {
      project: path.basename(project),
      selection,
      cold,
      warm,
      restart,
    }
  } finally {
    if (server) await stopServer(server)
    if (replay) await stopServer(replay)
    fs.rmSync(cacheRoot, { recursive: true, force: true })
  }
}

async function selectRepresentativeModel(baseUrl) {
  const project = await getJson(baseUrl, "/api/project")
  const motion = project.motions?.[0]
  if (motion) return { path: motion.from, motion: motion.id }
  const tree = await getJson(baseUrl, "/api/tree")
  const model = tree.files?.[0]
  if (!model) throw new Error("outcome project contains no supported models")
  return { path: model.path, motion: null }
}

async function fetchViewer(baseUrl, selection, phase) {
  const loadId = `${phase}-${Date.now()}-${Math.floor(Math.random() * 1_000_000)}`
  const query = new URLSearchParams({ path: selection.path, load: loadId })
  if (selection.motion) query.set("motion", selection.motion)
  const started = performance.now()
  const response = await fetch(`${baseUrl}/viewer?${query}`, { cache: "no-store" })
  const bytes = (await response.arrayBuffer()).byteLength
  const elapsedMs = performance.now() - started
  if (!response.ok) throw new Error(`viewer returned ${response.status} for ${selection.path}`)
  const status = await getJson(baseUrl, `/api/load-status?id=${encodeURIComponent(loadId)}`)
  if (status.state !== "ready") throw new Error(`viewer did not reach ready: ${status.state}`)
  return {
    milliseconds: Math.round(elapsedMs * 100) / 100,
    bytes,
    cache: status.cache,
  }
}

async function startServer(project, cacheRoot) {
  const port = await availablePort()
  const baseUrl = `http://127.0.0.1:${port}`
  const child = spawn(burr, ["."], {
    cwd: project,
    env: {
      ...process.env,
      BURR_CACHE_DIR: cacheRoot,
      BURR_VIEWER_NO_OPEN: "1",
      BURR_VIEWER_PORT: String(port),
    },
    stdio: ["ignore", "pipe", "pipe"],
  })
  let stderrTail = ""
  child.stdout.resume()
  child.stderr.on("data", (chunk) => {
    stderrTail = `${stderrTail}${chunk}`.slice(-8_000)
  })
  const closed = new Promise((resolve) => child.once("close", resolve))
  await waitForServer(child, baseUrl, () => stderrTail)
  return { child, closed, baseUrl }
}

async function stopServer(server) {
  if (server.child.exitCode === null) server.child.kill("SIGTERM")
  await Promise.race([server.closed, delay(2_000)])
}

async function waitForServer(child, baseUrl, stderrTail) {
  for (let attempt = 0; attempt < 120; attempt += 1) {
    if (child.exitCode !== null) {
      throw new Error(`Burr exited before startup with code ${child.exitCode}\n${stderrTail()}`)
    }
    try {
      const response = await fetch(`${baseUrl}/api/health`)
      if (response.ok) return
    } catch {
      // Burr has not bound its loopback port yet.
    }
    await delay(250)
  }
  throw new Error(`Burr did not start within 30 seconds\n${stderrTail()}`)
}

async function binaryVersion() {
  return new Promise((resolve, reject) => {
    const child = spawn(burr, ["--version"], { stdio: ["ignore", "pipe", "pipe"] })
    let stdout = ""
    let stderr = ""
    child.stdout.on("data", (chunk) => {
      stdout += chunk
    })
    child.stderr.on("data", (chunk) => {
      stderr += chunk
    })
    child.once("close", (code) => {
      if (code === 0) resolve(stdout.trim())
      else reject(new Error(`burr --version failed with code ${code}: ${stderr}`))
    })
  })
}

async function availablePort() {
  return new Promise((resolve, reject) => {
    const server = net.createServer()
    server.once("error", reject)
    server.listen(0, "127.0.0.1", () => {
      const address = server.address()
      const port = typeof address === "object" && address ? address.port : null
      server.close((error) => {
        if (error) reject(error)
        else if (port === null) reject(new Error("could not allocate a test port"))
        else resolve(port)
      })
    })
  })
}

async function getJson(baseUrl, route) {
  const response = await fetch(`${baseUrl}${route}`, { cache: "no-store" })
  if (!response.ok) throw new Error(`${route} returned ${response.status}`)
  return response.json()
}

function delay(milliseconds) {
  return new Promise((resolve) => setTimeout(resolve, milliseconds))
}
