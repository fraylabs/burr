#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";

const goodExamples = [
  {
    dir: "examples/gallery/t-slot-linear-slider",
    design: "examples/gallery/t-slot-linear-slider/design.py",
    rulepackVersion: "0.1.0",
  },
  {
    dir: "examples/gallery/dense-random-hole-plate",
    design: "examples/gallery/dense-random-hole-plate/design.py",
    rulepackVersion: "0.1.0",
  },
  {
    dir: "examples/gallery/relief-envelope-plate",
    design: "examples/gallery/relief-envelope-plate/design.py",
    rulepackVersion: "0.1.0",
  },
];

const caughtMistakes = [
  {
    dir: "examples/gallery/t-slot-linear-slider-tight-clearance",
    design: "examples/gallery/t-slot-linear-slider-tight-clearance/design.py",
    rulepackVersion: "0.1.0",
    expectedFailure: {
      rule_id: "captured_slider:head_side_clearance_window",
      reason: "numeric_value_out_of_range",
    },
  },
  {
    dir: "examples/gallery/t-slot-linear-slider-missing-capture-lip",
    design: "examples/gallery/t-slot-linear-slider-missing-capture-lip/design.py",
    rulepackVersion: "0.1.0",
    expectedFailure: {
      rule_id: "captured_slider:capture_lip_inventory",
      reason: "feature_count_out_of_range",
    },
  },
  {
    dir: "examples/gallery/t-slot-linear-slider-shallow-capture-lip",
    design: "examples/gallery/t-slot-linear-slider-shallow-capture-lip/design.py",
    rulepackVersion: "0.1.0",
    expectedFailure: {
      rule_id: "captured_slider:capture_lip_engagement",
      reason: "numeric_value_out_of_range",
    },
  },
  {
    dir: "examples/gallery/dense-random-hole-plate-too-many-reliefs",
    design: "examples/gallery/dense-random-hole-plate-too-many-reliefs/design.py",
    rulepackVersion: "0.1.0",
    expectedFailure: {
      rule_id: "printed_plate:cosmetic_relief_inventory",
      reason: "feature_count_out_of_range",
    },
  },
  {
    dir: "examples/gallery/hole-slot-thin-ligament",
    design: "examples/gallery/hole-slot-thin-ligament/design.py",
    rulepackVersion: "0.1.0",
    expectedFailure: {
      rule_id: "printed_plate:cosmetic_relief_ligament",
      reason: "insufficient_feature_pair_spacing",
    },
  },
];

for (const example of goodExamples) {
  run("uv", ["run", "--package", "burr-build123d", "python", example.design]);
  run("cargo", ["run", "--quiet", "--", "check", example.dir]);
  const receipt = readReceipt(example.dir);
  expectEqual(receipt.status, "pass", `${example.dir} receipt status`);
  expectEqual(receipt.rulepack_version, example.rulepackVersion, `${example.dir} rulepack version`);
  expectEqual(receipt.summary.failures, 0, `${example.dir} failures`);
}

for (const example of caughtMistakes) {
  run("uv", ["run", "--package", "burr-build123d", "python", example.design]);
  run("cargo", ["run", "--quiet", "--", "check", example.dir], { expectFailure: true });
  const receipt = readReceipt(example.dir);
  expectEqual(receipt.status, "fail", `${example.dir} receipt status`);
  expectEqual(receipt.rulepack_version, example.rulepackVersion, `${example.dir} rulepack version`);
  if (
    !receipt.checks.some(
      (check) =>
        check.rule_id === example.expectedFailure.rule_id &&
        check.reason === example.expectedFailure.reason,
    )
  ) {
    throw new Error(
      `${example.dir} did not report ${example.expectedFailure.rule_id} ${example.expectedFailure.reason}`,
    );
  }
}

console.log("mistake library v1 proof passed");

function run(command, args, options = {}) {
  const result = spawnSync(command, args, {
    encoding: "utf8",
    maxBuffer: 1024 * 1024 * 32,
    ...options,
  });
  const output = [result.stdout, result.stderr].filter(Boolean).join("\n");
  if (options.expectFailure) {
    if (result.status === 0) {
      throw new Error(`${command} ${args.join(" ")} unexpectedly passed\n${output}`);
    }
  } else if (result.status !== 0) {
    throw new Error(`${command} ${args.join(" ")} failed with exit ${result.status}\n${output}`);
  }
}

function readReceipt(dir) {
  return JSON.parse(fs.readFileSync(path.join(dir, "burr-receipt.json"), "utf8"));
}

function expectEqual(actual, expected, label) {
  if (actual !== expected) {
    throw new Error(`Unexpected ${label}: got ${actual}, expected ${expected}`);
  }
}
