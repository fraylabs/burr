#!/usr/bin/env node
import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";

const packageJson = JSON.parse(fs.readFileSync("package.json", "utf8"));
const sourceManifestPath = "docs/static-docs-manifest.json";
const sourceManifest = JSON.parse(fs.readFileSync(sourceManifestPath, "utf8"));
const version = packageJson.version;
const artifactSlug = `burr-docs-v${version}`;
const releaseDir = path.join("artifacts", "releases", artifactSlug);
const zipPath = `${releaseDir}.zip`;

if (sourceManifest.schema_version !== "burr.docs-source-manifest.v1") {
  throw new Error(
    `Unexpected docs source manifest schema: ${sourceManifest.schema_version}`,
  );
}

fs.rmSync(releaseDir, { recursive: true, force: true });
fs.rmSync(zipPath, { force: true });
fs.mkdirSync(releaseDir, { recursive: true });

const generatedAt = new Date().toISOString();
const manifest = {
  schema_version: "burr.docs-artifact.v1",
  burr_version: version,
  artifact_id: artifactSlug,
  generated_at: generatedAt,
  source: {
    repository: "fraylabs/burr",
    tag: `burr-v${version}`,
  },
  source_manifest: sourceManifestPath,
  documents: [],
  references: [],
};

for (const entry of sourceManifest.documents ?? []) {
  manifest.documents.push(copyManifestEntry(entry, "document"));
}

for (const entry of sourceManifest.references ?? []) {
  manifest.references.push(copyManifestEntry(entry, "reference"));
}

fs.writeFileSync(
  path.join(releaseDir, "manifest.json"),
  JSON.stringify(manifest, null, 2) + "\n",
);
fs.writeFileSync(
  path.join(releaseDir, "README.md"),
  [
    `# Burr Docs v${version}`,
    "",
    "Website-ready Burr static documentation artifact.",
    "",
    "The generated manifest is the index. Files under `markdown/` are static",
    "Markdown documents. Files under `reference/` are supporting package,",
    "rulepack, and license references.",
    "",
    "This artifact is not a mechanical proof bundle. Burr gallery receipts",
    "remain the proof artifacts for checked CAD examples.",
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

console.log(`docs artifact written to ${releaseDir}`);
console.log(`docs artifact zip written to ${zipPath}`);

function copyManifestEntry(entry, category) {
  const sourcePath = cleanRelativePath(entry.source_path, "source_path");
  const bundlePath = cleanRelativePath(entry.bundle_path, "bundle_path");
  const sourceStats = fs.statSync(sourcePath, { throwIfNoEntry: false });
  if (!sourceStats?.isFile()) {
    throw new Error(`Missing docs ${category} source: ${sourcePath}`);
  }

  const targetPath = path.join(releaseDir, bundlePath);
  const targetDir = path.dirname(targetPath);
  fs.mkdirSync(targetDir, { recursive: true });
  fs.copyFileSync(sourcePath, targetPath);

  const content = fs.readFileSync(sourcePath);
  const manifestEntry = {
    title: requireString(entry.title, `${sourcePath} title`),
    kind: requireString(entry.kind, `${sourcePath} kind`),
    content_type: requireString(entry.content_type, `${sourcePath} content_type`),
    source_path: sourcePath,
    bundle_path: bundlePath,
    sha256: crypto.createHash("sha256").update(content).digest("hex"),
    size_bytes: content.length,
  };

  for (const optionalField of ["slug", "section", "nav_order"]) {
    if (optionalField in entry) {
      manifestEntry[optionalField] = entry[optionalField];
    }
  }

  return manifestEntry;
}

function cleanRelativePath(value, field) {
  const raw = requireString(value, field);
  if (path.isAbsolute(raw)) {
    throw new Error(`${field} must be relative: ${raw}`);
  }
  const normalized = path.normalize(raw).replaceAll(path.sep, "/");
  if (
    normalized === "." ||
    normalized === "" ||
    normalized.startsWith("../") ||
    normalized.includes("/../")
  ) {
    throw new Error(`${field} must stay inside the repo/artifact: ${raw}`);
  }
  return normalized;
}

function requireString(value, label) {
  if (typeof value !== "string" || value.trim() === "") {
    throw new Error(`Expected non-empty string for ${label}`);
  }
  return value;
}

function run(command, args) {
  const result = spawnSync(command, args, {
    encoding: "utf8",
    maxBuffer: 1024 * 1024 * 16,
  });
  const output = [result.stdout, result.stderr].filter(Boolean).join("\n");
  if (result.status !== 0) {
    throw new Error(
      `${command} ${args.join(" ")} failed with exit ${result.status}\n${output}`,
    );
  }
  return output;
}
