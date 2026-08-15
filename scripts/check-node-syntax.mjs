#!/usr/bin/env node
import fs from "node:fs"
import path from "node:path"
import { spawnSync } from "node:child_process"
import { fileURLToPath } from "node:url"

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..")
const scriptsDir = path.join(repoRoot, "scripts")
const scripts = fs
  .readdirSync(scriptsDir)
  .filter((file) => file.endsWith(".mjs"))
  .sort()

for (const script of scripts) {
  const scriptPath = path.join(scriptsDir, script)
  const result = spawnSync(process.execPath, ["--check", scriptPath], {
    cwd: repoRoot,
    stdio: "inherit",
  })
  if (result.status !== 0) {
    throw new Error(`Node syntax check failed for scripts/${script}`)
  }
}

console.log(`node syntax checks passed for ${scripts.length} scripts`)
