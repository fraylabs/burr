#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { galleryExamples } from "./gallery-examples.mjs";

const packageJson = JSON.parse(fs.readFileSync("package.json", "utf8"));
const artifactSlug = `burr-gallery-v${packageJson.version}`;
const releaseDir = path.join("artifacts", "releases", artifactSlug);
const zipPath = `${releaseDir}.zip`;

const result = spawnSync("node", ["scripts/build-gallery-artifact.mjs"], {
  encoding: "utf8",
  maxBuffer: 1024 * 1024 * 32,
});
const output = [result.stdout, result.stderr].filter(Boolean).join("\n");
if (result.status !== 0) {
  throw new Error(
    `build-gallery-artifact failed with exit ${result.status}\n${output}`,
  );
}

const manifestPath = path.join(releaseDir, "manifest.json");
if (!fs.existsSync(manifestPath)) {
  throw new Error(`Missing manifest: ${manifestPath}`);
}

const manifest = JSON.parse(fs.readFileSync(manifestPath, "utf8"));
if (manifest.schema_version !== "burr.gallery-artifact.v1") {
  throw new Error(`Unexpected manifest schema: ${manifest.schema_version}`);
}
if (manifest.burr_version !== packageJson.version) {
  throw new Error(`Unexpected manifest Burr version: ${manifest.burr_version}`);
}
if (manifest.examples.length !== galleryExamples.length) {
  throw new Error(
    `Unexpected manifest example count: ${manifest.examples.length}`,
  );
}
if (!Array.isArray(manifest.repair_reports) || manifest.repair_reports.length < 1) {
  throw new Error("Manifest is missing repair reports.");
}

for (const example of galleryExamples) {
  const entry = manifest.examples.find((item) => item.slug === example.slug);
  if (!entry) {
    throw new Error(`Manifest missing ${example.slug}`);
  }
  if (entry.expectation !== example.expectation) {
    throw new Error(
      `Unexpected expectation for ${example.slug}: ${entry.expectation}`,
    );
  }
  if (entry.group !== example.group) {
    throw new Error(`Unexpected group for ${example.slug}: ${entry.group}`);
  }
  for (const key of ["preview", "receipt", "design_data"]) {
    const file = path.join(releaseDir, entry[key]);
    if (!fs.existsSync(file)) {
      throw new Error(`Missing ${key} file for ${example.slug}: ${file}`);
    }
    if (fs.statSync(file).size < 256) {
      throw new Error(`${key} file is too small for ${example.slug}: ${file}`);
    }
  }
  const receipt = JSON.parse(
    fs.readFileSync(path.join(releaseDir, entry.receipt), "utf8"),
  );
  if (receipt.status !== example.expectation) {
    throw new Error(`${example.slug} receipt is not ${example.expectation}`);
  }
  if (entry.status !== receipt.status) {
    throw new Error(`${example.slug} manifest status does not match receipt`);
  }
  if (example.expectation === "fail" && entry.failed_rules.length < 1) {
    throw new Error(
      `${example.slug} is a failing example without failed rules`,
    );
  }
  if (example.expectation === "pass" && entry.failed_rules.length !== 0) {
    throw new Error(`${example.slug} is a passing example with failed rules`);
  }
}

const statusCounts = manifest.examples.reduce(
  (counts, example) => ({
    ...counts,
    [example.status]: (counts[example.status] ?? 0) + 1,
  }),
  {},
);
if (!statusCounts.pass || !statusCounts.fail) {
  throw new Error(
    `Expected both pass and fail gallery examples: ${JSON.stringify(statusCounts)}`,
  );
}

if (!fs.existsSync(zipPath) || fs.statSync(zipPath).size < 4096) {
  throw new Error(`Gallery zip is missing or too small: ${zipPath}`);
}

for (const report of manifest.repair_reports) {
  for (const key of ["report_json", "report_markdown"]) {
    const file = path.join(releaseDir, report[key]);
    if (!fs.existsSync(file) || fs.statSync(file).size < 256) {
      throw new Error(`Missing or tiny repair report ${key}: ${file}`);
    }
  }
  const json = JSON.parse(fs.readFileSync(path.join(releaseDir, report.report_json), "utf8"));
  if (json.schema_version !== "burr.repair-report.v1") {
    throw new Error(`Unexpected repair report schema: ${json.schema_version}`);
  }
  if (json.status !== report.status) {
    throw new Error(`Repair report status mismatch for ${report.id}`);
  }
}

console.log("gallery artifact proof passed");
