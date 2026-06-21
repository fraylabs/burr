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
    `examples/build123d-counterbore/${fixture}/design.py`,
  ])
}

const bad = run("cargo", ["run", "--quiet", "--", "check", "examples/build123d-counterbore/bad"], {
  expectFailure: true,
})
if (!bad.output.includes("Declared counterbore m3_mount_counterbore is missing from the STEP artifact.")) {
  throw new Error(`Bad counterbore fixture did not report the missing counterbore.\n${bad.output}`)
}

const good = run("cargo", ["run", "--quiet", "--", "check", "examples/build123d-counterbore/good"])
if (!good.output.includes("PASS examples/build123d-counterbore/good/burr-design-data.json")) {
  throw new Error(`Good counterbore fixture did not pass.\n${good.output}`)
}

checkReceipt("examples/build123d-counterbore/good/burr-receipt.json", {
  status: "pass",
  checkStatus: "pass",
  checkReason: "ok",
  matchedBore: true,
  matchedCounterbore: true,
  matchedShoulder: true,
})
checkReceipt("examples/build123d-counterbore/bad/burr-receipt.json", {
  status: "fail",
  checkStatus: "fail",
  checkReason: "missing_declared_feature",
  matchedBore: true,
  matchedCounterbore: false,
  matchedShoulder: false,
})

console.log("build123d counterbore proof passed")

function checkReceipt(path, expected) {
  const receipt = JSON.parse(fs.readFileSync(path, "utf8"))
  expectEqual(receipt.status, expected.status, `${path} receipt status`)
  expectEqual(receipt.rulepack_version, "0.6.0", `${path} rulepack version`)

  const summary = receipt.summary.features
  expectEqual(summary.declared, 2, `${path} declared feature count`)
  expectEqual(summary.checked, 1, `${path} checked feature count`)
  expectEqual(summary.unchecked, 1, `${path} unchecked feature count`)
  expectIncludes(summary.checked_feature_ids, "m3_mount_counterbore", `${path} checked feature ids`)
  expectIncludes(summary.unchecked_feature_ids, "cosmetic_counterbore", `${path} unchecked feature ids`)
  expectEqual(summary.intent_counts.mechanical_interface, 1, `${path} mechanical intent count`)
  expectEqual(summary.intent_counts.cosmetic, 1, `${path} cosmetic intent count`)

  const counterboreCheck = receipt.checks.find(
    (check) =>
      check.rule_id === "actuator_mount:counterbore_step_presence" &&
      check.feature_id === "m3_mount_counterbore",
  )
  if (!counterboreCheck) {
    throw new Error(`${path} is missing the counterbore check`)
  }
  expectEqual(counterboreCheck.status, expected.checkStatus, `${path} counterbore status`)
  expectEqual(counterboreCheck.reason, expected.checkReason, `${path} counterbore reason`)
  expectEqual(
    counterboreCheck.measured.matched_bore_cylinder,
    expected.matchedBore,
    `${path} matched bore cylinder`,
  )
  expectEqual(
    counterboreCheck.measured.matched_counterbore_cylinder,
    expected.matchedCounterbore,
    `${path} matched counterbore cylinder`,
  )
  expectEqual(
    counterboreCheck.measured.matched_counterbore_shoulder_plane,
    expected.matchedShoulder,
    `${path} matched shoulder plane`,
  )
  if (counterboreCheck.measured.candidate_cylinders < 2) {
    throw new Error(`${path} expected at least two candidate cylinders`)
  }
  if (counterboreCheck.measured.candidate_planes < 1) {
    throw new Error(`${path} expected at least one candidate plane`)
  }

  const checkedFeatureIds = new Set(
    receipt.checks
      .map((check) => check.feature_id)
      .filter((featureId) => typeof featureId === "string"),
  )
  if (checkedFeatureIds.has("cosmetic_counterbore")) {
    throw new Error(`${path} linted the cosmetic counterbore`)
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
