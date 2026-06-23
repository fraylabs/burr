#!/usr/bin/env node
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { buildRepairReports } from "./repair-report-builder.mjs";

const beforeSlug = "bad-actuator-housing-edge-distance";
const afterSlug = "fixed-actuator-housing";
const reportId = "actuator-housing-edge-distance";
const receiptFile = "burr-receipt.json";
const designDataFile = "burr-design-data.json";
const badSource = "examples/build123d-actuator-housing-repair/bad/design.py";
const fixedSource = "examples/build123d-actuator-housing-repair/fixed/design.py";
const packageJson = readJson("package.json");
const tmp = fs.mkdtempSync(path.join(os.tmpdir(), "burr-agent-repair-runner-"));

try {
  proveNotRepairableWithoutExactEdits();
  proveRepairWithExactPacket();
  console.log("agent repair runner proof passed");
} finally {
  fs.rmSync(tmp, { recursive: true, force: true });
}

function proveNotRepairableWithoutExactEdits() {
  const targetDir = path.join(tmp, "not-repairable");
  fs.mkdirSync(targetDir, { recursive: true });
  copyRequired(badSource, path.join(targetDir, "design.py"));
  const beforeSource = readText(path.join(targetDir, "design.py"));

  const runResult = run("node", ["scripts/agent-repair-runner.mjs", targetDir]);
  const runnerReceipt = JSON.parse(runResult.output);
  expectEqual(runnerReceipt.status, "not_repairable", "not-repairable runner status");
  expectEqual(runnerReceipt.before.status, "fail", "not-repairable before status");
  expectEqual(
    runnerReceipt.repair_packet.exact_source_edits_available,
    false,
    "not-repairable exact edit availability",
  );
  expectEqual(readText(path.join(targetDir, "design.py")), beforeSource, "not-repairable source unchanged");
  expectEqual(
    readJson(path.join(targetDir, "burr-agent-repair-receipt.json")).status,
    "not_repairable",
    "not-repairable receipt file status",
  );
}

function proveRepairWithExactPacket() {
  const beforeDir = path.join(tmp, "before");
  const afterDir = path.join(tmp, "after");
  const targetDir = path.join(tmp, "repairable");
  fs.mkdirSync(beforeDir, { recursive: true });
  fs.mkdirSync(afterDir, { recursive: true });
  fs.mkdirSync(targetDir, { recursive: true });
  copyRequired(badSource, path.join(beforeDir, "design.py"));
  copyRequired(fixedSource, path.join(afterDir, "design.py"));
  copyRequired(badSource, path.join(targetDir, "design.py"));

  runGenerator(beforeDir);
  runBurrCheck(beforeDir, { allowFailure: true });
  runGenerator(afterDir);
  runBurrCheck(afterDir);

  const report = generateRepairReport({ releaseDir: path.join(tmp, "release"), beforeDir, afterDir });
  expectEqual(report.report_id, reportId, "repair report id");
  const reportPath = path.join(tmp, "repair-report.json");
  fs.writeFileSync(reportPath, `${JSON.stringify(report, null, 2)}\n`);
  const repairPacket = run("cargo", ["run", "--quiet", "--", "explain", "--json", reportPath]);
  const packetPath = path.join(tmp, "repair-packet.json");
  fs.writeFileSync(packetPath, repairPacket.output);

  const runResult = run("node", [
    "scripts/agent-repair-runner.mjs",
    targetDir,
    "--packet",
    packetPath,
    "--source-file",
    "design.py",
  ]);
  const runnerReceipt = JSON.parse(runResult.output);
  expectEqual(runnerReceipt.status, "repaired", "repairable runner status");
  expectEqual(runnerReceipt.before.status, "fail", "repairable before status");
  expectEqual(runnerReceipt.after.status, "pass", "repairable after status");
  expectEqual(runnerReceipt.applied_edits.length, 5, "repairable applied edit count");

  const finalReceipt = readJson(path.join(targetDir, receiptFile));
  expectEqual(finalReceipt.status, "pass", "repairable final receipt status");
  expectPassingFileHashes(finalReceipt);
  const repairedSource = readText(path.join(targetDir, "design.py"));
  expectIncludes(repairedSource, "housing_width = 48.0", "repairable housing width edit");
  expectIncludes(
    repairedSource,
    '    "m3_front_left": (-22.0, -12.0, hole_z),',
    "repairable mount hole edit",
  );
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
  if (!options.allowFailure && result.status !== 0) {
    throw new Error(`${command} ${args.join(" ")} failed with exit ${result.status}\n${output}`);
  }
  return { ...result, output };
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

function expectIncludes(value, needle, label) {
  if (!value.includes(needle)) {
    throw new Error(`${label} did not include ${JSON.stringify(needle)}`);
  }
}

function expectEqual(actual, expected, label) {
  if (actual !== expected) {
    throw new Error(`${label}: expected ${JSON.stringify(expected)}, got ${JSON.stringify(actual)}`);
  }
}
