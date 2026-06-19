#!/usr/bin/env node
import { spawnSync } from "node:child_process"

const python = splitCommand(process.env.PYTHON ?? "python")

function splitCommand(command) {
  return command.trim().split(/\s+/).filter(Boolean)
}

function run(command, args, options = {}) {
  const result = spawnSync(command, args, {
    encoding: "utf8",
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

function runPython(args) {
  return run(python[0], [...python.slice(1), ...args])
}

runPython(["examples/build123d-actuator/bad/design.py"])
runPython(["examples/build123d-actuator/good/design.py"])
runPython(["examples/build123d-wall-thickness/bad/design.py"])
runPython(["examples/build123d-wall-thickness/good/design.py"])
runPython(["examples/build123d-step-presence/bad/design.py"])
runPython(["examples/build123d-step-presence/good/design.py"])

const bad = run(
  "cargo",
  ["run", "--quiet", "--", "check", "examples/build123d-actuator/bad", "--no-write-receipt"],
  { expectFailure: true },
)
if (!bad.output.includes("Short by: 2.2 mm")) {
  throw new Error(`Bad build123d fixture did not print the expected fix hint.\n${bad.output}`)
}

const good = run(
  "cargo",
  ["run", "--quiet", "--", "check", "examples/build123d-actuator/good", "--no-write-receipt"],
)
if (!good.output.includes("PASS examples/build123d-actuator/good/burr-design-data.json")) {
  throw new Error(`Good build123d fixture did not pass as expected.\n${good.output}`)
}

const wallBad = run(
  "cargo",
  ["run", "--quiet", "--", "check", "examples/build123d-wall-thickness/bad", "--no-write-receipt"],
  { expectFailure: true },
)
if (!wallBad.output.includes("Measured wall thickness: 1.2 mm")) {
  throw new Error(`Bad wall-thickness fixture did not print measured wall thickness.\n${wallBad.output}`)
}
if (!wallBad.output.includes("Short by: 0.8 mm")) {
  throw new Error(`Bad wall-thickness fixture did not print expected shortage.\n${wallBad.output}`)
}

const wallGood = run(
  "cargo",
  ["run", "--quiet", "--", "check", "examples/build123d-wall-thickness/good", "--no-write-receipt"],
)
if (!wallGood.output.includes("PASS examples/build123d-wall-thickness/good/burr-design-data.json")) {
  throw new Error(`Good wall-thickness fixture did not pass as expected.\n${wallGood.output}`)
}

const presenceBad = run(
  "cargo",
  ["run", "--quiet", "--", "check", "examples/build123d-step-presence/bad", "--no-write-receipt"],
  { expectFailure: true },
)
if (!presenceBad.output.includes("Declared clearance hole m3_claimed is missing from the STEP artifact.")) {
  throw new Error(`Bad STEP-presence fixture did not print missing feature diagnostic.\n${presenceBad.output}`)
}
if (!presenceBad.output.includes("Candidate cylinders found: 0")) {
  throw new Error(`Bad STEP-presence fixture did not report zero candidate cylinders.\n${presenceBad.output}`)
}

const presenceGood = run(
  "cargo",
  ["run", "--quiet", "--", "check", "examples/build123d-step-presence/good", "--no-write-receipt"],
)
if (!presenceGood.output.includes("PASS examples/build123d-step-presence/good/burr-design-data.json")) {
  throw new Error(`Good STEP-presence fixture did not pass as expected.\n${presenceGood.output}`)
}

console.log("build123d examples passed")
