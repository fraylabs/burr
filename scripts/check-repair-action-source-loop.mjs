#!/usr/bin/env node
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { buildRepairReports } from "./repair-report-builder.mjs";

const beforeSlug = "bad-actuator-housing-edge-distance";
const afterSlug = "fixed-actuator-housing";
const reportId = "actuator-housing-edge-distance";
const focusRuleId = "actuator_mount:m3_loaded_hole_edge_distance";
const designDataFile = "burr-design-data.json";
const receiptFile = "burr-receipt.json";
const badSource = "examples/build123d-actuator-housing-repair/bad/design.py";
const fixedSource = "examples/build123d-actuator-housing-repair/fixed/design.py";
const expectedFeatureIds = [
  "m3_front_left",
  "m3_front_right",
  "m3_rear_left",
  "m3_rear_right",
];

const packageJson = readJson("package.json");
const tmp = fs.mkdtempSync(path.join(os.tmpdir(), "burr-repair-action-source-loop-"));

try {
  const beforeDir = path.join(tmp, "before");
  const afterDir = path.join(tmp, "after");
  const repairedDir = path.join(tmp, "repaired-from-actions");
  fs.mkdirSync(beforeDir, { recursive: true });
  fs.mkdirSync(afterDir, { recursive: true });
  fs.mkdirSync(repairedDir, { recursive: true });

  copyRequired(badSource, path.join(beforeDir, "design.py"));
  copyRequired(fixedSource, path.join(afterDir, "design.py"));
  copyRequired(badSource, path.join(repairedDir, "design.py"));

  runGenerator(beforeDir);
  const beforeCheck = runBurrCheck(beforeDir, { expectFailure: true });
  expectIncludes(beforeCheck.output, "FAIL", "bad source Burr check output");

  runGenerator(afterDir);
  const afterCheck = runBurrCheck(afterDir);
  expectIncludes(afterCheck.output, "PASS", "fixed source Burr check output");

  const releaseDir = path.join(tmp, "release");
  const report = generateRepairReport({ releaseDir, beforeDir, afterDir });
  expectEqual(report.report_id, reportId, "repair report id");
  expectEqual(report.focus_rule_id, focusRuleId, "repair report focus rule");
  expectEqual(report.before.status, "fail", "repair report before status");
  expectEqual(report.after.status, "pass", "repair report after status");

  const actions = report.repair_actions ?? [];
  const moveActions = actions.filter((action) => action.action === "move_feature");
  const sourceHintActions = actions.filter((action) => action.source_hint !== undefined);
  expectEqual(moveActions.length, expectedFeatureIds.length, "move repair action count");
  if (sourceHintActions.length !== actions.length) {
    throw new Error("Every repair action must include source_hint for the source repair loop.");
  }
  expectArrayEqual(
    moveActions.map((action) => action.feature_id).sort(),
    expectedFeatureIds,
    "repair action features",
  );

  const sourcePath = path.join(repairedDir, "design.py");
  const startingSource = fs.readFileSync(sourcePath, "utf8");
  const repairedSource = applyRepairActions(startingSource, actions);
  if (repairedSource === startingSource) {
    throw new Error("Repair actions did not modify the copied source.");
  }
  fs.writeFileSync(sourcePath, repairedSource);

  runGenerator(repairedDir);
  const repairedCheck = runBurrCheck(repairedDir);
  expectIncludes(repairedCheck.output, "PASS", "repaired source Burr check output");

  const repairedDesign = readJson(path.join(repairedDir, designDataFile));
  const repairedReceipt = readJson(path.join(repairedDir, receiptFile));
  expectEqual(repairedReceipt.status, "pass", "repaired source receipt status");
  expectPassingFileHashes(repairedReceipt);
  expectRepairedFeatures({ repairedDesign, repairedReceipt, actions: moveActions });

  console.log("repair action source loop proof passed");
} finally {
  fs.rmSync(tmp, { recursive: true, force: true });
}

function generateRepairReport({ releaseDir, beforeDir, afterDir }) {
  fs.mkdirSync(path.join(releaseDir, beforeSlug), { recursive: true });
  fs.mkdirSync(path.join(releaseDir, afterSlug), { recursive: true });

  copyRequired(
    path.join(beforeDir, receiptFile),
    path.join(releaseDir, beforeSlug, `${beforeSlug}.receipt.json`),
  );
  copyRequired(
    path.join(beforeDir, designDataFile),
    path.join(releaseDir, beforeSlug, `${beforeSlug}.design-data.json`),
  );
  copyRequired(
    path.join(afterDir, receiptFile),
    path.join(releaseDir, afterSlug, `${afterSlug}.receipt.json`),
  );
  copyRequired(
    path.join(afterDir, designDataFile),
    path.join(releaseDir, afterSlug, `${afterSlug}.design-data.json`),
  );

  const entries = buildRepairReports({
    releaseDir,
    version: packageJson.version,
    generatedAt: "1970-01-01T00:00:00.000Z",
    examples: [
      {
        slug: beforeSlug,
        receipt: `${beforeSlug}/${beforeSlug}.receipt.json`,
        design_data: `${beforeSlug}/${beforeSlug}.design-data.json`,
      },
      {
        slug: afterSlug,
        receipt: `${afterSlug}/${afterSlug}.receipt.json`,
        design_data: `${afterSlug}/${afterSlug}.design-data.json`,
      },
    ],
  });

  const entry = entries.find((item) => item.id === reportId);
  if (!entry) {
    throw new Error(`Generated repair reports did not include ${reportId}`);
  }
  return readJson(path.join(releaseDir, entry.report_json));
}

function applyRepairActions(source, actions) {
  return actions.reduce((current, action) => applyRepairAction(current, action), source);
}

function applyRepairAction(source, action) {
  expectEqual(action.rule_id, focusRuleId, `repair action rule for ${action.feature_id}`);
  if (action.action === "move_feature") {
    expectEqual(action.parameter, "center_mm", `repair action parameter for ${action.feature_id}`);
    expectVector(action.before_value_mm, `before value for ${action.feature_id}`);
    expectVector(action.after_value_mm, `after value for ${action.feature_id}`);
  } else if (action.action === "resize_part_envelope") {
    expectNumber(action.before_value_mm, `before value for ${action.feature_id}`);
    expectNumber(action.after_value_mm, `after value for ${action.feature_id}`);
  } else {
    throw new Error(`Unexpected repair action kind: ${action.action}`);
  }

  const sourceHint = normalizeSourceHint(action);
  return applyTextSourceHint(source, sourceHint, action.feature_id);
}

function normalizeSourceHint(action) {
  const sourceHint = action.source_hint ?? action.sourceHint;
  if (sourceHint === undefined) {
    throw new Error(
      `Repair action for ${action.feature_id} is missing source_hint. Expected source_hint.before_text and source_hint.after_text exact source snippets.`,
    );
  }
  return sourceHint;
}

function applyTextSourceHint(source, sourceHint, featureId) {
  // The proof loop is intentionally generic: it applies only exact text
  // replacements supplied by the report producer, with no fixture-specific
  // parsing or geometry inference.
  if (
    !sourceHint ||
    typeof sourceHint !== "object" ||
    typeof sourceHint.before_text !== "string" ||
    typeof sourceHint.after_text !== "string"
  ) {
    throw new Error(
      `Repair action for ${featureId} has invalid source_hint. Expected shape: { before_text: string, after_text: string } with exact source snippets.`,
    );
  }
  return replaceOnce(source, sourceHint.before_text, sourceHint.after_text, `source_hint for ${featureId}`);
}

function runGenerator(dir) {
  run("uv", ["run", "--package", "burr-build123d", "python", path.join(dir, "design.py")]);
  expectFile(path.join(dir, designDataFile), "generated design data");
  expectFile(path.join(dir, "actuator-housing.step"), "generated STEP artifact");
}

function runBurrCheck(dir, options = {}) {
  const result = run("cargo", ["run", "--quiet", "--", "check", dir], options);
  expectFile(path.join(dir, receiptFile), "Burr receipt");
  return result;
}

function run(command, args, options = {}) {
  const result = spawnSync(command, args, {
    encoding: "utf8",
    maxBuffer: 1024 * 1024 * 32,
  });
  const output = [result.stdout, result.stderr].filter(Boolean).join("\n");
  if (options.expectFailure) {
    if (result.status === 0) {
      throw new Error(`${command} ${args.join(" ")} unexpectedly passed\n${output}`);
    }
  } else if (result.status !== 0) {
    throw new Error(`${command} ${args.join(" ")} failed with exit ${result.status}\n${output}`);
  }
  return { ...result, output };
}

function expectRepairedFeatures({ repairedDesign, repairedReceipt, actions }) {
  const featureById = new Map((repairedDesign.features ?? []).map((feature) => [feature.id, feature]));
  const checkById = new Map(
    repairedReceipt.checks
      .filter((check) => check.rule_id === focusRuleId)
      .map((check) => [check.feature_id, check]),
  );

  for (const action of actions) {
    const feature = featureById.get(action.feature_id);
    if (!feature) {
      throw new Error(`Repaired design data is missing feature ${action.feature_id}`);
    }
    expectArrayEqual(feature.center_mm, action.after_value_mm, `${action.feature_id} repaired center`);

    const check = checkById.get(action.feature_id);
    if (!check) {
      throw new Error(`Repaired receipt is missing ${focusRuleId} check for ${action.feature_id}`);
    }
    expectEqual(check.status, "pass", `${action.feature_id} repaired check status`);
    expectEqual(check.reason, "ok", `${action.feature_id} repaired check reason`);
    if (check.margin_mm <= 0) {
      throw new Error(`Repaired check margin must be positive for ${action.feature_id}: ${check.margin_mm}`);
    }
  }
}

function expectPassingFileHashes(receipt) {
  for (const ruleId of [
    "burr_design_data:source_sha256_matches",
    "burr_design_data:artifact_sha256_matches",
  ]) {
    const check = receipt.checks.find((item) => item.rule_id === ruleId);
    if (!check) {
      throw new Error(`Repaired receipt is missing ${ruleId}`);
    }
    expectEqual(check.status, "pass", `${ruleId} status`);
  }
}

function copyRequired(source, target) {
  if (!fs.existsSync(source)) {
    throw new Error(`Missing required file: ${source}`);
  }
  fs.copyFileSync(source, target);
}

function readJson(file) {
  return JSON.parse(fs.readFileSync(file, "utf8"));
}

function expectFile(file, label) {
  if (!fs.existsSync(file)) {
    throw new Error(`Missing ${label}: ${file}`);
  }
  if (fs.statSync(file).size === 0) {
    throw new Error(`${label} is empty: ${file}`);
  }
}

function replaceOnce(source, before, after, label) {
  const first = source.indexOf(before);
  if (first < 0) {
    throw new Error(`Could not find ${label}`);
  }
  const second = source.indexOf(before, first + before.length);
  if (second >= 0) {
    throw new Error(`Expected one ${label}, found multiple`);
  }
  return `${source.slice(0, first)}${after}${source.slice(first + before.length)}`;
}

function expectVector(value, label) {
  if (!Array.isArray(value) || value.length !== 3 || value.some((item) => typeof item !== "number")) {
    throw new Error(`Expected numeric 3-vector for ${label}; got ${JSON.stringify(value)}`);
  }
}

function expectNumber(value, label) {
  if (typeof value !== "number" || !Number.isFinite(value)) {
    throw new Error(`Expected finite number for ${label}; got ${JSON.stringify(value)}`);
  }
}

function expectIncludes(value, expected, label) {
  if (!value.includes(expected)) {
    throw new Error(`Expected ${label} to include ${JSON.stringify(expected)}.\n${value}`);
  }
}

function expectEqual(actual, expected, label) {
  if (actual !== expected) {
    throw new Error(`Unexpected ${label}: got ${JSON.stringify(actual)}, expected ${JSON.stringify(expected)}`);
  }
}

function expectArrayEqual(actual, expected, label) {
  expectEqual(JSON.stringify(actual), JSON.stringify(expected), label);
}
