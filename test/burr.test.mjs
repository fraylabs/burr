import assert from "node:assert/strict"
import fs from "node:fs"
import os from "node:os"
import path from "node:path"
import { fileURLToPath } from "node:url"

import {
  burrVersion,
  lintManifestFile,
  receiptSchemaVersion,
  sha256File,
  stampTargets,
  supportedManifestSchemaVersions,
  supportedRulepackSchemaVersions,
} from "../src/index.mjs"

const __filename = fileURLToPath(import.meta.url)
const repoRoot = path.resolve(path.dirname(__filename), "..")
const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), "burr-"))
const rulepackPath = path.join(repoRoot, "rules/actuator_mount.rulepack.json")

try {
  fs.cpSync(path.join(repoRoot, "examples"), path.join(tempRoot, "examples"), {
    recursive: true,
  })

  const badDir = path.join(tempRoot, "examples/linear-actuator-bad")
  const goodDir = path.join(tempRoot, "examples/linear-actuator-good")
  stampTargets([badDir, goodDir])

  const bad = lintManifestFile(path.join(badDir, "fray-cad.json"), {
    rulepackPath,
  })
  assert.equal(bad.receipt.status, "fail")
  assert.ok(
    bad.receipt.checks.some(
      (check) =>
        check.rule_id === "actuator_mount:m3_loaded_hole_edge_distance" &&
        check.reason === "insufficient_edge_distance" &&
        check.measured.center_to_edge_mm === 8 &&
        check.required.center_to_edge_mm === 10.2,
    ),
  )

  const good = lintManifestFile(path.join(goodDir, "fray-cad.json"), {
    rulepackPath,
  })
  assert.equal(good.receipt.status, "pass")
  assert.equal(good.receipt.schema_version, receiptSchemaVersion)
  assert.equal(good.receipt.burr_version, burrVersion)
  assert.equal(good.receipt.artifact_version, "0.1.0")
  assert.equal(good.receipt.rulepack_version, "0.1.0")
  assert.deepEqual(
    good.receipt.compatibility.supported_manifest_schema_versions,
    supportedManifestSchemaVersions,
  )
  assert.deepEqual(
    good.receipt.compatibility.supported_rulepack_schema_versions,
    supportedRulepackSchemaVersions,
  )
  assert.ok(
    good.receipt.checks.some(
      (check) =>
        check.rule_id === "actuator_mount:m3_loaded_hole_edge_distance" &&
        check.measured.center_to_edge_mm === 12 &&
        check.margin_mm === 1.8,
    ),
  )

  const unsupportedSchemaManifest = JSON.parse(
    fs.readFileSync(path.join(goodDir, "fray-cad.json"), "utf8"),
  )
  unsupportedSchemaManifest.schema_version = "fray.cad.artifact.v99"
  fs.writeFileSync(
    path.join(goodDir, "fray-cad.json"),
    `${JSON.stringify(unsupportedSchemaManifest, null, 2)}\n`,
  )
  const unsupported = lintManifestFile(path.join(goodDir, "fray-cad.json"), {
    rulepackPath,
  })
  assert.equal(unsupported.receipt.status, "fail")
  assert.ok(
    unsupported.receipt.checks.some(
      (check) =>
        check.rule_id === "burr_manifest:schema_version_supported" &&
        check.reason === "unsupported_manifest_schema",
    ),
  )

  unsupportedSchemaManifest.schema_version = "fray.cad.artifact.v1"
  fs.writeFileSync(
    path.join(goodDir, "fray-cad.json"),
    `${JSON.stringify(unsupportedSchemaManifest, null, 2)}\n`,
  )

  fs.appendFileSync(path.join(goodDir, "source.py"), "\n# stale\n")
  const stale = lintManifestFile(path.join(goodDir, "fray-cad.json"), {
    rulepackPath,
  })
  assert.equal(stale.receipt.status, "fail")
  assert.ok(
    stale.receipt.checks.some(
      (check) =>
        check.rule_id === "burr_manifest:source_sha256_matches" &&
        check.reason === "source_hash_mismatch",
    ),
  )

  const manifest = JSON.parse(
    fs.readFileSync(path.join(goodDir, "fray-cad.json"), "utf8"),
  )
  manifest.source.sha256 = sha256File(path.join(goodDir, "source.py"))
  fs.writeFileSync(
    path.join(goodDir, "fray-cad.json"),
    `${JSON.stringify(manifest, null, 2)}\n`,
  )
  const restored = lintManifestFile(path.join(goodDir, "fray-cad.json"), {
    rulepackPath,
  })
  assert.equal(restored.receipt.status, "pass")
} finally {
  fs.rmSync(tempRoot, { recursive: true, force: true })
}

console.log("burr tests passed")
