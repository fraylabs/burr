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
    `examples/build123d-bearing-seat-edge-distance/${fixture}/design.py`,
  ])
}

const bad = run(
  "cargo",
  ["run", "--quiet", "--", "check", "examples/build123d-bearing-seat-edge-distance/bad"],
  { expectFailure: true },
)
if (!bad.output.includes("Feature bearing_608_seat is too close to the edge.")) {
  throw new Error(`Bad bearing-seat edge fixture did not report edge material.\n${bad.output}`)
}
if (!bad.output.includes("Measured feature-to-edge: 1.5 mm")) {
  throw new Error(`Bad bearing-seat edge fixture did not report measured edge material.\n${bad.output}`)
}
if (!bad.output.includes("Short by: 1.5 mm")) {
  throw new Error(`Bad bearing-seat edge fixture did not report expected shortage.\n${bad.output}`)
}

const good = run("cargo", ["run", "--quiet", "--", "check", "examples/build123d-bearing-seat-edge-distance/good"])
if (!good.output.includes("PASS examples/build123d-bearing-seat-edge-distance/good/burr-design-data.json")) {
  throw new Error(`Good bearing-seat edge fixture did not pass.\n${good.output}`)
}

checkReceipt("examples/build123d-bearing-seat-edge-distance/good/burr-receipt.json", {
  status: "pass",
  edgeStatus: "pass",
  edgeReason: "ok",
  wallToEdge: 7,
  margin: 4,
})
checkReceipt("examples/build123d-bearing-seat-edge-distance/bad/burr-receipt.json", {
  status: "fail",
  edgeStatus: "fail",
  edgeReason: "insufficient_feature_edge_distance",
  wallToEdge: 1.5,
  margin: -1.5,
})

console.log("build123d bearing-seat edge-distance proof passed")

function checkReceipt(path, expected) {
  const receipt = JSON.parse(fs.readFileSync(path, "utf8"))
  expectEqual(receipt.status, expected.status, `${path} receipt status`)
  expectEqual(receipt.rulepack_version, "0.13.0", `${path} rulepack version`)

  const summary = receipt.summary.features
  expectEqual(summary.declared, 1, `${path} declared feature count`)
  expectEqual(summary.checked, 1, `${path} checked feature count`)
  expectEqual(summary.unchecked, 0, `${path} unchecked feature count`)
  expectIncludes(summary.checked_feature_ids, "bearing_608_seat", `${path} checked feature ids`)

  const presenceCheck = receipt.checks.find(
    (check) =>
      check.rule_id === "actuator_mount:bearing_seat_step_presence" &&
      check.feature_id === "bearing_608_seat",
  )
  if (!presenceCheck) {
    throw new Error(`${path} is missing the bearing-seat STEP-presence check`)
  }
  expectEqual(presenceCheck.status, "pass", `${path} presence status`)
  expectEqual(presenceCheck.reason, "ok", `${path} presence reason`)
  expectEqual(presenceCheck.measured.matched_seat_cylinder, true, `${path} matched seat cylinder`)
  expectEqual(
    presenceCheck.measured.matched_seat_shoulder_plane,
    true,
    `${path} matched seat shoulder plane`,
  )

  const edgeCheck = receipt.checks.find(
    (check) =>
      check.rule_id === "actuator_mount:bearing_seat_edge_distance" &&
      check.feature_id === "bearing_608_seat",
  )
  if (!edgeCheck) {
    throw new Error(`${path} is missing the bearing-seat edge-distance check`)
  }
  expectEqual(edgeCheck.status, expected.edgeStatus, `${path} edge status`)
  expectEqual(edgeCheck.reason, expected.edgeReason, `${path} edge reason`)
  expectEqual(edgeCheck.measured.feature_shape, "circle", `${path} edge feature shape`)
  expectEqual(edgeCheck.measured.wall_to_edge_mm, expected.wallToEdge, `${path} wall-to-edge`)
  expectEqual(edgeCheck.required.diameter_field, "seat_diameter_mm", `${path} diameter field`)
  expectEqual(edgeCheck.required.min_wall_to_edge_mm, 3, `${path} required wall-to-edge`)
  expectEqual(edgeCheck.margin_mm, expected.margin, `${path} edge margin`)
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
