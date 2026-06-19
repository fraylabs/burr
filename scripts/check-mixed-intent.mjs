#!/usr/bin/env node
import fs from "node:fs"
import { spawnSync } from "node:child_process"

const python = splitCommand(process.env.PYTHON ?? "uv run --package burr-build123d python")

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

run(python[0], [...python.slice(1), "examples/build123d-mixed-intent/design.py"])

const check = run("cargo", ["run", "--quiet", "--", "check", "examples/build123d-mixed-intent"])
if (!check.output.includes("PASS examples/build123d-mixed-intent/burr-design-data.json")) {
  throw new Error(`Mixed-intent fixture did not pass.\n${check.output}`)
}

const receipt = JSON.parse(fs.readFileSync("examples/build123d-mixed-intent/burr-receipt.json", "utf8"))
const summary = receipt.summary.features

expectEqual(summary.declared, 4, "declared feature count")
expectEqual(summary.checked, 1, "checked feature count")
expectEqual(summary.unchecked, 3, "unchecked feature count")
expectIncludes(summary.checked_feature_ids, "m3_mount", "checked feature ids")
expectIncludes(summary.unchecked_feature_ids, "lightening_hole", "unchecked feature ids")
expectIncludes(summary.unchecked_feature_ids, "air_passage", "unchecked feature ids")
expectIncludes(summary.unchecked_feature_ids, "cosmetic_dot", "unchecked feature ids")
expectEqual(summary.intent_counts.mechanical_interface, 1, "mechanical intent count")
expectEqual(summary.intent_counts.weight_reduction, 1, "weight intent count")
expectEqual(summary.intent_counts.fluid_or_air_path, 1, "fluid intent count")
expectEqual(summary.intent_counts.cosmetic, 1, "cosmetic intent count")

if (summary.step_candidate_cylinders_considered < 4) {
  throw new Error(
    `Expected at least 4 STEP candidate cylinders, got ${summary.step_candidate_cylinders_considered}`,
  )
}

const checkedFeatureIds = new Set(
  receipt.checks
    .map((check) => check.feature_id)
    .filter((featureId) => typeof featureId === "string"),
)
for (const incidental of ["lightening_hole", "air_passage", "cosmetic_dot"]) {
  if (checkedFeatureIds.has(incidental)) {
    throw new Error(`${incidental} was linted even though it is not mechanical_interface intent`)
  }
}

console.log("mixed-intent example passed")

function expectEqual(actual, expected, label) {
  if (actual !== expected) {
    throw new Error(`Unexpected ${label}: got ${actual}, expected ${expected}`)
  }
}

function expectIncludes(values, expected, label) {
  if (!Array.isArray(values) || !values.includes(expected)) {
    throw new Error(`Expected ${label} to include ${expected}; got ${JSON.stringify(values)}`)
  }
}
