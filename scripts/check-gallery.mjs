#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { galleryExamples } from "./gallery-examples.mjs";

function run(command, args, options = {}) {
  const result = spawnSync(command, args, {
    encoding: "utf8",
    maxBuffer: 1024 * 1024 * 32,
    ...options,
  });
  const output = [result.stdout, result.stderr].filter(Boolean).join("\n");
  if (options.expectFailure) {
    if (result.status === 0) {
      throw new Error(
        `${command} ${args.join(" ")} unexpectedly passed\n${output}`,
      );
    }
  } else if (result.status !== 0) {
    throw new Error(
      `${command} ${args.join(" ")} failed with exit ${result.status}\n${output}`,
    );
  }
  return { ...result, output };
}

for (const example of galleryExamples) {
  if (example.design) {
    run("uv", ["run", "--package", "burr-build123d", "python", example.design]);
  }

  run("cargo", ["run", "--quiet", "--", "check", example.dir], {
    expectFailure: example.expectation === "fail",
  });

  const dataPath = path.join(example.dir, "burr-design-data.json");
  const stepPath = example.step;
  const receiptPath = path.join(example.dir, "burr-receipt.json");

  assertFile(dataPath);
  assertFile(stepPath);
  assertFile(receiptPath);

  const receipt = JSON.parse(fs.readFileSync(receiptPath, "utf8"));
  assertEqual(
    receipt.status,
    example.expectation,
    `${example.dir} receipt status`,
  );
  assertEqual(
    receipt.rulepack_version,
    example.rulepackVersion ?? "0.13.0",
    `${example.dir} rulepack version`,
  );
  if (example.expectation === "pass") {
    assertEqual(receipt.summary.failures, 0, `${example.dir} failure count`);
  } else if (receipt.summary.failures < 1) {
    throw new Error(`${example.dir} expected at least one failing check`);
  }

  for (const expectedFailure of example.expectedFailures ?? []) {
    if (
      !receipt.checks.some(
        (check) =>
          check.rule_id === expectedFailure.rule_id &&
          check.reason === expectedFailure.reason,
      )
    ) {
      throw new Error(
        `${example.dir} did not report ${expectedFailure.rule_id} ${expectedFailure.reason}`,
      );
    }
  }

  if (receipt.summary.features.checked_feature_ids.length < 1) {
    throw new Error(`${example.dir} did not check any features`);
  }

  const designData = JSON.parse(fs.readFileSync(dataPath, "utf8"));
  const stepRef = designData.artifacts.find(
    (artifact) => path.basename(example.step) === artifact.path,
  );
  if (!stepRef?.sha256 || !stepRef?.size_bytes) {
    throw new Error(`${dataPath} did not stamp ${example.artifact}`);
  }
}

console.log("gallery examples passed");

function assertFile(filePath) {
  if (!fs.existsSync(filePath)) {
    throw new Error(`Expected file to exist: ${filePath}`);
  }
}

function assertEqual(actual, expected, label) {
  if (actual !== expected) {
    throw new Error(`Unexpected ${label}: got ${actual}, expected ${expected}`);
  }
}
