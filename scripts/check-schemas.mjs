#!/usr/bin/env node
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";

const CHECK_JSONSCHEMA_VERSION = "0.38.0";
const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const schemaDir = path.join(repoRoot, "schemas");

const schemaFiles = fs
  .readdirSync(schemaDir)
  .filter((name) => name.endsWith(".schema.json"))
  .sort()
  .map((name) => path.join("schemas", name));

const rulepackFiles = fs
  .readdirSync(path.join(repoRoot, "rules"))
  .filter((name) => name.endsWith(".rulepack.json"))
  .sort()
  .map((name) => path.join("rules", name));

const trackedJsonFiles = gitTrackedFiles().filter((name) => name.endsWith(".json"));
const designDataFiles = trackedJsonFiles
  .filter((name) => path.basename(name) === "burr-design-data.json")
  .sort();
const receiptFiles = trackedJsonFiles
  .filter((name) => path.basename(name) === "burr-receipt.json")
  .sort();

assertFiles("JSON Schemas", schemaFiles);
assertFiles("rulepacks", rulepackFiles);
assertFiles("tracked design-data instances", designDataFiles);

runCheck(["--check-metaschema", ...schemaFiles], "JSON Schema metaschemas");
runCheck(
  ["--schemafile", "schemas/burr.rulepack.v1.schema.json", ...rulepackFiles],
  "shipped rulepacks",
);
runCheck(
  ["--schemafile", "schemas/burr.design-data.v1.schema.json", ...designDataFiles],
  "tracked design-data instances",
);

if (receiptFiles.length > 0) {
  for (const receiptFile of receiptFiles) {
    const receipt = JSON.parse(fs.readFileSync(path.join(repoRoot, receiptFile), "utf8"));
    if (receipt.schema_version !== "burr.receipt.v2") {
      throw new Error(
        `${receiptFile} declares ${JSON.stringify(receipt.schema_version)}; expected burr.receipt.v2`,
      );
    }
  }
  runCheck(
    ["--schemafile", "schemas/burr.receipt.v2.schema.json", ...receiptFiles],
    "tracked receipt instances",
  );
}

const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "burr-schema-check-"));
try {
  const receiptFixture = path.join(tempDir, "receipt-v2.json");
  fs.writeFileSync(receiptFixture, `${JSON.stringify(minimalReceiptV2(), null, 2)}\n`);
  runCheck(
    ["--schemafile", "schemas/burr.receipt.v2.schema.json", receiptFixture],
    "receipt v2 contract fixture",
  );

  const vacuousPassFixture = path.join(tempDir, "vacuous-pass-v2.json");
  const vacuousPass = minimalReceiptV2();
  vacuousPass.scope.rules.evaluated = 0;
  vacuousPass.scope.rules.evaluated_rule_ids = [];
  vacuousPass.summary.rules.evaluated = 0;
  fs.writeFileSync(vacuousPassFixture, `${JSON.stringify(vacuousPass, null, 2)}\n`);
  runCheck(
    ["--schemafile", "schemas/burr.receipt.v2.schema.json", vacuousPassFixture],
    "vacuous pass rejection",
    { expectFailure: true },
  );
} finally {
  fs.rmSync(tempDir, { recursive: true, force: true });
}

console.log(
  `schema checks passed (${schemaFiles.length} schemas, ${rulepackFiles.length} rulepacks, ` +
    `${designDataFiles.length} tracked design-data instances, ${receiptFiles.length} tracked receipts)`,
);

function gitTrackedFiles() {
  const result = spawnSync("git", ["ls-files", "-z"], {
    cwd: repoRoot,
    encoding: "buffer",
  });
  if (result.error) {
    throw new Error(`failed to list tracked files: ${result.error.message}`);
  }
  if (result.status !== 0) {
    throw new Error(`git ls-files failed with exit ${result.status}`);
  }
  return result.stdout
    .toString("utf8")
    .split("\0")
    .filter(Boolean);
}

function assertFiles(label, files) {
  if (files.length === 0) {
    throw new Error(`no ${label} found`);
  }
  for (const file of files) {
    if (!fs.existsSync(path.join(repoRoot, file))) {
      throw new Error(`${label} file is missing: ${file}`);
    }
  }
}

function runCheck(arguments_, label, options = {}) {
  const result = spawnSync(
    "uvx",
    [
      "--from",
      `check-jsonschema==${CHECK_JSONSCHEMA_VERSION}`,
      "check-jsonschema",
      ...arguments_,
    ],
    {
      cwd: repoRoot,
      encoding: "utf8",
      env: { ...process.env, UV_NO_PROGRESS: "1" },
    },
  );
  if (result.error) {
    throw new Error(`failed to run ${label}: ${result.error.message}`);
  }
  if (options.expectFailure) {
    if (result.status === 0) {
      throw new Error(`${label} unexpectedly passed`);
    }
    return;
  }
  if (result.status !== 0) {
    if (result.stdout) process.stdout.write(result.stdout);
    if (result.stderr) process.stderr.write(result.stderr);
    throw new Error(`${label} failed with exit ${result.status}`);
  }
}

function minimalReceiptV2() {
  const emptyCoverage = {
    declared: 0,
    checked: 0,
    unchecked: 0,
    checked_feature_ids: [],
    unchecked_feature_ids: [],
    mechanical_declared: 0,
    mechanical_checked: 0,
    mechanical_unchecked: 0,
    checked_mechanical_feature_ids: [],
    unchecked_mechanical_feature_ids: [],
    intent_counts: {},
    step_candidate_cylinders_considered: 0,
  };
  return {
    schema_version: "burr.receipt.v2",
    burr_version: "0.0.0-schema-check",
    outcome: "pass",
    status: "pass",
    artifact_id: "schema-check",
    artifact_version: null,
    artifact_type: "schema_check",
    rulepack_id: "schema_check",
    rulepack_version: "0.0.0",
    compatibility: {
      design_data_schema_version: "burr.design-data.v1",
      supported_design_data_schema_versions: ["burr.design-data.v1"],
      manifest_schema_version: "burr.design-data.v1",
      supported_manifest_schema_versions: ["burr.design-data.v1"],
      rulepack_schema_version: "burr.rulepack.v1",
      supported_rulepack_schema_versions: ["burr.rulepack.v1"],
    },
    source_design_data: null,
    source_manifest: null,
    checks: [
      {
        rule_id: "schema_check:inventory",
        status: "pass",
        reason: "ok",
        message: "Schema fixture rule passed.",
      },
    ],
    warnings: [],
    scope: {
      artifact_type: {
        design: "schema_check",
        rulepack: "schema_check",
        compatible: true,
      },
      process_kind: {
        design: null,
        rulepack: null,
        restricted: false,
        compatible: true,
      },
      rules: {
        declared: 1,
        evaluated: 1,
        evaluated_rule_ids: ["schema_check:inventory"],
      },
      mechanical_features: {
        declared: 0,
        checked: 0,
        unchecked: 0,
        unchecked_feature_ids: [],
      },
    },
    summary: {
      checks: 1,
      failures: 0,
      incomplete_checks: 0,
      warnings: 0,
      rules: { declared: 1, evaluated: 1 },
      features: emptyCoverage,
    },
  };
}
