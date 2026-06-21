#!/usr/bin/env node
import fs from "node:fs"
import os from "node:os"
import path from "node:path"
import { spawnSync } from "node:child_process"

const packageJson = JSON.parse(fs.readFileSync("package.json", "utf8"))
const burrVersion = packageJson.version
const tmp = fs.mkdtempSync(path.join(os.tmpdir(), "burr-fresh-install-"))
const toolRoot = path.join(tmp, "tools")
const cargoHome = path.join(tmp, "cargo-home")
const projectRoot = path.join(tmp, "project")
const partDir = path.join(projectRoot, "my-part")

function run(command, args, options = {}) {
  const result = spawnSync(command, args, {
    cwd: options.cwd ?? process.cwd(),
    env: {
      ...process.env,
      CARGO_HOME: cargoHome,
      PATH: `${path.join(toolRoot, "bin")}:${process.env.PATH}`,
      UV_NO_CACHE: "1",
      ...(options.env ?? {}),
    },
    encoding: "utf8",
    maxBuffer: 1024 * 1024 * 32,
  })
  const output = [result.stdout, result.stderr].filter(Boolean).join("\n")
  if (result.status !== 0) {
    throw new Error(`${command} ${args.join(" ")} failed with exit ${result.status}\n${output}`)
  }
  return { ...result, output }
}

try {
  fs.mkdirSync(projectRoot, { recursive: true })

  run("cargo", ["install", "burr", "--version", burrVersion, "--root", toolRoot])

  const version = run("burr", ["--version"]).output.trim()
  if (version !== burrVersion) {
    throw new Error(`Expected burr ${burrVersion}, got ${version}`)
  }

  run("burr", ["init", "my-part"], { cwd: projectRoot })

  const pyproject = fs.readFileSync(path.join(partDir, "pyproject.toml"), "utf8")
  if (!pyproject.includes("burr-build123d==0.7.0")) {
    throw new Error("Starter pyproject does not pin burr-build123d==0.7.0")
  }

  run("uv", ["run", "python", "design.py"], { cwd: partDir })
  run("burr", ["check", "."], { cwd: partDir })
  const explain = run("burr", ["explain", "."], { cwd: partDir }).output
  if (!explain.includes("Status: pass")) {
    throw new Error(`Fresh install explain output did not report pass.\n${explain}`)
  }
  if (!explain.includes("No failed checks in this receipt.")) {
    throw new Error(`Fresh install explain output did not report no failures.\n${explain}`)
  }

  for (const file of ["design.py", "actuator.step", "burr-design-data.json", "burr-receipt.json"]) {
    const target = path.join(partDir, file)
    if (!fs.existsSync(target)) {
      throw new Error(`Fresh install smoke test did not create ${file}`)
    }
  }

  console.log(`fresh install proof passed with burr ${burrVersion}`)
} finally {
  fs.rmSync(tmp, { recursive: true, force: true })
}

