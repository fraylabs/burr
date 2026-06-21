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
    `examples/build123d-heat-set-insert-pocket/${fixture}/design.py`,
  ])
}

const bad = run(
  "cargo",
  ["run", "--quiet", "--", "check", "examples/build123d-heat-set-insert-pocket/bad"],
  { expectFailure: true },
)
if (!bad.output.includes("Declared heat-set insert pocket m3_insert_pocket is missing from the STEP artifact.")) {
  throw new Error(`Bad heat-set insert pocket fixture did not report the missing pocket.\n${bad.output}`)
}

const good = run("cargo", ["run", "--quiet", "--", "check", "examples/build123d-heat-set-insert-pocket/good"])
if (!good.output.includes("PASS examples/build123d-heat-set-insert-pocket/good/burr-design-data.json")) {
  throw new Error(`Good heat-set insert pocket fixture did not pass.\n${good.output}`)
}

checkReceipt("examples/build123d-heat-set-insert-pocket/good/burr-receipt.json", {
  status: "pass",
  checkStatus: "pass",
  checkReason: "ok",
  matchedPocket: true,
  matchedBottom: true,
})
checkReceipt("examples/build123d-heat-set-insert-pocket/bad/burr-receipt.json", {
  status: "fail",
  checkStatus: "fail",
  checkReason: "missing_declared_feature",
  matchedPocket: true,
  matchedBottom: false,
})

console.log("build123d heat-set insert pocket proof passed")

function checkReceipt(path, expected) {
  const receipt = JSON.parse(fs.readFileSync(path, "utf8"))
  expectEqual(receipt.status, expected.status, `${path} receipt status`)
  expectEqual(receipt.rulepack_version, "0.7.0", `${path} rulepack version`)

  const summary = receipt.summary.features
  expectEqual(summary.declared, 2, `${path} declared feature count`)
  expectEqual(summary.checked, 1, `${path} checked feature count`)
  expectEqual(summary.unchecked, 1, `${path} unchecked feature count`)
  expectIncludes(summary.checked_feature_ids, "m3_insert_pocket", `${path} checked feature ids`)
  expectIncludes(summary.unchecked_feature_ids, "cosmetic_insert_pocket", `${path} unchecked feature ids`)
  expectEqual(summary.intent_counts.mechanical_interface, 1, `${path} mechanical intent count`)
  expectEqual(summary.intent_counts.cosmetic, 1, `${path} cosmetic intent count`)

  const pocketCheck = receipt.checks.find(
    (check) =>
      check.rule_id === "actuator_mount:heat_set_insert_pocket_step_presence" &&
      check.feature_id === "m3_insert_pocket",
  )
  if (!pocketCheck) {
    throw new Error(`${path} is missing the heat-set insert pocket check`)
  }
  expectEqual(pocketCheck.status, expected.checkStatus, `${path} pocket status`)
  expectEqual(pocketCheck.reason, expected.checkReason, `${path} pocket reason`)
  expectEqual(
    pocketCheck.measured.matched_pocket_cylinder,
    expected.matchedPocket,
    `${path} matched pocket cylinder`,
  )
  expectEqual(
    pocketCheck.measured.matched_pocket_bottom_plane,
    expected.matchedBottom,
    `${path} matched pocket bottom plane`,
  )
  if (pocketCheck.measured.candidate_cylinders < 1) {
    throw new Error(`${path} expected at least one candidate cylinder`)
  }
  if (pocketCheck.measured.candidate_planes < 1) {
    throw new Error(`${path} expected at least one candidate plane`)
  }

  const checkedFeatureIds = new Set(
    receipt.checks
      .map((check) => check.feature_id)
      .filter((featureId) => typeof featureId === "string"),
  )
  if (checkedFeatureIds.has("cosmetic_insert_pocket")) {
    throw new Error(`${path} linted the cosmetic insert pocket`)
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
