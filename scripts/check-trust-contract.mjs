#!/usr/bin/env node
import crypto from "node:crypto";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";

const repoRoot = process.cwd();
const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), "burr-trust-contract-"));
const receiptPaths = [];

const m3Insert = {
  id: "m3_insert",
  part: "fixture",
  kind: "heat_set_insert_pocket",
  intent: "mechanical_interface",
  role: "threaded_mount",
  insert: "M3x5.7",
};

const m4Insert = {
  ...m3Insert,
  id: "m4_insert",
  insert: "M4x8.1",
};

const m3InventoryRule = {
  id: "m3_insert_inventory",
  kind: "feature_count",
  applies_to: {
    kind: "heat_set_insert_pocket",
    insert: "M3x5.7",
    intent_any: ["mechanical_interface"],
  },
  min_count: 1,
  max_count: 1,
};

try {
  checkExplicitRulepackPass();
  checkMissingRulepackSelection();
  checkArtifactTypeMismatch();
  checkProcessKindMismatch();
  checkUnsupportedRuleKind();
  checkUnknownSelectorKey();
  checkDuplicateRuleIds();
  checkInvalidDesignDataContract();
  checkZeroApplicableMechanicalCoverage();
  checkUncheckedMechanicalFeature();
  checkUnknownIntentRequiresCoverage();
  checkNonMechanicalUncheckedFeature();
  checkInsertSelectorIsolation();
  checkSinglePairSpacingCandidate();
  checkMultiTargetReceiptWritesAreAtomic();
  validateRuntimeReceipts();
  console.log("trust contract fixtures passed");
} finally {
  fs.rmSync(tempRoot, { recursive: true, force: true });
}

function checkExplicitRulepackPass() {
  const fixture = writeFixture("explicit-good-rulepack");
  const result = runCheck(fixture.dir);
  const receipt = expectReceipt(fixture, result, { exitCode: 0, outcome: "pass" });
  expectReasonAbsent(receipt, "no_evaluated_mechanical_rules", fixture.slug);
  expectReasonAbsent(receipt, "unchecked_mechanical_features", fixture.slug);
  expectCoverage(receipt, {
    declared: 1,
    checked: 1,
    unchecked: 0,
    checkedIds: ["m3_insert"],
    uncheckedIds: [],
  }, fixture.slug);
}

function checkMissingRulepackSelection() {
  const fixture = writeFixture("missing-rulepack-selection", { includeRulepackSelection: false });
  const result = runCheck(fixture.dir);
  expectExitCode(result, 2, fixture.slug);
  expectIncludes(result.output, "No rulepack selected for", fixture.slug);
  expectIncludes(result.output, "Pass --rulepack <file> or add rulepack.path", fixture.slug);
  if (fs.existsSync(fixture.receiptPath)) {
    throw new Error(`${fixture.slug} must not write a receipt for an invocation/configuration error`);
  }
}

function checkArtifactTypeMismatch() {
  const fixture = writeFixture("artifact-type-mismatch", {
    designData: { artifact_type: "other_part" },
  });
  const result = runCheck(fixture.dir);
  const receipt = expectReceipt(fixture, result, { exitCode: 3, outcome: "incomplete" });
  expectReason(receipt, "artifact_type_not_targeted", fixture.slug);
  expectCoverage(receipt, {
    declared: 1,
    checked: 0,
    unchecked: 1,
    checkedIds: [],
    uncheckedIds: ["m3_insert"],
  }, fixture.slug);
  expectIncludes(result.output, "Warnings:", fixture.slug);
  expectIncludes(result.output, "artifact_type_not_targeted [incomplete]", fixture.slug);

  const explanation = runExplainJson(fixture.receiptPath);
  expectExitCode(explanation, 0, `${fixture.slug} explain`);
  const packet = JSON.parse(explanation.stdout);
  expectEqual(packet.outcome, "incomplete", `${fixture.slug} explain outcome`);
  expectEqual(
    packet.scope?.artifact_type?.compatible,
    false,
    `${fixture.slug} explain artifact scope`,
  );
  if (!packet.incomplete_reasons?.some((reason) => reason.reason === "artifact_type_not_targeted")) {
    throw new Error(`${fixture.slug} explain packet omitted artifact_type_not_targeted`);
  }
}

function checkProcessKindMismatch() {
  const fixture = writeFixture("process-kind-mismatch", {
    designData: { process: { kind: "CNC" } },
  });
  const result = runCheck(fixture.dir);
  const receipt = expectReceipt(fixture, result, { exitCode: 3, outcome: "incomplete" });
  expectReason(receipt, "process_kind_not_targeted", fixture.slug);
  expectCoverage(receipt, {
    declared: 1,
    checked: 0,
    unchecked: 1,
    checkedIds: [],
    uncheckedIds: ["m3_insert"],
  }, fixture.slug);
}

function checkUnsupportedRuleKind() {
  const fixture = writeFixture("unsupported-rule-kind", {
    rules: [{ id: "invented_rule", kind: "universal_cad_brain" }],
  });
  const result = runCheck(fixture.dir);
  const receipt = expectReceipt(fixture, result, { exitCode: 1, outcome: "fail" });
  expectContractFailure(receipt, "unsupported_rule_kind", fixture.slug);
  expectCoverage(receipt, {
    declared: 1,
    checked: 0,
    unchecked: 1,
    checkedIds: [],
    uncheckedIds: ["m3_insert"],
  }, fixture.slug);
}

function checkUnknownSelectorKey() {
  const fixture = writeFixture("unknown-selector-key", {
    rules: [
      {
        ...m3InventoryRule,
        applies_to: {
          ...m3InventoryRule.applies_to,
          fastner: "M3",
        },
      },
    ],
  });
  const result = runCheck(fixture.dir);
  const receipt = expectReceipt(fixture, result, { exitCode: 1, outcome: "fail" });
  expectContractFailure(receipt, "unknown_applies_to_selector", fixture.slug);
  expectCoverage(receipt, {
    declared: 1,
    checked: 0,
    unchecked: 1,
    checkedIds: [],
    uncheckedIds: ["m3_insert"],
  }, fixture.slug);
}

function checkDuplicateRuleIds() {
  const fixture = writeFixture("duplicate-rule-ids", {
    rules: [m3InventoryRule, { ...m3InventoryRule }],
  });
  const result = runCheck(fixture.dir);
  const receipt = expectReceipt(fixture, result, { exitCode: 1, outcome: "fail" });
  expectContractFailure(receipt, "duplicate_rule_id", fixture.slug);
  expectCoverage(receipt, {
    declared: 1,
    checked: 0,
    unchecked: 1,
    checkedIds: [],
    uncheckedIds: ["m3_insert"],
  }, fixture.slug);
}

function checkInvalidDesignDataContract() {
  const fixture = writeFixture("invalid-design-data-contract", {
    designData: { artifact_id: "" },
  });
  const result = runCheck(fixture.dir);
  const receipt = expectReceipt(fixture, result, { exitCode: 1, outcome: "fail" });
  const check = receipt.checks?.find(
    (candidate) =>
      candidate.rule_id === "burr_design_data:contract_valid" &&
      candidate.reason === "missing_artifact_id",
  );
  if (!check) {
    throw new Error(`${fixture.slug} did not fail the design-data contract`);
  }
  expectEqual(check.status, "fail", `${fixture.slug} design-data contract status`);
  expectReasonAbsent(receipt, "unchecked_mechanical_features", fixture.slug);
}

function checkZeroApplicableMechanicalCoverage() {
  const fixture = writeFixture("zero-applicable-mechanical-coverage", {
    rules: [
      {
        id: "clearance_hole_wall",
        kind: "minimum_wall_thickness",
        applies_to: {
          kind: "clearance_hole",
          intent_any: ["mechanical_interface"],
        },
        min_wall_thickness_mm: 2,
      },
    ],
  });
  const result = runCheck(fixture.dir);
  const receipt = expectReceipt(fixture, result, { exitCode: 3, outcome: "incomplete" });
  expectReason(receipt, "no_applicable_features", fixture.slug);
  expectReason(receipt, "no_evaluated_mechanical_rules", fixture.slug);
  expectReason(receipt, "unchecked_mechanical_features", fixture.slug);
  expectCoverage(receipt, {
    declared: 1,
    checked: 0,
    unchecked: 1,
    checkedIds: [],
    uncheckedIds: ["m3_insert"],
  }, fixture.slug);
}

function checkUncheckedMechanicalFeature() {
  const fixture = writeFixture("unchecked-mechanical-feature", {
    features: [
      m3Insert,
      {
        id: "uncovered_mount_hole",
        part: "fixture",
        kind: "clearance_hole",
        intent: "mechanical_interface",
        role: "mount",
        fastener: "M3",
      },
    ],
  });
  const result = runCheck(fixture.dir);
  const receipt = expectReceipt(fixture, result, { exitCode: 3, outcome: "incomplete" });
  expectReason(receipt, "unchecked_mechanical_features", fixture.slug);
  expectReasonAbsent(receipt, "no_evaluated_mechanical_rules", fixture.slug);
  expectCoverage(receipt, {
    declared: 2,
    checked: 1,
    unchecked: 1,
    checkedIds: ["m3_insert"],
    uncheckedIds: ["uncovered_mount_hole"],
  }, fixture.slug);
}

function checkUnknownIntentRequiresCoverage() {
  const fixture = writeFixture("unknown-intent-requires-coverage", {
    features: [
      m3Insert,
      {
        id: "typoed_mount_hole",
        part: "fixture",
        kind: "clearance_hole",
        intent: "mechanical-interface",
        role: "mount",
        fastener: "M3",
      },
    ],
  });
  const result = runCheck(fixture.dir);
  const receipt = expectReceipt(fixture, result, { exitCode: 3, outcome: "incomplete" });
  expectReason(receipt, "unchecked_mechanical_features", fixture.slug);
  expectCoverage(receipt, {
    declared: 2,
    checked: 1,
    unchecked: 1,
    checkedIds: ["m3_insert"],
    uncheckedIds: ["typoed_mount_hole"],
  }, fixture.slug);
}

function checkNonMechanicalUncheckedFeature() {
  const fixture = writeFixture("non-mechanical-unchecked-feature", {
    features: [
      m3Insert,
      {
        id: "cosmetic_vent",
        part: "fixture",
        kind: "cutout",
        intent: "cosmetic",
        role: "visual_detail",
      },
    ],
  });
  const result = runCheck(fixture.dir);
  const receipt = expectReceipt(fixture, result, { exitCode: 0, outcome: "pass" });
  expectReasonAbsent(receipt, "unchecked_mechanical_features", fixture.slug);
  expectCoverage(receipt, {
    declared: 2,
    checked: 1,
    unchecked: 1,
    checkedIds: ["m3_insert"],
    uncheckedIds: ["cosmetic_vent"],
    mechanical: {
      declared: 1,
      checked: 1,
      unchecked: 0,
      checkedIds: ["m3_insert"],
      uncheckedIds: [],
    },
  }, fixture.slug);
}

function checkInsertSelectorIsolation() {
  const fixture = writeFixture("m3-selector-does-not-match-m4", {
    features: [m4Insert],
  });
  const result = runCheck(fixture.dir);
  const receipt = expectReceipt(fixture, result, { exitCode: 1, outcome: "fail" });
  const check = expectCheck(receipt, "trust_contract:m3_insert_inventory", fixture.slug);
  expectEqual(check.status, "fail", `${fixture.slug} selector check status`);
  expectEqual(check.reason, "feature_count_out_of_range", `${fixture.slug} selector check reason`);
  expectEqual(check.measured?.count, 0, `${fixture.slug} selected M3 feature count`);
  expectArrayEqual(check.feature_ids, [], `${fixture.slug} selected M3 feature ids`);
  expectCoverage(receipt, {
    declared: 1,
    checked: 0,
    unchecked: 1,
    checkedIds: [],
    uncheckedIds: ["m4_insert"],
  }, fixture.slug);
}

function checkSinglePairSpacingCandidate() {
  const fixture = writeFixture("single-pair-spacing-candidate", {
    features: [
      {
        id: "only_hole",
        part: "fixture",
        kind: "clearance_hole",
        intent: "mechanical_interface",
        role: "mount",
        fastener: "M3",
        diameter_mm: 3.4,
        center_mm: [0, 0, 0],
        axis: [0, 0, 1],
      },
    ],
    rules: [
      {
        id: "mount_hole_spacing",
        kind: "feature_pair_spacing",
        applies_to: {
          kind: "clearance_hole",
          fastener: "M3",
          intent_any: ["mechanical_interface"],
        },
        min_clearance_mm: 2,
      },
    ],
  });
  const result = runCheck(fixture.dir);
  const receipt = expectReceipt(fixture, result, { exitCode: 3, outcome: "incomplete" });
  const check = expectCheck(receipt, "trust_contract:mount_hole_spacing", fixture.slug);
  expectEqual(check.status, "incomplete", `${fixture.slug} pair-spacing status`);
  expectEqual(check.reason, "insufficient_pair_spacing_candidates", `${fixture.slug} pair-spacing reason`);
  expectEqual(check.measured?.pair_count, 0, `${fixture.slug} pair count`);
  expectArrayEqual(check.feature_ids, ["only_hole"], `${fixture.slug} pair-spacing candidates`);
  expectCoverage(receipt, {
    declared: 1,
    checked: 1,
    unchecked: 0,
    checkedIds: ["only_hole"],
    uncheckedIds: [],
  }, fixture.slug);
}

function checkMultiTargetReceiptWritesAreAtomic() {
  const valid = writeFixture("multi-target-valid");
  const invalid = writeFixture("multi-target-missing-selection", {
    includeRulepackSelection: false,
  });
  const result = runCheckInputs([valid.dir, invalid.dir]);
  expectExitCode(result, 2, "multi-target atomic receipts");
  expectIncludes(result.output, "No rulepack selected for", "multi-target atomic receipts");
  for (const fixture of [valid, invalid]) {
    if (fs.existsSync(fixture.receiptPath)) {
      throw new Error(`${fixture.slug} wrote a partial receipt before multi-target preflight passed`);
    }
  }
}

function writeFixture(slug, options = {}) {
  const dir = path.join(tempRoot, slug);
  fs.mkdirSync(dir, { recursive: true });

  const sourcePath = path.join(dir, "design.py");
  const stepPath = path.join(dir, "part.step");
  const designDataPath = path.join(dir, "burr-design-data.json");
  const receiptPath = path.join(dir, "burr-receipt.json");
  const rulepackPath = path.join(dir, "rulepack.json");

  fs.writeFileSync(sourcePath, "print('isolated trust-contract fixture')\n");
  fs.writeFileSync(stepPath, "ISO-10303-21;\nEND-ISO-10303-21;\n");

  const designData = {
    schema_version: "burr.design-data.v1",
    artifact_id: slug,
    artifact_version: "0.1.0",
    artifact_type: "trust_part",
    units: "mm",
    process: { kind: "FDM" },
    source: fileRef(sourcePath, "design.py"),
    artifacts: [{ kind: "step", ...fileRef(stepPath, "part.step") }],
    features: options.features ?? [m3Insert],
    ...options.designData,
  };

  if (options.includeRulepackSelection !== false) {
    designData.rulepack = { path: "rulepack.json" };
    const rulepack = {
      schema_version: "burr.rulepack.v1",
      id: "trust_contract",
      version: "0.1.0",
      artifact_type: "trust_part",
      process_kind: "FDM",
      rules: options.rules ?? [m3InventoryRule],
      ...options.rulepack,
    };
    writeJson(rulepackPath, rulepack);
  }

  writeJson(designDataPath, designData);
  return { slug, dir, receiptPath };
}

function runCheck(dir) {
  return runCheckInputs([dir]);
}

function runCheckInputs(inputs) {
  const result = spawnSync("cargo", ["run", "--quiet", "--", "check", ...inputs], {
    cwd: repoRoot,
    encoding: "utf8",
    maxBuffer: 1024 * 1024 * 32,
  });
  return {
    ...result,
    output: [result.stdout, result.stderr].filter(Boolean).join("\n"),
  };
}

function runExplainJson(receiptPath) {
  const result = spawnSync("cargo", ["run", "--quiet", "--", "explain", "--json", receiptPath], {
    cwd: repoRoot,
    encoding: "utf8",
    maxBuffer: 1024 * 1024 * 32,
  });
  return {
    ...result,
    output: [result.stdout, result.stderr].filter(Boolean).join("\n"),
  };
}

function expectReceipt(fixture, result, expected) {
  expectExitCode(result, expected.exitCode, fixture.slug);
  if (!fs.existsSync(fixture.receiptPath)) {
    throw new Error(`${fixture.slug} did not write ${fixture.receiptPath}\n${result.output}`);
  }
  const receipt = JSON.parse(fs.readFileSync(fixture.receiptPath, "utf8"));
  expectEqual(receipt.status, expected.outcome, `${fixture.slug} receipt status`);
  expectEqual(receipt.outcome, expected.outcome, `${fixture.slug} receipt outcome`);
  receiptPaths.push(fixture.receiptPath);
  return receipt;
}

function validateRuntimeReceipts() {
  const outcomes = new Set(
    receiptPaths.map((receiptPath) => JSON.parse(fs.readFileSync(receiptPath, "utf8")).outcome),
  );
  for (const outcome of ["pass", "incomplete", "fail"]) {
    if (!outcomes.has(outcome)) {
      throw new Error(`runtime receipt fixtures do not cover ${outcome}`);
    }
  }

  const result = spawnSync(
    "uvx",
    [
      "--from",
      "check-jsonschema==0.38.0",
      "check-jsonschema",
      "--schemafile",
      "schemas/burr.receipt.v2.schema.json",
      ...receiptPaths,
    ],
    {
      cwd: repoRoot,
      encoding: "utf8",
      env: { ...process.env, UV_NO_PROGRESS: "1" },
      maxBuffer: 1024 * 1024 * 32,
    },
  );
  if (result.error) {
    throw new Error(`failed to validate runtime receipts: ${result.error.message}`);
  }
  if (result.status !== 0) {
    throw new Error(
      `runtime receipt schema validation failed with exit ${result.status}\n${result.stdout}${result.stderr}`,
    );
  }
}

function expectContractFailure(receipt, reason, slug) {
  const check = receipt.checks?.find(
    (candidate) => candidate.rule_id === "burr_rulepack:contract_valid" && candidate.reason === reason,
  );
  if (!check) {
    throw new Error(`${slug} did not report burr_rulepack:contract_valid ${reason}`);
  }
  expectEqual(check.status, "fail", `${slug} ${reason} status`);
}

function expectCheck(receipt, ruleId, slug) {
  const check = receipt.checks?.find((candidate) => candidate.rule_id === ruleId);
  if (!check) {
    throw new Error(`${slug} did not report ${ruleId}`);
  }
  return check;
}

function expectReason(receipt, reason, slug) {
  if (!diagnostics(receipt).some((diagnostic) => diagnostic.reason === reason)) {
    throw new Error(`${slug} did not report ${reason}`);
  }
}

function expectReasonAbsent(receipt, reason, slug) {
  if (diagnostics(receipt).some((diagnostic) => diagnostic.reason === reason)) {
    throw new Error(`${slug} unexpectedly reported ${reason}`);
  }
}

function diagnostics(receipt) {
  return [...(receipt.checks ?? []), ...(receipt.warnings ?? [])];
}

function expectCoverage(receipt, expected, slug) {
  const coverage = receipt.summary?.features;
  if (!coverage) {
    throw new Error(`${slug} receipt is missing summary.features`);
  }
  expectEqual(coverage.declared, expected.declared, `${slug} declared feature count`);
  expectEqual(coverage.checked, expected.checked, `${slug} checked feature count`);
  expectEqual(coverage.unchecked, expected.unchecked, `${slug} unchecked feature count`);
  expectArrayEqual(coverage.checked_feature_ids, expected.checkedIds, `${slug} checked feature ids`);
  expectArrayEqual(coverage.unchecked_feature_ids, expected.uncheckedIds, `${slug} unchecked feature ids`);

  const mechanical = expected.mechanical ?? expected;
  expectEqual(
    coverage.mechanical_declared,
    mechanical.declared,
    `${slug} declared mechanical feature count`,
  );
  expectEqual(
    coverage.mechanical_checked,
    mechanical.checked,
    `${slug} checked mechanical feature count`,
  );
  expectEqual(
    coverage.mechanical_unchecked,
    mechanical.unchecked,
    `${slug} unchecked mechanical feature count`,
  );
  expectArrayEqual(
    coverage.checked_mechanical_feature_ids,
    mechanical.checkedIds,
    `${slug} checked mechanical feature ids`,
  );
  expectArrayEqual(
    coverage.unchecked_mechanical_feature_ids,
    mechanical.uncheckedIds,
    `${slug} unchecked mechanical feature ids`,
  );
}

function expectExitCode(result, expected, slug) {
  if (result.status !== expected) {
    throw new Error(`${slug} exited ${result.status}, expected ${expected}\n${result.output}`);
  }
}

function expectIncludes(output, expected, slug) {
  if (!output.includes(expected)) {
    throw new Error(`${slug} output did not include ${JSON.stringify(expected)}\n${output}`);
  }
}

function fileRef(filePath, label) {
  return {
    path: label,
    sha256: sha256(filePath),
    size_bytes: fs.statSync(filePath).size,
  };
}

function sha256(filePath) {
  return crypto.createHash("sha256").update(fs.readFileSync(filePath)).digest("hex");
}

function writeJson(filePath, value) {
  fs.writeFileSync(filePath, `${JSON.stringify(value, null, 2)}\n`);
}

function expectEqual(actual, expected, label) {
  if (actual !== expected) {
    throw new Error(`${label}: got ${JSON.stringify(actual)}, expected ${JSON.stringify(expected)}`);
  }
}

function expectArrayEqual(actual, expected, label) {
  if (
    !Array.isArray(actual) ||
    actual.length !== expected.length ||
    actual.some((value, index) => value !== expected[index])
  ) {
    throw new Error(`${label}: got ${JSON.stringify(actual)}, expected ${JSON.stringify(expected)}`);
  }
}
