#!/usr/bin/env node
import fs from "node:fs"
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

for (const fixture of ["bad", "good"]) {
  run("uv", [
    "run",
    "--package",
    "burr-build123d",
    "python",
    `examples/build123d-straight-slot/${fixture}/design.py`,
  ])
}

const bad = run("cargo", ["run", "--quiet", "--", "check", "examples/build123d-straight-slot/bad"], {
  expectFailure: true,
})
if (!bad.output.includes("Declared straight slot motor_adjust_slot is missing from the STEP artifact.")) {
  throw new Error(`Bad straight-slot fixture did not report the missing slot.\n${bad.output}`)
}

const good = run("cargo", ["run", "--quiet", "--", "check", "examples/build123d-straight-slot/good"])
if (!good.output.includes("PASS examples/build123d-straight-slot/good/burr-design-data.json")) {
  throw new Error(`Good straight-slot fixture did not pass.\n${good.output}`)
}

checkReceipt("examples/build123d-straight-slot/good/burr-receipt.json", {
  status: "pass",
  slotStatus: "pass",
  slotReason: "ok",
  matchedEndpoints: 2,
  matchedSidePlanes: 2,
})
checkReceipt("examples/build123d-straight-slot/bad/burr-receipt.json", {
  status: "fail",
  slotStatus: "fail",
  slotReason: "missing_declared_feature",
  matchedEndpoints: 0,
  matchedSidePlanes: 0,
})

console.log("build123d straight-slot proof passed")

function checkReceipt(path, expected) {
  const receipt = JSON.parse(fs.readFileSync(path, "utf8"))
  expectEqual(receipt.status, expected.status, `${path} receipt status`)
  expectEqual(receipt.rulepack_version, "0.7.0", `${path} rulepack version`)

  const summary = receipt.summary.features
  expectEqual(summary.declared, 2, `${path} declared feature count`)
  expectEqual(summary.checked, 1, `${path} checked feature count`)
  expectEqual(summary.unchecked, 1, `${path} unchecked feature count`)
  expectIncludes(summary.checked_feature_ids, "motor_adjust_slot", `${path} checked feature ids`)
  expectIncludes(summary.unchecked_feature_ids, "cosmetic_logo_slot", `${path} unchecked feature ids`)
  expectEqual(summary.intent_counts.mechanical_interface, 1, `${path} mechanical intent count`)
  expectEqual(summary.intent_counts.cosmetic, 1, `${path} cosmetic intent count`)

  const slotCheck = receipt.checks.find(
    (check) =>
      check.rule_id === "actuator_mount:straight_slot_step_presence" &&
      check.feature_id === "motor_adjust_slot",
  )
  if (!slotCheck) {
    throw new Error(`${path} is missing the straight-slot check`)
  }
  expectEqual(slotCheck.status, expected.slotStatus, `${path} slot status`)
  expectEqual(slotCheck.reason, expected.slotReason, `${path} slot reason`)
  expectEqual(
    slotCheck.measured.matched_slot_endpoints,
    expected.matchedEndpoints,
    `${path} matched slot endpoints`,
  )
  expectEqual(
    slotCheck.measured.matched_slot_side_planes,
    expected.matchedSidePlanes,
    `${path} matched slot side planes`,
  )
  if (slotCheck.measured.candidate_cylinders < 2) {
    throw new Error(`${path} expected at least two candidate cylinders`)
  }
  if (slotCheck.measured.candidate_planes < 2) {
    throw new Error(`${path} expected at least two candidate planes`)
  }

  const checkedFeatureIds = new Set(
    receipt.checks
      .map((check) => check.feature_id)
      .filter((featureId) => typeof featureId === "string"),
  )
  if (checkedFeatureIds.has("cosmetic_logo_slot")) {
    throw new Error(`${path} linted the cosmetic slot`)
  }
}

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
