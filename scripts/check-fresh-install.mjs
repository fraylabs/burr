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
  if (!options.allowFailure && result.status !== 0) {
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
  if (!pyproject.includes("burr-build123d==0.9.0")) {
    throw new Error("Starter pyproject does not pin burr-build123d==0.9.0")
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

  const designPath = path.join(partDir, "design.py")
  const goodDesign = fs.readFileSync(designPath, "utf8")
  const badDesign = goodDesign.replace("m3_hole_y = 12.0", "m3_hole_y = 20.0")
  if (badDesign === goodDesign) {
    throw new Error("Could not patch starter design into failing edge-distance case.")
  }

  fs.writeFileSync(designPath, badDesign)
  run("uv", ["run", "python", "design.py"], { cwd: partDir })

  const badCheck = run("burr", ["check", "."], { allowFailure: true, cwd: partDir })
  if (badCheck.status === 0) {
    throw new Error("Expected bad starter design to fail Burr check.")
  }
  if (!badCheck.output.includes("FAIL burr-design-data.json -> burr-receipt.json")) {
    throw new Error(`Unexpected failing Burr check output.\n${badCheck.output}`)
  }

  const badReceipt = JSON.parse(fs.readFileSync(path.join(partDir, "burr-receipt.json"), "utf8"))
  const hasEdgeDistanceFailure = badReceipt.checks?.some(
    (check) => check.status === "fail" && check.reason === "insufficient_edge_distance"
  )
  if (!hasEdgeDistanceFailure) {
    throw new Error("Bad starter receipt did not fail on insufficient_edge_distance.")
  }

  const badExplain = run("burr", ["explain", "."], { cwd: partDir }).output
  for (const expected of [
    "Status: fail",
    "Feature: m3_lower_left",
    "Rule: actuator_mount:m3_loaded_hole_edge_distance",
    "Problem: the loaded M3 hole is too close to a free edge.",
    "Evidence: Measured center-to-edge = 4 mm.",
    "Evidence: Required center-to-edge = 10.2 mm.",
    "Fix: move the hole inward or make the surrounding part larger.",
  ]) {
    if (!badExplain.includes(expected)) {
      throw new Error(`Fresh install explain output missing ${expected}.\n${badExplain}`)
    }
  }

  fs.writeFileSync(designPath, goodDesign)
  run("uv", ["run", "python", "design.py"], { cwd: partDir })
  const fixedCheck = run("burr", ["check", "."], { cwd: partDir })
  if (!fixedCheck.output.includes("PASS burr-design-data.json -> burr-receipt.json")) {
    throw new Error(`Fixed starter design did not pass Burr check.\n${fixedCheck.output}`)
  }

  for (const file of ["design.py", "actuator.step", "burr-design-data.json", "burr-receipt.json"]) {
    const target = path.join(partDir, file)
    if (!fs.existsSync(target)) {
      throw new Error(`Fresh install smoke test did not create ${file}`)
    }
  }

  console.log(`fresh install and failure-to-fix proof passed with burr ${burrVersion}`)
} finally {
  fs.rmSync(tmp, { recursive: true, force: true })
}
