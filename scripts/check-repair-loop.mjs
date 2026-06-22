#!/usr/bin/env node
import fs from "node:fs"
import os from "node:os"
import path from "node:path"
import { spawnSync } from "node:child_process"

const badDir = "examples/build123d-actuator-housing-repair/bad"
const fixedDir = "examples/build123d-actuator-housing-repair/fixed"
const expectedHoleIds = ["m3_front_left", "m3_front_right", "m3_rear_left", "m3_rear_right"]

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

for (const dir of [badDir, fixedDir]) {
  run("uv", ["run", "--package", "burr-build123d", "python", path.join(dir, "design.py")])
}

const badCheck = run("cargo", ["run", "--quiet", "--", "check", badDir], {
  expectFailure: true,
})
expectIncludes(badCheck.output, "FAIL examples/build123d-actuator-housing-repair/bad/burr-design-data.json")
expectIncludes(badCheck.output, "4 problems:")
for (const featureId of expectedHoleIds) {
  expectIncludes(badCheck.output, `${featureId} is too close to the edge`)
}
expectIncludes(badCheck.output, "Short by: 2.2 mm")

const badReceipt = readReceipt(badDir)
expectEqual(badReceipt.status, "fail", "bad actuator receipt status")
expectEqual(badReceipt.summary.features.declared, 4, "bad actuator declared feature count")
expectEqual(badReceipt.summary.features.checked, 4, "bad actuator checked feature count")
expectArrayEqual(badReceipt.summary.features.checked_feature_ids, expectedHoleIds, "bad checked feature ids")

const failingLoadedHoles = checksForRule(badReceipt, "actuator_mount:m3_loaded_hole_edge_distance")
if (failingLoadedHoles.length !== 4) {
  throw new Error(`Expected four failing actuator mounting holes, got ${failingLoadedHoles.length}`)
}
for (const check of failingLoadedHoles) {
  expectEqual(check.status, "fail", `${check.feature_id} edge-distance status`)
  expectEqual(check.reason, "insufficient_edge_distance", `${check.feature_id} edge-distance reason`)
  expectEqual(check.measured.center_to_edge_mm, 8, `${check.feature_id} measured edge distance`)
  expectEqual(check.required.center_to_edge_mm, 10.2, `${check.feature_id} required edge distance`)
  expectEqual(check.margin_mm, -2.2, `${check.feature_id} edge-distance margin`)
}

const badExplain = run("cargo", [
  "run",
  "--quiet",
  "--",
  "explain",
  path.join(badDir, "burr-receipt.json"),
])
expectOrder(badExplain.output, [
  "4 failed checks:",
  "1. Fix dimension: move or resize unsafe geometry.",
  "Feature: m3_front_left",
  "2. Fix dimension: move or resize unsafe geometry.",
  "Feature: m3_front_right",
  "3. Fix dimension: move or resize unsafe geometry.",
  "Feature: m3_rear_left",
  "4. Fix dimension: move or resize unsafe geometry.",
  "Feature: m3_rear_right",
])
expectIncludes(badExplain.output, "Category: unsafe dimension")
expectIncludes(badExplain.output, "Problem: the loaded M3 hole is too close to a free edge.")
expectIncludes(badExplain.output, "Evidence: Measured center-to-edge = 8 mm.")
expectIncludes(badExplain.output, "Evidence: Required center-to-edge = 10.2 mm.")
expectIncludes(badExplain.output, "Evidence: short by 2.2 mm.")
expectIncludes(badExplain.output, "Fix: move the hole inward or make the surrounding part larger.")

const tmp = fs.mkdtempSync(path.join(os.tmpdir(), "burr-repair-loop-"))
try {
  const staleReceiptPath = path.join(tmp, "stale-actuator-housing-receipt.json")
  fs.writeFileSync(
    staleReceiptPath,
    JSON.stringify(
      {
        ...badReceipt,
        checks: [
          ...badReceipt.checks,
          {
            rule_id: "burr_design_data:source_sha256_matches",
            status: "fail",
            reason: "source_hash_mismatch",
            path: "design.py",
            measured: { sha256: "stale-source-hash" },
            required: { sha256: "fresh-source-hash" },
            message: "Source hash does not match design data.",
          },
        ],
      },
      null,
      2,
    ) + "\n",
  )
  const staleExplain = run("cargo", ["run", "--quiet", "--", "explain", staleReceiptPath])
  expectOrder(staleExplain.output, [
    "5 failed checks:",
    "1. Fix first: regenerate or restamp stale CAD artifacts.",
    "Category: stale artifact",
    "Fix: rerun the CAD generator or burr stamp, then rerun burr check.",
    "2. Fix dimension: move or resize unsafe geometry.",
    "Feature: m3_front_left",
    "Category: unsafe dimension",
  ])
} finally {
  fs.rmSync(tmp, { recursive: true, force: true })
}

const fixedCheck = run("cargo", ["run", "--quiet", "--", "check", fixedDir])
expectIncludes(
  fixedCheck.output,
  "PASS examples/build123d-actuator-housing-repair/fixed/burr-design-data.json",
)

const fixedReceipt = readReceipt(fixedDir)
expectEqual(fixedReceipt.status, "pass", "fixed actuator receipt status")
expectEqual(fixedReceipt.summary.features.declared, 4, "fixed actuator declared feature count")
expectEqual(fixedReceipt.summary.features.checked, 4, "fixed actuator checked feature count")
expectArrayEqual(fixedReceipt.summary.features.checked_feature_ids, expectedHoleIds, "fixed checked feature ids")

const fixedLoadedHoles = checksForRule(fixedReceipt, "actuator_mount:m3_loaded_hole_edge_distance")
if (fixedLoadedHoles.length !== 4) {
  throw new Error(`Expected four checked fixed actuator mounting holes, got ${fixedLoadedHoles.length}`)
}
for (const check of fixedLoadedHoles) {
  expectEqual(check.status, "pass", `${check.feature_id} fixed edge-distance status`)
  expectEqual(check.reason, "ok", `${check.feature_id} fixed edge-distance reason`)
  expectEqual(check.required.center_to_edge_mm, 10.2, `${check.feature_id} fixed required edge distance`)
  if (check.measured.center_to_edge_mm <= check.required.center_to_edge_mm || check.margin_mm <= 0) {
    throw new Error(`Unsafe fixed actuator margin for ${check.feature_id}: ${JSON.stringify(check)}`)
  }
}

const fixedExplain = run("cargo", [
  "run",
  "--quiet",
  "--",
  "explain",
  path.join(fixedDir, "burr-receipt.json"),
])
expectIncludes(fixedExplain.output, "Status: pass")
expectIncludes(fixedExplain.output, "No failed checks in this receipt.")

console.log("repair loop proof passed")

function readReceipt(dir) {
  return JSON.parse(fs.readFileSync(path.join(dir, "burr-receipt.json"), "utf8"))
}

function checksForRule(receipt, ruleId) {
  return receipt.checks.filter((check) => check.rule_id === ruleId)
}

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

function expectEqual(actual, expected, label) {
  if (actual !== expected) {
    throw new Error(`Unexpected ${label}: got ${actual}, expected ${expected}`)
  }
}

function expectArrayEqual(actual, expected, label) {
  if (!Array.isArray(actual)) {
    throw new Error(`Expected ${label} to be an array; got ${JSON.stringify(actual)}`)
  }
  expectEqual(JSON.stringify(actual), JSON.stringify(expected), label)
}
