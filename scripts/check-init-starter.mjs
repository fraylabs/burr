#!/usr/bin/env node
import fs from "node:fs"
import os from "node:os"
import path from "node:path"
import { spawnSync } from "node:child_process"
import { fileURLToPath } from "node:url"

const __filename = fileURLToPath(import.meta.url)
const repoRoot = path.resolve(path.dirname(__filename), "..")
const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), "burr-init-check-"))
const projectDir = path.join(tempRoot, "starter-part")

function run(command, args, options = {}) {
  const result = spawnSync(command, args, {
    cwd: options.cwd ?? repoRoot,
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
  const init = run("cargo", ["run", "--quiet", "--", "init", projectDir])
  for (const expected of ["INIT", "WRITE", "uv run python design.py", "burr check ."]) {
    if (!init.output.includes(expected)) {
      throw new Error(`burr init output missing ${expected}\n${init.output}`)
    }
  }

  for (const fileName of ["pyproject.toml", "design.py", ".gitignore"]) {
    const filePath = path.join(projectDir, fileName)
    if (!fs.existsSync(filePath)) throw new Error(`starter missing ${fileName}`)
  }

  const generatedPyproject = fs.readFileSync(path.join(projectDir, "pyproject.toml"), "utf8")
  if (!generatedPyproject.includes("burr-build123d==0.9.0")) {
    throw new Error("starter pyproject does not pin burr-build123d==0.9.0")
  }

  run("uv", ["add", "--editable", path.join(repoRoot, "packages", "burr-build123d")], {
    cwd: projectDir,
  })
  run("uv", ["run", "python", "design.py"], { cwd: projectDir })

  for (const fileName of ["actuator.step", "burr-design-data.json"]) {
    const filePath = path.join(projectDir, fileName)
    if (!fs.existsSync(filePath)) throw new Error(`starter did not generate ${fileName}`)
  }

  const check = run("cargo", [
    "run",
    "--quiet",
    "--manifest-path",
    path.join(repoRoot, "Cargo.toml"),
    "--",
    "check",
    projectDir,
    "--no-write-receipt",
  ])
  if (!check.output.includes("PASS")) {
    throw new Error(`starter check did not pass\n${check.output}`)
  }

  console.log("init starter passed")
} finally {
  fs.rmSync(tempRoot, { recursive: true, force: true })
}
