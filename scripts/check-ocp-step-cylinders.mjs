#!/usr/bin/env node
import { spawnSync } from "node:child_process"

function run(command, args, options = {}) {
  const result = spawnSync(command, args, {
    encoding: "utf8",
    env: { ...process.env, ...(options.env ?? {}) },
    maxBuffer: 1024 * 1024 * 32,
  })
  const output = [result.stdout, result.stderr].filter(Boolean).join("\n")
  if (options.expectFailure) {
    if (result.status === 0) {
      throw new Error(`${command} ${args.join(" ")} unexpectedly passed\n${output}`)
    }
  } else if (result.status !== 0) {
    throw new Error(`${command} ${args.join(" ")} failed with exit ${result.status}\n${output}`)
  }
  return { ...result, output }
}

run("uv", ["run", "--package", "burr-build123d", "python", "examples/build123d-step-presence/bad/design.py"])
run("uv", ["run", "--package", "burr-build123d", "python", "examples/build123d-step-presence/good/design.py"])

const extractor = run("uv", [
  "run",
  "--package",
  "burr-ocp",
  "burr-ocp-step-cylinders",
  "examples/build123d-step-presence/good/presence.step",
])
const data = JSON.parse(extractor.stdout)
if (data.schema_version !== "burr.ocp-step-cylinders.v1") {
  throw new Error(`Unexpected OCP extractor schema.\n${extractor.stdout}`)
}
if (data.cylinders.length !== 1 || data.cylinders[0].radius_mm !== 1.7) {
  throw new Error(`Unexpected OCP cylinder extraction.\n${extractor.stdout}`)
}
if (!Array.isArray(data.planes) || data.planes.length < 2) {
  throw new Error(`Unexpected OCP plane extraction.\n${extractor.stdout}`)
}

const env = {
  BURR_STEP_CYLINDER_BACKEND: "ocp",
  BURR_OCP_STEP_CYLINDERS: "uv run --package burr-ocp burr-ocp-step-cylinders",
}

const bad = run(
  "cargo",
  ["run", "--quiet", "--", "check", "examples/build123d-step-presence/bad", "--no-write-receipt"],
  { env, expectFailure: true },
)
if (!bad.output.includes("Declared clearance hole m3_claimed is missing from the STEP artifact.")) {
  throw new Error(`OCP backend did not fail the missing-hole fixture correctly.\n${bad.output}`)
}

const good = run(
  "cargo",
  ["run", "--quiet", "--", "check", "examples/build123d-step-presence/good", "--no-write-receipt"],
  { env },
)
if (!good.output.includes("PASS examples/build123d-step-presence/good/burr-design-data.json")) {
  throw new Error(`OCP backend did not pass the good fixture.\n${good.output}`)
}

console.log("OCP STEP cylinder backend passed")
