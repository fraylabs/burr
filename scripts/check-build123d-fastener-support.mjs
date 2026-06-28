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
    `examples/build123d-fastener-support/${fixture}/design.py`,
  ])
}

const bad = run("cargo", ["run", "--quiet", "--", "check", "examples/build123d-fastener-support/bad"], {
  expectFailure: true,
})
if (!bad.output.includes("Measured support wall: 1.3 mm")) {
  throw new Error(`Bad fastener-support fixture did not report measured support wall.\n${bad.output}`)
}
if (!bad.output.includes("Short by: 0.7 mm")) {
  throw new Error(`Bad fastener-support fixture did not report expected shortage.\n${bad.output}`)
}

const good = run("cargo", ["run", "--quiet", "--", "check", "examples/build123d-fastener-support/good"])
if (!good.output.includes("PASS examples/build123d-fastener-support/good/burr-design-data.json")) {
  throw new Error(`Good fastener-support fixture did not pass.\n${good.output}`)
}

checkReceipt("examples/build123d-fastener-support/good/burr-receipt.json", {
  status: "pass",
  supportWall: 2.3,
})
checkReceipt("examples/build123d-fastener-support/bad/burr-receipt.json", {
  status: "fail",
  supportWall: 1.3,
})

console.log("build123d fastener-support proof passed")

function checkReceipt(path, expected) {
  const receipt = JSON.parse(fs.readFileSync(path, "utf8"))
  expectEqual(receipt.status, expected.status, `${path} receipt status`)
  expectEqual(receipt.rulepack_version, "0.11.0", `${path} rulepack version`)

  const check = receipt.checks.find(
    (item) =>
      item.rule_id === "actuator_mount:m3_bossed_mount_support_wall_thickness" &&
      item.feature_id === "m3_bossed_mount",
  )
  if (!check) {
    throw new Error(`${path} is missing the fastener-support wall check`)
  }
  expectEqual(check.measured.support_wall_thickness_mm, expected.supportWall, `${path} support wall`)
}

function expectEqual(actual, expected, label) {
  if (actual !== expected) {
    throw new Error(`Unexpected ${label}: got ${actual}, expected ${expected}`)
  }
}
