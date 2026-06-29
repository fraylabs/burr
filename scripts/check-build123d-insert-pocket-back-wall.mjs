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
    `examples/build123d-insert-pocket-back-wall/${fixture}/design.py`,
  ])
}

const bad = run(
  "cargo",
  ["run", "--quiet", "--", "check", "examples/build123d-insert-pocket-back-wall/bad"],
  { expectFailure: true },
)
if (!bad.output.includes("Blind pocket m3_insert_pocket leaves too little back wall.")) {
  throw new Error(`Bad insert-pocket back-wall fixture did not report thin back wall.\n${bad.output}`)
}
if (!bad.output.includes("Measured back wall: 1 mm")) {
  throw new Error(`Bad insert-pocket back-wall fixture did not report measured back wall.\n${bad.output}`)
}
if (!bad.output.includes("Short by: 1 mm")) {
  throw new Error(`Bad insert-pocket back-wall fixture did not report expected shortage.\n${bad.output}`)
}

const good = run("cargo", ["run", "--quiet", "--", "check", "examples/build123d-insert-pocket-back-wall/good"])
if (!good.output.includes("PASS examples/build123d-insert-pocket-back-wall/good/burr-design-data.json")) {
  throw new Error(`Good insert-pocket back-wall fixture did not pass.\n${good.output}`)
}

checkReceipt("examples/build123d-insert-pocket-back-wall/good/burr-receipt.json", {
  status: "pass",
  backWallStatus: "pass",
  backWallReason: "ok",
  backWallThickness: 3,
  margin: 1,
})
checkReceipt("examples/build123d-insert-pocket-back-wall/bad/burr-receipt.json", {
  status: "fail",
  backWallStatus: "fail",
  backWallReason: "insufficient_blind_pocket_back_wall",
  backWallThickness: 1,
  margin: -1,
})

console.log("build123d insert-pocket back-wall proof passed")

function checkReceipt(path, expected) {
  const receipt = JSON.parse(fs.readFileSync(path, "utf8"))
  expectEqual(receipt.status, expected.status, `${path} receipt status`)
  expectEqual(receipt.rulepack_version, "0.14.0", `${path} rulepack version`)

  const summary = receipt.summary.features
  expectEqual(summary.declared, 1, `${path} declared feature count`)
  expectEqual(summary.checked, 1, `${path} checked feature count`)
  expectEqual(summary.unchecked, 0, `${path} unchecked feature count`)
  expectIncludes(summary.checked_feature_ids, "m3_insert_pocket", `${path} checked feature ids`)

  const presenceCheck = receipt.checks.find(
    (check) =>
      check.rule_id === "actuator_mount:heat_set_insert_pocket_step_presence" &&
      check.feature_id === "m3_insert_pocket",
  )
  if (!presenceCheck) {
    throw new Error(`${path} is missing the heat-set insert pocket STEP-presence check`)
  }
  expectEqual(presenceCheck.status, "pass", `${path} presence status`)
  expectEqual(presenceCheck.reason, "ok", `${path} presence reason`)
  expectEqual(presenceCheck.measured.matched_pocket_cylinder, true, `${path} matched pocket cylinder`)
  expectEqual(presenceCheck.measured.matched_pocket_bottom_plane, true, `${path} matched pocket bottom plane`)

  const backWallCheck = receipt.checks.find(
    (check) =>
      check.rule_id === "actuator_mount:m3_insert_pocket_back_wall_thickness" &&
      check.feature_id === "m3_insert_pocket",
  )
  if (!backWallCheck) {
    throw new Error(`${path} is missing the insert-pocket back-wall check`)
  }
  expectEqual(backWallCheck.status, expected.backWallStatus, `${path} back-wall status`)
  expectEqual(backWallCheck.reason, expected.backWallReason, `${path} back-wall reason`)
  expectEqual(
    backWallCheck.measured.back_wall_thickness_mm,
    expected.backWallThickness,
    `${path} back-wall thickness`,
  )
  expectEqual(backWallCheck.measured.back_face.axis, "x", `${path} back-wall axis`)
  expectEqual(backWallCheck.measured.back_face.side, "max", `${path} back-wall side`)
  expectEqual(
    backWallCheck.required.min_back_wall_thickness_mm,
    2,
    `${path} required back-wall thickness`,
  )
  expectEqual(backWallCheck.margin_mm, expected.margin, `${path} back-wall margin`)
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
