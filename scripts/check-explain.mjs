#!/usr/bin/env node
import { spawnSync } from "node:child_process"
import fs from "node:fs"
import os from "node:os"
import path from "node:path"

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

run("cargo", ["run", "--quiet", "--", "check", "examples/linear-actuator-bad"], {
  expectFailure: true,
})
const edgeDistance = run("cargo", [
  "run",
  "--quiet",
  "--",
  "explain",
  "examples/linear-actuator-bad",
])
expectIncludes(edgeDistance.output, "EXPLAIN examples/linear-actuator-bad/burr-receipt.json")
expectIncludes(edgeDistance.output, "Feature: m3_lower_left")
expectIncludes(edgeDistance.output, "Category: unsafe dimension")
expectIncludes(edgeDistance.output, "Problem: the loaded M3 hole is too close to a free edge.")
expectIncludes(edgeDistance.output, "Why it matters: thin edge material can crack")
expectIncludes(edgeDistance.output, "Fix: move the hole inward or make the surrounding part larger.")

for (const fixture of ["bad", "good"]) {
  run("uv", [
    "run",
    "--package",
    "burr-build123d",
    "python",
    `examples/build123d-bearing-seat/${fixture}/design.py`,
  ])
}
run("cargo", ["run", "--quiet", "--", "check", "examples/build123d-bearing-seat/bad"], {
  expectFailure: true,
})
const missingFeature = run("cargo", [
  "run",
  "--quiet",
  "--",
  "explain",
  "examples/build123d-bearing-seat/bad",
])
expectIncludes(missingFeature.output, "Feature: bearing_608_seat")
expectIncludes(missingFeature.output, "Fix geometry: regenerate the missing bearing seat.")
expectIncludes(missingFeature.output, "Category: missing geometry")
expectIncludes(
  missingFeature.output,
  "Problem: the design data declares a bearing seat, but Burr cannot find matching STEP geometry.",
)
expectIncludes(missingFeature.output, "Evidence: matched seat cylinder = true.")
expectIncludes(missingFeature.output, "Evidence: matched bearing shoulder plane = false.")
expectIncludes(
  missingFeature.output,
  "Fix: regenerate the STEP from the bearing_seat helper or update the declared seat center/diameter/depth.",
)

run("cargo", ["run", "--quiet", "--", "check", "examples/build123d-bearing-seat/good"])
const passing = run("cargo", [
  "run",
  "--quiet",
  "--",
  "explain",
  "examples/build123d-bearing-seat/good",
])
expectIncludes(passing.output, "Status: pass")
expectIncludes(passing.output, "No failed checks in this receipt.")

const tmp = fs.mkdtempSync(path.join(os.tmpdir(), "burr-explain-triage-"))
try {
  const messyReceipt = path.join(tmp, "burr-receipt.json")
  fs.writeFileSync(
    messyReceipt,
    JSON.stringify(
      {
        schema_version: "burr.receipt.v1",
        status: "fail",
        source_design_data: "burr-design-data.json",
        checks: [
          {
            rule_id: "actuator_mount:m3_loaded_hole_edge_distance",
            status: "fail",
            reason: "insufficient_edge_distance",
            feature_id: "m3_lower_left",
            measured: { center_to_edge_mm: 4.0 },
            required: { center_to_edge_mm: 10.2 },
            margin_mm: -6.2,
            message: "Hole edge distance is short by 6.2 mm.",
          },
          {
            rule_id: "freshness:source_hash",
            status: "fail",
            reason: "source_hash_mismatch",
            feature_id: null,
            path: "design.py",
            message: "Source hash does not match design data.",
          },
          {
            rule_id: "bearing_seat:presence",
            status: "fail",
            reason: "missing_declared_feature",
            feature_id: "bearing_608_seat",
            measured: {
              artifact_path: "bearing-seat.step",
              matched_seat_cylinder: true,
              matched_seat_shoulder_plane: false,
            },
            message: "Declared bearing seat is missing from STEP.",
          },
        ],
      },
      null,
      2,
    ) + "\n",
  )
  const triage = run("cargo", ["run", "--quiet", "--", "explain", messyReceipt])
  expectOrder(triage.output, [
    "1. Fix first: regenerate or restamp stale CAD artifacts.",
    "Category: stale artifact",
    "2. Fix geometry: regenerate the missing bearing seat.",
    "Category: missing geometry",
    "3. Fix dimension: move or resize unsafe geometry.",
    "Category: unsafe dimension",
  ])
} finally {
  fs.rmSync(tmp, { recursive: true, force: true })
}

console.log("explain proof passed")

function expectIncludes(output, expected) {
  if (!output.includes(expected)) {
    throw new Error(`Expected output to include ${JSON.stringify(expected)}.\n${output}`)
  }
}

function expectOrder(output, expectedParts) {
  let cursor = 0
  for (const expected of expectedParts) {
    const index = output.indexOf(expected, cursor)
    if (index < 0) {
      throw new Error(`Expected output to include ${JSON.stringify(expected)} after offset ${cursor}.\n${output}`)
    }
    cursor = index + expected.length
  }
}
