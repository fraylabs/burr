#!/usr/bin/env node
import fs from "node:fs"
import path from "node:path"
import { spawnSync } from "node:child_process"

const goodDir = "examples/build123d-counterbore-edge-distance/good"
const badDir = "examples/build123d-counterbore-edge-distance/bad"
const edgeRuleId = "actuator_mount:counterbore_edge_distance"
const presenceRuleId = "actuator_mount:counterbore_step_presence"

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
expectIncludes(bad.output, "FAIL examples/build123d-counterbore-edge-distance/bad/burr-design-data.json")
expectIncludes(bad.output, "Feature m3_head_recess is too close to the edge.")
expectIncludes(bad.output, "Measured feature-to-edge: 1.6 mm")
expectIncludes(bad.output, "Required feature-to-edge: 3 mm")
expectIncludes(bad.output, "Short by: 1.4 mm")

const badReceipt = readReceipt(badDir)
expectEqual(badReceipt.status, "fail", "bad fixture receipt status")
expectEqual(badReceipt.rulepack_version, "0.11.0", "bad fixture rulepack version")
const badEdge = findRuleCheck(badReceipt, edgeRuleId)
expectEqual(badEdge.status, "fail", "bad fixture edge status")
expectEqual(badEdge.reason, "insufficient_feature_edge_distance", "bad fixture edge reason")
expectEqual(badEdge.measured.wall_to_edge_mm, 1.6, "bad fixture measured edge")
expectEqual(badEdge.measured.feature_shape, "circle", "bad fixture edge shape")
expectEqual(badEdge.measured.closest_edge.axis, "y", "bad fixture closest edge axis")
expectEqual(badEdge.measured.closest_edge.side, "min", "bad fixture closest edge side")
expectEqual(badEdge.required.min_wall_to_edge_mm, 3, "bad fixture required edge")
expectEqual(badEdge.required.diameter_field, "counterbore_diameter_mm", "bad fixture diameter field")
expectEqual(badEdge.margin_mm, -1.4, "bad fixture margin")
const badPresence = findRuleCheck(badReceipt, presenceRuleId)
expectEqual(badPresence.status, "pass", "bad fixture presence status")
expectEqual(badPresence.reason, "ok", "bad fixture presence reason")

const badExplain = run("cargo", ["run", "--quiet", "--", "explain", path.join(badDir, "burr-receipt.json")])
expectIncludes(badExplain.output, "Problem: the declared counterbore is too close to a free edge.")
expectIncludes(badExplain.output, "Evidence: Measured feature-to-edge = 1.6 mm.")
expectIncludes(badExplain.output, "Evidence: Required feature-to-edge = 3 mm.")
expectIncludes(
  badExplain.output,
  "Fix: move the counterbore inward, reduce the counterbore diameter, or make the surrounding part larger.",
)

const good = run("cargo", ["run", "--quiet", "--", "check", goodDir])
expectIncludes(good.output, "PASS examples/build123d-counterbore-edge-distance/good/burr-design-data.json")

const goodReceipt = readReceipt(goodDir)
expectEqual(goodReceipt.status, "pass", "good fixture receipt status")
expectEqual(goodReceipt.rulepack_version, "0.11.0", "good fixture rulepack version")
const goodEdge = findRuleCheck(goodReceipt, edgeRuleId)
expectEqual(goodEdge.status, "pass", "good fixture edge status")
expectEqual(goodEdge.reason, "ok", "good fixture edge reason")
expectEqual(goodEdge.measured.wall_to_edge_mm, 6.6, "good fixture measured edge")
expectEqual(goodEdge.required.min_wall_to_edge_mm, 3, "good fixture required edge")
expectEqual(goodEdge.required.diameter_field, "counterbore_diameter_mm", "good fixture diameter field")
expectEqual(goodEdge.margin_mm, 3.6, "good fixture margin")
const goodPresence = findRuleCheck(goodReceipt, presenceRuleId)
expectEqual(goodPresence.status, "pass", "good fixture presence status")
expectEqual(goodPresence.reason, "ok", "good fixture presence reason")

console.log("build123d counterbore edge-distance proof passed")

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
