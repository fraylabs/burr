#!/usr/bin/env node
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";

const designDataFile = "burr-design-data.json";
const receiptFile = "burr-receipt.json";
const fixtureDir = "examples/gallery/relief-envelope-plate-thin-ligament";
const goodSource = "examples/gallery/relief-envelope-plate/design.py";
const focusRuleId = "printed_plate:cosmetic_relief_ligament";
const featureId = "near_relief_hole";
const beforeText = `    ("near_relief_hole", 13.2, 0.0, 4.0),`;
const afterText = `    ("near_relief_hole", 15.0, 0.0, 4.0),`;
const beforeCenter = [0, 13.2, 0];
const afterCenter = [0, 15, 0];

const tmp = fs.mkdtempSync(path.join(os.tmpdir(), "burr-spacing-envelope-agent-repair-"));

try {
  expectOccursOnce(readText(path.join(fixtureDir, "design.py")), beforeText, "bad source before_text");
  expectOccursOnce(readText(goodSource), afterText, "good source after_text");

  const before = copyFixture("before");
  runGenerator(before.dir);
  runBurrCheck(before.dir, { expectFailure: true });
  const beforeReceipt = readJson(path.join(before.dir, receiptFile));
  const beforeDesign = readJson(path.join(before.dir, designDataFile));
  expectEqual(beforeReceipt.status, "fail", "before receipt status");
  const beforeCheck = expectCheck(beforeReceipt, focusRuleId, { status: "fail" });
  expectEqual(beforeCheck.reason, "insufficient_feature_pair_spacing", "before failure reason");
  expectEqual(beforeCheck.margin_mm, -1.0, "before margin");
  expectClosestPair(beforeCheck, {
    featureIds: ["rounded_relief_window", featureId],
    featureShapes: ["capsule", "circle"],
    shapeDistanceMm: 5.2,
    clearanceMm: 0.2,
  });

  const reportPath = path.join(tmp, "spacing-envelope-repair-report.json");
  fs.writeFileSync(
    reportPath,
    `${JSON.stringify(buildRepairReport({ beforeReceipt, beforeDesign }), null, 2)}\n`,
  );

  const explain = run("cargo", ["run", "--quiet", "--", "explain", "--json", reportPath]);
  const packetPath = path.join(tmp, "spacing-envelope-repair-packet.json");
  fs.writeFileSync(packetPath, explain.output);
  const packet = JSON.parse(explain.output);
  expectEqual(packet.schema_version, "burr.repair-packet.v1", "repair packet schema");
  expectEqual(packet.source_kind, "repair_report", "repair packet source kind");
  expectEqual(packet.summary?.exact_source_edits_available, true, "exact source edits available");
  expectEqual(packet.summary?.exact_source_edit_count, 1, "exact source edit count");
  expectEqual(packet.repair_actions?.[0]?.source_hint?.before_text, beforeText, "packet before_text");
  expectEqual(packet.repair_actions?.[0]?.source_hint?.after_text, afterText, "packet after_text");

  const target = copyFixture("target");
  const runner = run("node", [
    "scripts/agent-repair-runner.mjs",
    target.dir,
    "--packet",
    packetPath,
    "--source-file",
    "design.py",
  ]);
  const runnerReceipt = JSON.parse(runner.output);
  expectEqual(runnerReceipt.status, "repaired", "runner status");
  expectEqual(runnerReceipt.before.status, "fail", "runner before status");
  expectEqual(runnerReceipt.after.status, "pass", "runner after status");
  expectEqual(runnerReceipt.applied_edits.length, 1, "runner applied edit count");

  const repairedSource = readText(path.join(target.dir, "design.py"));
  expectOccursOnce(repairedSource, afterText, "repaired source after_text");
  const repairedDesign = readJson(path.join(target.dir, designDataFile));
  const repairedFeature = repairedDesign.features.find((feature) => feature.id === featureId);
  expectArrayEqual(repairedFeature?.center_mm, afterCenter, "repaired feature center");

  const finalReceipt = readJson(path.join(target.dir, receiptFile));
  expectEqual(finalReceipt.status, "pass", "final receipt status");
  expectPassingFileHashes(finalReceipt);
  const finalCheck = expectCheck(finalReceipt, focusRuleId, { status: "pass" });
  expectEqual(finalCheck.reason, "ok", "final check reason");
  expectEqual(finalCheck.margin_mm, 0.8, "final margin");
  expectClosestPair(finalCheck, {
    featureIds: ["rounded_relief_window", featureId],
    featureShapes: ["capsule", "circle"],
    shapeDistanceMm: 7.0,
    clearanceMm: 2.0,
  });

  console.log("spacing-envelope agent repair proof passed");
} finally {
  fs.rmSync(tmp, { recursive: true, force: true });
}

function buildRepairReport({ beforeReceipt, beforeDesign }) {
  return {
    schema_version: "burr.repair-report.v1",
    id: "spacing-envelope-ligament",
    report_id: "spacing-envelope-ligament",
    status: "repaired",
    focus_rule_id: focusRuleId,
    before: {
      status: "fail",
      receipt: "before/burr-receipt.json",
    },
    after: {
      status: "pass",
      receipt: "after/burr-receipt.json",
    },
    summary: {
      before_failures: 1,
      after_failures: 0,
      repair_status: "repaired",
    },
    failures: beforeReceipt.checks.filter((check) => check.rule_id === focusRuleId && check.status === "fail"),
    repair_actions: [
      {
        action: "move_feature",
        rule_id: focusRuleId,
        feature_id: featureId,
        parameter: "center_mm",
        failure_reason: "insufficient_feature_pair_spacing",
        before_value_mm: beforeCenter,
        after_value_mm: afterCenter,
        before_margin_mm: -1.0,
        after_margin_mm: 0.8,
        reason: "Move the nearby cosmetic hole away from the declared relief-window spacing envelope.",
        source_hint: {
          source_file_path: beforeDesign.source?.path ?? "design.py",
          edit_kind: "replace_python_tuple_entry",
          selector: `cosmetic_holes.${featureId}`,
          feature_id: featureId,
          parameter: "center_mm",
          value_path: `features[id=${featureId}].center_mm`,
          before_value_mm: beforeCenter,
          after_value_mm: afterCenter,
          before_text: beforeText,
          after_text: afterText,
          confidence: "exact_from_design_data",
          rationale:
            "The tuple entry directly emits the declared clearance-hole center checked against the relief spacing envelope.",
        },
      },
    ],
  };
}

function copyFixture(label) {
  const root = path.join(tmp, label);
  const dir = path.join(root, fixtureDir);
  fs.mkdirSync(dir, { recursive: true });
  copyRequired("rules/printed_plate.rulepack.json", path.join(root, "rules/printed_plate.rulepack.json"));
  copyRequired(path.join(fixtureDir, "design.py"), path.join(dir, "design.py"));
  return { root, dir };
}

function runGenerator(dir) {
  run("uv", ["run", "--package", "burr-build123d", "python", path.join(dir, "design.py")]);
  expectFile(path.join(dir, designDataFile), "generated design data");
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

function expectPassingFileHashes(receipt) {
  for (const ruleId of [
    "burr_design_data:source_sha256_matches",
    "burr_design_data:artifact_sha256_matches",
  ]) {
    expectCheck(receipt, ruleId, { status: "pass" });
  }
}

function expectCheck(receipt, ruleId, options = {}) {
  const check = receipt.checks.find((item) => item.rule_id === ruleId);
  if (!check) {
    throw new Error(`Missing check ${ruleId}`);
  }
  if (options.status) {
    expectEqual(check.status, options.status, `${ruleId} status`);
  }
  return check;
}

function expectClosestPair(check, expected) {
  const pair = check.measured?.closest_pair;
  expectArrayEqual(pair?.feature_ids, expected.featureIds, `${check.rule_id} closest pair ids`);
  expectArrayEqual(pair?.feature_shapes, expected.featureShapes, `${check.rule_id} closest pair shapes`);
  expectEqual(pair?.shape_distance_mm, expected.shapeDistanceMm, `${check.rule_id} shape distance`);
  expectEqual(pair?.clearance_mm, expected.clearanceMm, `${check.rule_id} clearance`);
}

function copyRequired(source, target) {
  if (!fs.existsSync(source)) {
    throw new Error(`Missing required file: ${source}`);
  }
  fs.mkdirSync(path.dirname(target), { recursive: true });
  fs.copyFileSync(source, target);
}

function readJson(file) {
  return JSON.parse(readText(file));
}

function readText(file) {
  if (!fs.existsSync(file)) {
    throw new Error(`Missing file: ${file}`);
  }
  return fs.readFileSync(file, "utf8");
}

function expectFile(file, label) {
  if (!fs.existsSync(file) || fs.statSync(file).size === 0) {
    throw new Error(`Missing ${label}: ${file}`);
  }
}

function expectOccursOnce(source, needle, label) {
  const first = source.indexOf(needle);
  if (first < 0) {
    throw new Error(`${label}: missing ${JSON.stringify(needle)}`);
  }
  const second = source.indexOf(needle, first + needle.length);
  if (second >= 0) {
    throw new Error(`${label}: expected one occurrence, found multiple`);
  }
}

function expectEqual(actual, expected, label) {
  if (actual !== expected) {
    throw new Error(`${label}: expected ${JSON.stringify(expected)}, got ${JSON.stringify(actual)}`);
  }
}

function expectArrayEqual(actual, expected, label) {
  if (!Array.isArray(actual) || actual.length !== expected.length || actual.some((value, index) => value !== expected[index])) {
    throw new Error(`${label}: expected ${JSON.stringify(expected)}, got ${JSON.stringify(actual)}`);
  }
}
