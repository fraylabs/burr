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

const beforeFailures = checksForRule(beforeReceipt, expected.ruleId).filter(
  (check) => check.status === "fail",
);
expectEqual(
  report.failures?.length,
  beforeFailures.length,
  "report failure count",
);
expectArrayEqual(
  report.failures.map(failureKey),
  beforeFailures.map(failureKey),
  "report failures",
);
expectRepairActions({
  actions: report.repair_actions,
  failures: beforeFailures,
  featureResults: report.feature_results ?? [],
  afterReceipt,
  ruleId: expected.ruleId,
});

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
  "Repair Actions",
  "move_feature",
  "center_mm",
  "[6, -4, 0] mm",
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

function checksForRule(receipt, ruleId) {
  return receipt.checks.filter((check) => check.rule_id === ruleId);
}

function expectRepairActions({ actions, failures, featureResults, afterReceipt, ruleId }) {
  if (!Array.isArray(actions)) {
    throw new Error("Repair report must include repair_actions array");
  }
  expectEqual(actions.length, failures.length, "repair action count");

  const failureByKey = new Map(failures.map((failure) => [failureKey(failure), failure]));
  const resultByKey = new Map(
    featureResults.map((result) => [
      failureKey({ rule_id: ruleId, feature_id: result.feature_id }),
      result,
    ]),
  );
  const seenActionKeys = new Set();

  for (const action of actions) {
    const key = failureKey(action);
    const failure = failureByKey.get(key);
    if (!failure) {
      throw new Error(`Repair action does not map to a before failure: ${key}`);
    }
    if (seenActionKeys.has(key)) {
      throw new Error(`Duplicate repair action for failure: ${key}`);
    }
    seenActionKeys.add(key);

    expectEqual(action.action, "move_feature", `repair action kind for ${key}`);
    expectEqual(action.parameter, "center_mm", `repair action parameter for ${key}`);
    expectNonEmptyString(action.reason, `repair action reason for ${key}`);
    expectEqual(action.failure_reason, failure.reason, `repair action failure reason for ${key}`);
    expectArrayEqual(
      action.suggested_delta_mm,
      vectorDelta(action.after_value_mm, action.before_value_mm),
      `repair action feature movement for ${key}`,
    );
    expectFailureSnapshot(action, failure, key);

    const featureResult = resultByKey.get(key);
    if (!featureResult) {
      throw new Error(`Missing feature result for repair action: ${key}`);
    }

    const afterCheck = requireCheck(afterReceipt, {
      status: "pass",
      ruleId,
      featureId: action.feature_id,
    });
    if (afterCheck.margin_mm <= 0) {
      throw new Error(
        `Fixed receipt after margin must be positive for ${key}: ${afterCheck.margin_mm}`,
      );
    }

    const expectedMeasuredDelta = roundMm(
      (afterCheck.measured?.center_to_edge_mm ?? 0) -
        (failure.measured?.center_to_edge_mm ?? 0),
    );
    const expectedMarginDelta = roundMm(
      (afterCheck.margin_mm ?? 0) - (failure.margin_mm ?? 0),
    );
    const actionDelta = repairActionDelta(action);
    expectSuggestedDelta(action, failure, key);
    expectAfterProof(action, afterCheck, key);

    expectEqual(
      actionDelta.measured_delta_mm,
      featureResult.measured_delta_mm,
      `repair action measured delta matches feature result for ${key}`,
    );
    expectEqual(
      actionDelta.margin_delta_mm,
      featureResult.margin_delta_mm,
      `repair action margin delta matches feature result for ${key}`,
    );
    expectEqual(
      actionDelta.measured_delta_mm,
      expectedMeasuredDelta,
      `repair action measured delta matches receipts for ${key}`,
    );
    expectEqual(
      actionDelta.margin_delta_mm,
      expectedMarginDelta,
      `repair action margin delta matches receipts for ${key}`,
    );
  }

  expectArrayEqual([...seenActionKeys].sort(), [...failureByKey.keys()].sort(), "repair action keys");
}

function repairActionDelta(action) {
  const afterProof = action.verifies_against_after_feature ?? action.after;
  const measuredDelta =
    action.measured_delta_mm ??
    action.delta?.measured_delta_mm ??
    action.delta?.center_to_edge_mm ??
    action.delta?.measured?.center_to_edge_mm ??
    roundOptionalDelta(
      afterProof?.measured?.center_to_edge_mm,
      action.measured?.center_to_edge_mm,
    );
  const marginDelta =
    action.margin_delta_mm ??
    action.delta?.margin_delta_mm ??
    action.delta?.margin_mm ??
    roundOptionalDelta(afterProof?.margin_mm, action.margin_mm ?? action.margin);

  expectFiniteNumber(measuredDelta, `repair action measured delta for ${failureKey(action)}`);
  expectFiniteNumber(marginDelta, `repair action margin delta for ${failureKey(action)}`);

  return {
    measured_delta_mm: roundMm(measuredDelta),
    margin_delta_mm: roundMm(marginDelta),
  };
}

function expectFailureSnapshot(action, failure, key) {
  if (action.measured !== undefined) {
    expectEqual(
      action.measured.center_to_edge_mm,
      failure.measured?.center_to_edge_mm,
      `repair action measured center-to-edge for ${key}`,
    );
  }
  if (action.required !== undefined) {
    expectEqual(
      action.required.center_to_edge_mm,
      failure.required?.center_to_edge_mm,
      `repair action required center-to-edge for ${key}`,
    );
  }
  if (action.margin_mm !== undefined || action.margin !== undefined) {
    expectEqual(action.margin_mm ?? action.margin, failure.margin_mm, `repair action margin for ${key}`);
  }
}

function expectSuggestedDelta(action, failure, key) {
  if (action.suggested_delta_mm === undefined) {
    throw new Error(`Repair action missing suggested_delta_mm for ${key}`);
  }
  expectArrayEqual(
    action.suggested_delta_mm,
    vectorDelta(action.after_value_mm, action.before_value_mm),
    `repair action suggested delta for ${key}`,
  );
}

function expectAfterProof(action, afterCheck, key) {
  const afterProof = action.verifies_against_after_feature ?? action.after;
  if (afterProof === undefined) {
    return;
  }
  expectEqual(afterProof.feature_id, afterCheck.feature_id, `repair action after proof feature for ${key}`);
  expectEqual(afterProof.status, "pass", `repair action after proof status for ${key}`);
  expectEqual(afterProof.margin_mm ?? afterProof.margin, afterCheck.margin_mm, `repair action after proof margin for ${key}`);
  expectEqual(
    afterProof.measured?.center_to_edge_mm,
    afterCheck.measured?.center_to_edge_mm,
    `repair action after proof measured center-to-edge for ${key}`,
  );
  if ((afterProof.margin_mm ?? afterProof.margin) <= 0) {
    throw new Error(`Repair action after proof margin must be positive for ${key}`);
  }
}

function roundOptionalDelta(after, before) {
  if (Number.isFinite(after) && Number.isFinite(before)) {
    return roundMm(after - before);
  }
  return undefined;
}

function failureKey(value) {
  return `${value.rule_id}:${value.feature_id}`;
}

function roundMm(value) {
  return Math.round(value * 1000) / 1000;
}

function vectorDelta(after, before) {
  if (!Array.isArray(after) || !Array.isArray(before) || after.length !== before.length) {
    throw new Error("Repair action before/after values must be same-length arrays");
  }
  return after.map((value, index) => roundMm(value - before[index]));
}

function expectEqual(actual, expectedValue, label) {
  if (actual !== expectedValue) {
    throw new Error(
      `Unexpected ${label}: got ${JSON.stringify(actual)}, expected ${JSON.stringify(expectedValue)}`,
    );
  }
}

function expectArrayEqual(actual, expectedValue, label) {
  expectEqual(JSON.stringify(actual), JSON.stringify(expectedValue), label);
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

function expectFiniteNumber(value, label) {
  if (typeof value !== "number" || !Number.isFinite(value)) {
    throw new Error(`Expected finite ${label}; got ${JSON.stringify(value)}`);
  }
}
