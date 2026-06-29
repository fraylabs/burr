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
    `examples/build123d-standoff-boss-presence/${fixture}/design.py`,
  ])
}

const bad = run(
  "cargo",
  ["run", "--quiet", "--", "check", "examples/build123d-standoff-boss-presence/bad"],
  { expectFailure: true },
)
if (!bad.output.includes("Declared standoff boss m3_standoff_boss is missing from the STEP artifact.")) {
  throw new Error(`Bad standoff-boss fixture did not report the missing boss.\n${bad.output}`)
}

const good = run("cargo", [
  "run",
  "--quiet",
  "--",
  "check",
  "examples/build123d-standoff-boss-presence/good",
])
if (!good.output.includes("PASS examples/build123d-standoff-boss-presence/good/burr-design-data.json")) {
  throw new Error(`Good standoff-boss fixture did not pass.\n${good.output}`)
}

checkReceipt("examples/build123d-standoff-boss-presence/good/burr-receipt.json", {
  status: "pass",
  matchedBoss: true,
  matchedTop: true,
})
checkReceipt("examples/build123d-standoff-boss-presence/bad/burr-receipt.json", {
  status: "fail",
  matchedBoss: false,
  matchedTop: false,
})

console.log("build123d standoff-boss presence proof passed")

function checkReceipt(path, expected) {
  const receipt = JSON.parse(fs.readFileSync(path, "utf8"))
  expectEqual(receipt.status, expected.status, `${path} receipt status`)
  expectEqual(receipt.rulepack_version, "0.14.0", `${path} rulepack version`)

  const check = receipt.checks.find(
    (item) =>
      item.rule_id === "actuator_mount:m3_standoff_boss_step_presence" &&
      item.feature_id === "m3_standoff_boss",
  )
  if (!check) {
    throw new Error(`${path} is missing the standoff-boss presence check`)
  }
  expectEqual(check.measured.matched_boss_cylinder, expected.matchedBoss, `${path} matched boss`)
  expectEqual(check.measured.matched_boss_top_plane, expected.matchedTop, `${path} matched top`)
}

function expectEqual(actual, expected, label) {
  if (actual !== expected) {
    throw new Error(`Unexpected ${label}: got ${actual}, expected ${expected}`)
  }
}
