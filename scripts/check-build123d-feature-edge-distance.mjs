#!/usr/bin/env node
import fs from "node:fs"
import path from "node:path"
import { spawnSync } from "node:child_process"

const goodDir = "examples/build123d-feature-edge-distance/good"
const badDir = "examples/build123d-feature-edge-distance/bad"
const ruleId = "actuator_mount:mechanical_slot_edge_distance"

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

for (const dir of [goodDir, badDir]) {
  run("uv", ["run", "--package", "burr-build123d", "python", path.join(dir, "design.py")])
}

const bad = run("cargo", ["run", "--quiet", "--", "check", badDir], {
  expectFailure: true,
})
expectIncludes(bad.output, "FAIL examples/build123d-feature-edge-distance/bad/burr-design-data.json")
expectIncludes(bad.output, "Feature motor_adjust_slot is too close to the edge.")
expectIncludes(bad.output, "Measured feature-to-edge: 2 mm")
expectIncludes(bad.output, "Required feature-to-edge: 3 mm")
expectIncludes(bad.output, "Short by: 1 mm")

const badReceipt = readReceipt(badDir)
expectEqual(badReceipt.status, "fail", "bad fixture receipt status")
expectEqual(badReceipt.rulepack_version, "0.10.0", "bad fixture rulepack version")
const badCheck = findRuleCheck(badReceipt, ruleId)
expectEqual(badCheck.status, "fail", "bad fixture edge status")
expectEqual(badCheck.reason, "insufficient_feature_edge_distance", "bad fixture edge reason")
expectEqual(badCheck.measured.wall_to_edge_mm, 2, "bad fixture measured edge")
expectEqual(badCheck.required.min_wall_to_edge_mm, 3, "bad fixture required edge")
expectEqual(badCheck.margin_mm, -1, "bad fixture margin")

const badExplain = run("cargo", ["run", "--quiet", "--", "explain", path.join(badDir, "burr-receipt.json")])
expectIncludes(badExplain.output, "Problem: the declared straight slot is too close to a free edge.")
expectIncludes(badExplain.output, "Evidence: Measured feature-to-edge = 2 mm.")
expectIncludes(badExplain.output, "Evidence: Required feature-to-edge = 3 mm.")
expectIncludes(badExplain.output, "Fix: move the feature inward, shorten the feature, or make the surrounding part larger.")

const good = run("cargo", ["run", "--quiet", "--", "check", goodDir])
expectIncludes(good.output, "PASS examples/build123d-feature-edge-distance/good/burr-design-data.json")

const goodReceipt = readReceipt(goodDir)
expectEqual(goodReceipt.status, "pass", "good fixture receipt status")
expectEqual(goodReceipt.rulepack_version, "0.10.0", "good fixture rulepack version")
const goodCheck = findRuleCheck(goodReceipt, ruleId)
expectEqual(goodCheck.status, "pass", "good fixture edge status")
expectEqual(goodCheck.reason, "ok", "good fixture edge reason")
expectEqual(goodCheck.measured.wall_to_edge_mm, 5, "good fixture measured edge")
expectEqual(goodCheck.required.min_wall_to_edge_mm, 3, "good fixture required edge")
expectEqual(goodCheck.margin_mm, 2, "good fixture margin")

console.log("build123d feature-edge-distance proof passed")

function readReceipt(dir) {
  return JSON.parse(fs.readFileSync(path.join(dir, "burr-receipt.json"), "utf8"))
}

function findRuleCheck(receipt, ruleId) {
  const check = receipt.checks.find((check) => check.rule_id === ruleId)
  if (!check) {
    throw new Error(`Missing check ${ruleId}`)
  }
  return check
}

function expectIncludes(output, expected) {
  if (!output.includes(expected)) {
    throw new Error(`Expected output to include ${JSON.stringify(expected)}.\n${output}`)
  }
}

function expectEqual(actual, expected, label) {
  if (actual !== expected) {
    throw new Error(`${label}: expected ${JSON.stringify(expected)}, got ${JSON.stringify(actual)}`)
  }
}
