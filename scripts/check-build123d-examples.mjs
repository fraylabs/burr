#!/usr/bin/env node
import { spawnSync } from "node:child_process"

const python = splitCommand(process.env.PYTHON ?? "python")

function splitCommand(command) {
  return command.trim().split(/\s+/).filter(Boolean)
}

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

function runPython(args) {
  return run(python[0], [...python.slice(1), ...args])
}

runPython(["examples/build123d-actuator/bad/design.py"])
runPython(["examples/build123d-actuator/good/design.py"])

const bad = run(
  "node",
  ["bin/burr.mjs", "check", "examples/build123d-actuator/bad", "--no-write-receipt"],
  { expectFailure: true },
)
if (!bad.output.includes("Short by: 2.2 mm")) {
  throw new Error(`Bad build123d fixture did not print the expected fix hint.\n${bad.output}`)
}

const good = run(
  "node",
  ["bin/burr.mjs", "check", "examples/build123d-actuator/good", "--no-write-receipt"],
)
if (!good.output.includes("PASS examples/build123d-actuator/good/burr-design-data.json")) {
  throw new Error(`Good build123d fixture did not pass as expected.\n${good.output}`)
}

console.log("build123d examples passed")
