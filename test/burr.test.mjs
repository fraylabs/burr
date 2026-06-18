import assert from "node:assert/strict"
import fs from "node:fs"
import os from "node:os"
import path from "node:path"
import { fileURLToPath } from "node:url"

import {
  lintManifestFile,
  sha256File,
  stampTargets,
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
  assert.ok(
    good.receipt.checks.some(
      (check) =>
        check.rule_id === "actuator_mount:m3_loaded_hole_edge_distance" &&
        check.measured.center_to_edge_mm === 12 &&
        check.margin_mm === 1.8,
    ),
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
