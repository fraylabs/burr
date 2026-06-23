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

const edgeDistancePacket = run("cargo", [
  "run",
  "--quiet",
  "--",
  "explain",
  "--json",
  "examples/linear-actuator-bad",
])
const edgeDistanceJson = JSON.parse(edgeDistancePacket.output)
expectEqual(edgeDistanceJson.schema_version, "burr.repair-packet.v1", "receipt packet schema")
expectEqual(edgeDistanceJson.source_kind, "receipt", "receipt packet source kind")
expectEqual(edgeDistanceJson.status, "fail", "receipt packet status")
expectEqual(edgeDistanceJson.summary.exact_source_edits_available, false, "receipt exact edit availability")
const edgeDistanceFailure = edgeDistanceJson.failures.find(
  (failure) => failure.feature_id === "m3_lower_left",
)
if (!edgeDistanceFailure) {
  throw new Error(`Missing m3_lower_left failure in JSON packet.\n${edgeDistancePacket.output}`)
}
expectEqual(edgeDistanceFailure.category, "unsafe dimension", "receipt packet category")
expectEqual(
  edgeDistanceFailure.fix,
  "move the hole inward or make the surrounding part larger.",
  "receipt packet fix",
)

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

  const repairReport = path.join(tmp, "repair-report.json")
  fs.writeFileSync(
    repairReport,
    JSON.stringify(
      {
        schema_version: "burr.repair-report.v1",
        id: "demo-repair",
        report_id: "demo-repair",
        status: "pass",
        focus_rule_id: "actuator_mount:m3_loaded_hole_edge_distance",
        first_fix: "Move the loaded M3 hole inward.",
        summary: { before_failures: 1 },
        failures: [
          {
            feature_id: "m3_lower_left",
            rule_id: "actuator_mount:m3_loaded_hole_edge_distance",
            reason: "insufficient_edge_distance",
          },
        ],
        repair_actions: [
          {
            feature_id: "m3_lower_left",
            action: "move_feature",
            parameter: "center_mm",
            rule_id: "actuator_mount:m3_loaded_hole_edge_distance",
            suggested_delta_mm: [4, 0, 0],
            source_hint: {
              source_file_path: "source.py",
              edit_kind: "replace_python_assignment",
              selector: "m3_lower_left_center",
              value_path: "features[id=m3_lower_left].center_mm",
              before_text: "m3_lower_left_center = (-28.0, -8.0, 0.0)",
              after_text: "m3_lower_left_center = (-24.0, -8.0, 0.0)",
              confidence: "exact_from_design_data",
            },
          },
        ],
      },
      null,
      2,
    ) + "\n",
  )
  const repairPacket = run("cargo", ["run", "--quiet", "--", "explain", "--json", repairReport])
  const repairJson = JSON.parse(repairPacket.output)
  expectEqual(repairJson.schema_version, "burr.repair-packet.v1", "repair packet schema")
  expectEqual(repairJson.source_kind, "repair_report", "repair packet source kind")
  expectEqual(repairJson.summary.exact_source_edits_available, true, "repair packet exact edit availability")
  expectEqual(repairJson.summary.exact_source_edit_count, 1, "repair packet exact edit count")
  expectEqual(
    repairJson.repair_actions[0].source_hint.before_text,
    "m3_lower_left_center = (-28.0, -8.0, 0.0)",
    "repair packet before_text",
  )
  expectEqual(
    repairJson.repair_actions[0].source_hint.after_text,
    "m3_lower_left_center = (-24.0, -8.0, 0.0)",
    "repair packet after_text",
  )
} finally {
  fs.rmSync(tmp, { recursive: true, force: true })
}

console.log("explain proof passed")

function expectIncludes(output, expected) {
  if (!output.includes(expected)) {
    throw new Error(`Expected output to include ${JSON.stringify(expected)}.\n${output}`)
  }
}

function expectEqual(actual, expected, label) {
  if (actual !== expected) {
    throw new Error(`Unexpected ${label}: got ${JSON.stringify(actual)}, expected ${JSON.stringify(expected)}`)
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
