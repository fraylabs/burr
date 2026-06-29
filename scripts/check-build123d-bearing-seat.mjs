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
    `examples/build123d-bearing-seat/${fixture}/design.py`,
  ])
}

const bad = run("cargo", ["run", "--quiet", "--", "check", "examples/build123d-bearing-seat/bad"], {
  expectFailure: true,
})
if (!bad.output.includes("Declared bearing seat bearing_608_seat is missing from the STEP artifact.")) {
  throw new Error(`Bad bearing-seat fixture did not report the missing seat.\n${bad.output}`)
}

const good = run("cargo", ["run", "--quiet", "--", "check", "examples/build123d-bearing-seat/good"])
if (!good.output.includes("PASS examples/build123d-bearing-seat/good/burr-design-data.json")) {
  throw new Error(`Good bearing-seat fixture did not pass.\n${good.output}`)
}

checkReceipt("examples/build123d-bearing-seat/good/burr-receipt.json", {
  status: "pass",
  checkStatus: "pass",
  checkReason: "ok",
  matchedSeat: true,
  matchedShoulder: true,
})
checkReceipt("examples/build123d-bearing-seat/bad/burr-receipt.json", {
  status: "fail",
  checkStatus: "fail",
  checkReason: "missing_declared_feature",
  matchedSeat: true,
  matchedShoulder: false,
})

console.log("build123d bearing-seat proof passed")

function checkReceipt(path, expected) {
  const receipt = JSON.parse(fs.readFileSync(path, "utf8"))
  expectEqual(receipt.status, expected.status, `${path} receipt status`)
  expectEqual(receipt.rulepack_version, "0.12.0", `${path} rulepack version`)

  const summary = receipt.summary.features
  expectEqual(summary.declared, 2, `${path} declared feature count`)
  expectEqual(summary.checked, 1, `${path} checked feature count`)
  expectEqual(summary.unchecked, 1, `${path} unchecked feature count`)
  expectIncludes(summary.checked_feature_ids, "bearing_608_seat", `${path} checked feature ids`)
  expectIncludes(summary.unchecked_feature_ids, "cosmetic_bearing_recess", `${path} unchecked feature ids`)
  expectEqual(summary.intent_counts.mechanical_interface, 1, `${path} mechanical intent count`)
  expectEqual(summary.intent_counts.cosmetic, 1, `${path} cosmetic intent count`)

  const seatCheck = receipt.checks.find(
    (check) =>
      check.rule_id === "actuator_mount:bearing_seat_step_presence" &&
      check.feature_id === "bearing_608_seat",
  )
  if (!seatCheck) {
    throw new Error(`${path} is missing the bearing-seat check`)
  }
  expectEqual(seatCheck.status, expected.checkStatus, `${path} seat status`)
  expectEqual(seatCheck.reason, expected.checkReason, `${path} seat reason`)
  expectEqual(
    seatCheck.measured.matched_seat_cylinder,
    expected.matchedSeat,
    `${path} matched seat cylinder`,
  )
  expectEqual(
    seatCheck.measured.matched_seat_shoulder_plane,
    expected.matchedShoulder,
    `${path} matched seat shoulder plane`,
  )
  if (seatCheck.measured.candidate_cylinders < 1) {
    throw new Error(`${path} expected at least one candidate cylinder`)
  }
  if (seatCheck.measured.candidate_planes < 1) {
    throw new Error(`${path} expected at least one candidate plane`)
  }

  const checkedFeatureIds = new Set(
    receipt.checks
      .map((check) => check.feature_id)
      .filter((featureId) => typeof featureId === "string"),
  )
  if (checkedFeatureIds.has("cosmetic_bearing_recess")) {
    throw new Error(`${path} linted the cosmetic bearing seat`)
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
