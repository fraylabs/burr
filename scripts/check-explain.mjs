#!/usr/bin/env node
import { spawnSync } from "node:child_process"

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

console.log("explain proof passed")

function expectIncludes(output, expected) {
  if (!output.includes(expected)) {
    throw new Error(`Expected output to include ${JSON.stringify(expected)}.\n${output}`)
  }
}
