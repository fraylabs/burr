#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { galleryExamples } from "./gallery-examples.mjs";
import { buildRepairReports } from "./repair-report-builder.mjs";

const packageJson = JSON.parse(fs.readFileSync("package.json", "utf8"));
const version = packageJson.version;
const artifactSlug = `burr-gallery-v${version}`;
const releaseDir = path.join("artifacts", "releases", artifactSlug);
const previewDir = "artifacts/gallery-previews";
const zipPath = `${releaseDir}.zip`;

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

fs.rmSync(releaseDir, { recursive: true, force: true });
fs.rmSync(zipPath, { force: true });
fs.mkdirSync(releaseDir, { recursive: true });

run("node", ["scripts/render-gallery.mjs"]);

const generatedAt = new Date().toISOString();
const manifest = {
  schema_version: "burr.gallery-artifact.v1",
  burr_version: version,
  artifact_id: artifactSlug,
  generated_at: generatedAt,
  source: {
    repository: "fraylabs/burr",
    tag: `burr-v${version}`,
  },
  examples: [],
  repair_reports: [],
};

for (const example of galleryExamples) {
  const exampleDir = path.join(releaseDir, example.slug);
  fs.mkdirSync(exampleDir, { recursive: true });

  const previewSource = path.join(previewDir, example.preview);
  const receiptSource = path.join(example.dir, "burr-receipt.json");
  const designDataSource = path.join(example.dir, "burr-design-data.json");

  const previewTarget = path.join(exampleDir, example.preview);
  const receiptTarget = path.join(exampleDir, example.receipt);
  const designDataTarget = path.join(exampleDir, example.designData);

  copyRequired(previewSource, previewTarget);
  copyRequired(receiptSource, receiptTarget);
  copyRequired(designDataSource, designDataTarget);

  const receipt = JSON.parse(fs.readFileSync(receiptSource, "utf8"));
  if (receipt.status !== example.expectation) {
    throw new Error(
      `${receiptSource} status ${receipt.status} did not match expected ${example.expectation}`,
    );
  }

  manifest.examples.push({
    slug: example.slug,
    title: example.title,
    expectation: example.expectation,
    group: example.group,
    preview: `${example.slug}/${example.preview}`,
    receipt: `${example.slug}/${example.receipt}`,
    design_data: `${example.slug}/${example.designData}`,
    status: receipt.status,
    failed_rules: receipt.checks
      .filter((check) => check.status === "fail")
      .map((check) => ({
        rule_id: check.rule_id,
        feature_id: check.feature_id ?? null,
        reason: check.reason,
        message: check.message,
      })),
    checked_features: receipt.summary?.features?.checked_feature_ids ?? [],
    unchecked_features: receipt.summary?.features?.unchecked_feature_ids ?? [],
  });
}

manifest.repair_reports = buildRepairReports({
  releaseDir,
  version,
  generatedAt,
  examples: manifest.examples,
});

fs.writeFileSync(
  path.join(releaseDir, "manifest.json"),
  JSON.stringify(manifest, null, 2) + "\n",
);
fs.writeFileSync(
  path.join(releaseDir, "README.md"),
  [
    `# Burr Gallery v${version}`,
    "",
    "Website-ready Burr gallery artifacts.",
    "",
    "Each example includes:",
    "",
    "- a PNG preview for visual review",
    "- a Burr receipt as proof",
    "- the stamped design data that generated the receipt",
    "- repair reports that compare selected before/after receipts",
    "",
    "Passing examples show accepted design intent.",
    "Failing examples are intentional negative fixtures that show mistakes Burr catches.",
    "Preview PNGs are not the verifier. The Burr receipts are.",
    "Repair reports are derived from the Burr receipts in this artifact.",
    "",
  ].join("\n"),
);

run("python3", [
  "-c",
  `
import pathlib
import zipfile

root = pathlib.Path(${JSON.stringify(releaseDir)})
zip_path = pathlib.Path(${JSON.stringify(zipPath)})
with zipfile.ZipFile(zip_path, "w", compression=zipfile.ZIP_DEFLATED) as archive:
    for path in sorted(root.rglob("*")):
        if path.is_file():
            archive.write(path, path.relative_to(root.parent))
`,
]);

console.log(`gallery artifact written to ${releaseDir}`);
console.log(`gallery artifact zip written to ${zipPath}`);

function copyRequired(source, target) {
  if (!fs.existsSync(source)) {
    throw new Error(`Missing required gallery artifact source: ${source}`);
  }
  fs.copyFileSync(source, target);
}
