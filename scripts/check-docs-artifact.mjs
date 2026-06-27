#!/usr/bin/env node
import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";

const packageJson = JSON.parse(fs.readFileSync("package.json", "utf8"));
const sourceManifest = JSON.parse(
  fs.readFileSync("docs/static-docs-manifest.json", "utf8"),
);
const artifactSlug = `burr-docs-v${packageJson.version}`;
const releaseDir = path.join("artifacts", "releases", artifactSlug);
const zipPath = `${releaseDir}.zip`;

const result = spawnSync("node", ["scripts/build-docs-artifact.mjs"], {
  encoding: "utf8",
  maxBuffer: 1024 * 1024 * 16,
});
const output = [result.stdout, result.stderr].filter(Boolean).join("\n");
if (result.status !== 0) {
  throw new Error(
    `build-docs-artifact failed with exit ${result.status}\n${output}`,
  );
}

const manifestPath = path.join(releaseDir, "manifest.json");
if (!fs.existsSync(manifestPath)) {
  throw new Error(`Missing manifest: ${manifestPath}`);
}

const manifest = JSON.parse(fs.readFileSync(manifestPath, "utf8"));
if (manifest.schema_version !== "burr.docs-artifact.v1") {
  throw new Error(`Unexpected manifest schema: ${manifest.schema_version}`);
}
if (manifest.burr_version !== packageJson.version) {
  throw new Error(`Unexpected Burr version: ${manifest.burr_version}`);
}
if (manifest.artifact_id !== artifactSlug) {
  throw new Error(`Unexpected artifact id: ${manifest.artifact_id}`);
}
if (manifest.source?.repository !== "fraylabs/burr") {
  throw new Error(`Unexpected source repository: ${manifest.source?.repository}`);
}
if (manifest.source?.tag !== `burr-v${packageJson.version}`) {
  throw new Error(`Unexpected source tag: ${manifest.source?.tag}`);
}

checkEntries("documents");
checkEntries("references");
checkDocsContract();
checkZip();

console.log("docs artifact proof passed");

function checkEntries(key) {
  const expected = sourceManifest[key] ?? [];
  const actual = manifest[key] ?? [];
  if (actual.length !== expected.length) {
    throw new Error(`Unexpected ${key} count: ${actual.length}`);
  }

  for (const expectedEntry of expected) {
    const entry = actual.find(
      (item) => item.source_path === expectedEntry.source_path,
    );
    if (!entry) {
      throw new Error(`Manifest missing ${expectedEntry.source_path}`);
    }
    for (const field of ["title", "kind", "content_type", "bundle_path"]) {
      if (entry[field] !== expectedEntry[field]) {
        throw new Error(
          `${expectedEntry.source_path} ${field} mismatch: ${entry[field]}`,
        );
      }
    }

    const sourceContent = fs.readFileSync(expectedEntry.source_path);
    const artifactPath = path.join(releaseDir, entry.bundle_path);
    if (!fs.existsSync(artifactPath)) {
      throw new Error(`Missing bundled file: ${artifactPath}`);
    }
    const artifactContent = fs.readFileSync(artifactPath);
    if (!artifactContent.equals(sourceContent)) {
      throw new Error(`Bundled content mismatch: ${entry.bundle_path}`);
    }

    const sha256 = crypto
      .createHash("sha256")
      .update(sourceContent)
      .digest("hex");
    if (entry.sha256 !== sha256) {
      throw new Error(`Hash mismatch for ${entry.bundle_path}`);
    }
    if (entry.size_bytes !== sourceContent.length) {
      throw new Error(`Size mismatch for ${entry.bundle_path}`);
    }

    if (entry.content_type.startsWith("text/markdown")) {
      const text = sourceContent.toString("utf8");
      if (!entry.bundle_path.endsWith(".md") || !text.includes("#")) {
        throw new Error(`Invalid Markdown document: ${entry.bundle_path}`);
      }
    } else if (entry.content_type === "application/json") {
      JSON.parse(sourceContent.toString("utf8"));
    }
  }
}

const routedDocuments = manifest.documents.filter((entry) => entry.slug);
if (routedDocuments.length < 5) {
  throw new Error("Docs artifact must include routed website documents.");
}
for (const requiredSlug of [
  "how-it-works",
  "reference/design-data",
  "reference/receipt",
  "reference/cli",
  "reference/rulepack",
]) {
  const entry = routedDocuments.find((item) => item.slug === requiredSlug);
  if (!entry) {
    throw new Error(`Docs artifact missing routed page: ${requiredSlug}`);
  }
  if (!entry.section) {
    throw new Error(`Routed page ${requiredSlug} is missing section.`);
  }
}

function checkDocsContract() {
  const contract = fs.readFileSync("docs/static-docs-bundle.md", "utf8");
  const tag = `burr-v${packageJson.version}`;
  const assetName = `${artifactSlug}.zip`;
  const expectedUrl = `https://github.com/fraylabs/burr/releases/download/${tag}/${assetName}`;
  for (const expectedLine of [
    `release_tag: ${tag}`,
    `asset_name: ${assetName}`,
    `asset_url: ${expectedUrl}`,
  ]) {
    if (!contract.includes(expectedLine)) {
      throw new Error(`Docs contract is missing: ${expectedLine}`);
    }
  }
}

function checkZip() {
  if (!fs.existsSync(zipPath) || fs.statSync(zipPath).size < 2048) {
    throw new Error(`Docs zip is missing or too small: ${zipPath}`);
  }

  const expectedFiles = [
    `${artifactSlug}/README.md`,
    `${artifactSlug}/manifest.json`,
    ...manifest.documents.map((entry) => `${artifactSlug}/${entry.bundle_path}`),
    ...manifest.references.map((entry) => `${artifactSlug}/${entry.bundle_path}`),
  ];

  const zipCheck = spawnSync(
    "python3",
    [
      "-c",
      `
import json
import pathlib
import sys
import zipfile

zip_path = pathlib.Path(sys.argv[1])
expected = set(json.loads(sys.argv[2]))
with zipfile.ZipFile(zip_path) as archive:
    names = set(archive.namelist())
    missing = sorted(expected - names)
    if missing:
        raise SystemExit(f"Missing zip entries: {missing}")
    manifest = json.loads(archive.read(${JSON.stringify(`${artifactSlug}/manifest.json`)}))
    assert manifest["schema_version"] == "burr.docs-artifact.v1"
    assert manifest["artifact_id"] == ${JSON.stringify(artifactSlug)}
`,
      zipPath,
      JSON.stringify(expectedFiles),
    ],
    { encoding: "utf8" },
  );
  if (zipCheck.status !== 0) {
    const zipOutput = [zipCheck.stdout, zipCheck.stderr]
      .filter(Boolean)
      .join("\n");
    throw new Error(`Docs zip validation failed\n${zipOutput}`);
  }
}
