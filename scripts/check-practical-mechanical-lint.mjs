#!/usr/bin/env node
import fs from "node:fs"
import path from "node:path"
import { spawnSync } from "node:child_process"

const goodExamples = [
  {
    dir: "examples/gallery/practical-insert-fit-good",
    design: "examples/gallery/practical-insert-fit-good/design.py",
  },
  {
    dir: "examples/gallery/practical-driver-access-good",
    design: "examples/gallery/practical-driver-access-good/design.py",
  },
  {
    dir: "examples/gallery/practical-mount-pattern-good",
    design: "examples/gallery/practical-mount-pattern-good/design.py",
  },
  {
    dir: "examples/gallery/practical-snap-hook-good",
    design: "examples/gallery/practical-snap-hook-good/design.py",
  },
  {
    dir: "examples/gallery/practical-boss-support-good",
    design: "examples/gallery/practical-boss-support-good/design.py",
  },
]

const caughtMistakes = [
  {
    dir: "examples/gallery/practical-insert-fit-tight",
    design: "examples/gallery/practical-insert-fit-tight/design.py",
    expectedFailures: [
      {
        rule_id: "hardware_fit:m3_insert_pocket_radial_clearance",
        reason: "numeric_value_out_of_range",
      },
    ],
  },
  {
    dir: "examples/gallery/practical-driver-access-blocked",
    design: "examples/gallery/practical-driver-access-blocked/design.py",
    expectedFailures: [
      {
        rule_id: "tool_access:m3_driver_access_diameter",
        reason: "numeric_value_out_of_range",
      },
      {
        rule_id: "tool_access:m3_driver_side_clearance",
        reason: "numeric_value_out_of_range",
      },
    ],
  },
  {
    dir: "examples/gallery/practical-mount-pattern-shifted",
    design: "examples/gallery/practical-mount-pattern-shifted/design.py",
    expectedFailures: [
      {
        rule_id: "mount_pattern:m3_mount_pattern_max_pitch_error",
        reason: "numeric_value_out_of_range",
      },
    ],
  },
  {
    dir: "examples/gallery/practical-snap-hook-thin",
    design: "examples/gallery/practical-snap-hook-thin/design.py",
    expectedFailures: [
      {
        rule_id: "printable_retention:snap_hook_arm_thickness",
        reason: "numeric_value_out_of_range",
      },
    ],
  },
  {
    dir: "examples/gallery/practical-boss-support-unsupported",
    design: "examples/gallery/practical-boss-support-unsupported/design.py",
    expectedFailures: [
      {
        rule_id: "boss_support:m3_boss_height_to_diameter_ratio",
        reason: "numeric_value_out_of_range",
      },
      {
        rule_id: "boss_support:m3_boss_gusset_inventory",
        reason: "feature_count_out_of_range",
      },
    ],
  },
]

for (const example of [...goodExamples, ...caughtMistakes]) {
  run("uv", ["run", "--package", "burr-build123d", "python", example.design])
}

for (const example of goodExamples) {
  const result = run("cargo", ["run", "--quiet", "--", "check", example.dir])
  expectIncludes(result.output, `PASS ${example.dir}/burr-design-data.json`)
  const receipt = readReceipt(example.dir)
  expectEqual(receipt.status, "pass", `${example.dir} receipt status`)
  expectEqual(receipt.rulepack_version, "0.1.0", `${example.dir} rulepack version`)
  expectEqual(receipt.summary.failures, 0, `${example.dir} failures`)
}

for (const example of caughtMistakes) {
  run("cargo", ["run", "--quiet", "--", "check", example.dir], { expectFailure: true })
  const receipt = readReceipt(example.dir)
  expectEqual(receipt.status, "fail", `${example.dir} receipt status`)
  expectEqual(receipt.rulepack_version, "0.1.0", `${example.dir} rulepack version`)
  for (const failure of example.expectedFailures) {
    findRuleCheck(receipt, failure.rule_id, failure.reason)
  }
}

console.log("practical mechanical lint proof passed")

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

function readReceipt(dir) {
  return JSON.parse(fs.readFileSync(path.join(dir, "burr-receipt.json"), "utf8"))
}

function findRuleCheck(receipt, ruleId, reason) {
  const check = receipt.checks.find(
    (candidate) => candidate.rule_id === ruleId && candidate.reason === reason,
  )
  if (!check) {
    throw new Error(`Missing ${ruleId} ${reason} in ${receipt.artifact_id}`)
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
