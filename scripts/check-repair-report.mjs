#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";

const packageJson = readJson("package.json");
const artifactSlug = `burr-gallery-v${packageJson.version}`;
const releaseDir = path.join("artifacts", "releases", artifactSlug);
const expected = {
  reportId: "actuator-housing-edge-distance",
  beforeSlug: "bad-actuator-housing-edge-distance",
  afterSlug: "fixed-actuator-housing",
  ruleId: "actuator_mount:m3_loaded_hole_edge_distance",
  featureId: "m3_front_left",
};

run("node", ["scripts/build-gallery-artifact.mjs"]);

const manifest = readJson(path.join(releaseDir, "manifest.json"));
const reportEntry = (manifest.repair_reports ?? []).find(
  (entry) => entry.id === expected.reportId,
);
if (!reportEntry) {
  throw new Error(`Missing repair report manifest entry: ${expected.reportId}`);
}
expectEqual(
  reportEntry.before_slug ?? reportEntry.before_example,
  expected.beforeSlug,
  "report before slug",
);
expectEqual(
  reportEntry.after_slug ?? reportEntry.after_example,
  expected.afterSlug,
  "report after slug",
);
expectOneOf(reportEntry.status, ["pass", "repaired"], "report status");

const reportJsonPath = path.join(releaseDir, reportEntry.report_json);
const reportMarkdownPath = path.join(releaseDir, reportEntry.report_markdown);
expectFile(reportJsonPath, "repair report JSON");
expectFile(reportMarkdownPath, "repair report Markdown");

const report = readJson(reportJsonPath);
const markdown = fs.readFileSync(reportMarkdownPath, "utf8");

expectEqual(report.schema_version, "burr.repair-report.v1", "report schema");
expectEqual(report.report_id ?? report.id, expected.reportId, "report id");
expectOneOf(report.status, ["pass", "repaired"], "report JSON status");
expectEqual(
  report.before.slug ?? report.before.example_slug,
  expected.beforeSlug,
  "report JSON before slug",
);
expectEqual(
  report.after.slug ?? report.after.example_slug,
  expected.afterSlug,
  "report JSON after slug",
);
expectEqual(report.before.status, "fail", "before status");
expectEqual(report.after.status, "pass", "after status");
expectNonEmptyString(report.first_fix, "report first fix");

const beforeReceiptPath = path.join(releaseDir, report.before.receipt);
const beforeDesignPath = path.join(releaseDir, report.before.design_data);
const afterReceiptPath = path.join(releaseDir, report.after.receipt);
const afterDesignPath = path.join(releaseDir, report.after.design_data);
expectFile(beforeReceiptPath, "bad actuator receipt");
expectFile(beforeDesignPath, "bad actuator design data");
expectFile(afterReceiptPath, "fixed actuator receipt");
expectFile(afterDesignPath, "fixed actuator design data");

const beforeReceipt = readJson(beforeReceiptPath);
const beforeDesign = readJson(beforeDesignPath);
const afterReceipt = readJson(afterReceiptPath);
const afterDesign = readJson(afterDesignPath);

expectEqual(beforeReceipt.status, "fail", "bad actuator receipt status");
expectEqual(afterReceipt.status, "pass", "fixed actuator receipt status");
expectIncludes(beforeDesign.artifact_id, "repair-bad", "bad design artifact id");
expectIncludes(afterDesign.artifact_id, "repair-fixed", "fixed design artifact id");

const receiptFailure = requireCheck(beforeReceipt, {
  status: "fail",
  ruleId: expected.ruleId,
  featureId: expected.featureId,
});
const reportFailure = (report.failures ?? []).find(
  (failure) =>
    failure.rule_id === expected.ruleId &&
    failure.feature_id === expected.featureId,
);
if (!reportFailure) {
  throw new Error(`Report missing failing feature: ${expected.featureId}`);
}
expectEqual(reportFailure.reason, receiptFailure.reason, "failure reason");
expectEqual(
  reportFailure.measured.center_to_edge_mm,
  receiptFailure.measured.center_to_edge_mm,
  "failure measured center-to-edge",
);
expectEqual(
  reportFailure.required.center_to_edge_mm,
  receiptFailure.required.center_to_edge_mm,
  "failure required center-to-edge",
);
expectEqual(reportFailure.margin_mm, receiptFailure.margin_mm, "failure margin");
expectEqual(reportFailure.first_fix, report.first_fix, "first fix");

const receiptAfterPass = requireCheck(afterReceipt, {
  status: "pass",
  ruleId: expected.ruleId,
  featureId: expected.featureId,
});
const reportAfterPass = (report.after.proof_checks ?? []).find(
  (check) =>
    check.rule_id === expected.ruleId &&
    check.feature_id === expected.featureId,
)
  ?? (report.feature_results ?? []).find(
    (result) => result.feature_id === expected.featureId,
  )?.after;
if (!reportAfterPass) {
  throw new Error(`Report missing after pass for feature: ${expected.featureId}`);
}
expectEqual(reportAfterPass.reason, receiptAfterPass.reason, "after pass reason");
expectEqual(
  reportAfterPass.measured.center_to_edge_mm,
  receiptAfterPass.measured.center_to_edge_mm,
  "after pass measured center-to-edge",
);
expectEqual(
  reportAfterPass.required.center_to_edge_mm,
  receiptAfterPass.required.center_to_edge_mm,
  "after pass required center-to-edge",
);
expectEqual(reportAfterPass.margin_mm, receiptAfterPass.margin_mm, "after pass margin");
if (reportAfterPass.margin_mm <= 0) {
  throw new Error(`After pass margin must be positive: ${reportAfterPass.margin_mm}`);
}

for (const expectedText of [
  report.before.receipt,
  report.before.design_data,
  report.after.receipt,
  report.after.design_data,
  expected.featureId,
  String(reportFailure.measured.center_to_edge_mm),
  String(reportFailure.required.center_to_edge_mm),
  String(reportFailure.margin_mm),
  report.first_fix,
  "After Pass",
  String(reportAfterPass.measured.center_to_edge_mm),
  String(reportAfterPass.required.center_to_edge_mm),
  String(reportAfterPass.margin_mm),
]) {
  expectMarkdownIncludes(markdown, expectedText);
}

console.log("repair report proof passed");

function run(command, args) {
  const result = spawnSync(command, args, {
    encoding: "utf8",
    maxBuffer: 1024 * 1024 * 32,
  });
  const output = [result.stdout, result.stderr].filter(Boolean).join("\n");
  if (result.status !== 0) {
    throw new Error(
      `${command} ${args.join(" ")} failed with exit ${result.status}\n${output}`,
    );
  }
  return output;
}

function readJson(file) {
  return JSON.parse(fs.readFileSync(file, "utf8"));
}

function expectFile(file, label) {
  if (!fs.existsSync(file)) {
    throw new Error(`Missing ${label}: ${file}`);
  }
  if (fs.statSync(file).size < 64) {
    throw new Error(`${label} is too small: ${file}`);
  }
}

function requireCheck(receipt, { status, ruleId, featureId }) {
  const check = receipt.checks.find(
    (item) =>
      item.status === status &&
      item.rule_id === ruleId &&
      item.feature_id === featureId,
  );
  if (!check) {
    throw new Error(`Missing ${status} ${ruleId} check for ${featureId}`);
  }
  return check;
}

function expectEqual(actual, expectedValue, label) {
  if (actual !== expectedValue) {
    throw new Error(
      `Unexpected ${label}: got ${JSON.stringify(actual)}, expected ${JSON.stringify(expectedValue)}`,
    );
  }
}

function expectOneOf(actual, expectedValues, label) {
  if (!expectedValues.includes(actual)) {
    throw new Error(
      `Unexpected ${label}: got ${JSON.stringify(actual)}, expected one of ${JSON.stringify(expectedValues)}`,
    );
  }
}

function expectIncludes(value, expectedText, label) {
  if (typeof value !== "string" || !value.includes(expectedText)) {
    throw new Error(
      `Expected ${label} to include ${JSON.stringify(expectedText)}; got ${JSON.stringify(value)}`,
    );
  }
}

function expectMarkdownIncludes(markdown, expectedText) {
  if (!markdown.includes(expectedText)) {
    throw new Error(
      `Expected Markdown report to include ${JSON.stringify(expectedText)}.\n${markdown}`,
    );
  }
}

function expectNonEmptyString(value, label) {
  if (typeof value !== "string" || value.trim() === "") {
    throw new Error(`Expected non-empty ${label}`);
  }
}
