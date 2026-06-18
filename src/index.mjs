import crypto from "node:crypto"
import fs from "node:fs"
import path from "node:path"
import { fileURLToPath } from "node:url"

const skipDirs = new Set([".git", ".jj", "node_modules", ".next", "dist", "build"])
const __filename = fileURLToPath(import.meta.url)
const packageRoot = path.resolve(path.dirname(__filename), "..")

export const defaultRulepackPath = path.join(packageRoot, "rules/actuator_mount.rulepack.json")

function readJson(filePath) {
  return JSON.parse(fs.readFileSync(filePath, "utf8"))
}

function writeJson(filePath, value) {
  fs.writeFileSync(`${filePath}.tmp`, `${JSON.stringify(value, null, 2)}\n`)
  fs.renameSync(`${filePath}.tmp`, filePath)
}

export function sha256File(filePath) {
  return crypto.createHash("sha256").update(fs.readFileSync(filePath)).digest("hex")
}

function isFiniteNumber(value) {
  return typeof value === "number" && Number.isFinite(value)
}

function round(value) {
  if (!isFiniteNumber(value)) return value
  return Number(value.toFixed(4))
}

function normalizeRoles(role) {
  if (Array.isArray(role)) return role.map(String)
  if (role == null) return []
  return [String(role)]
}

function featureApplies(feature, appliesTo = {}) {
  if (appliesTo.kind && feature.kind !== appliesTo.kind) return false
  if (appliesTo.fastener && feature.fastener !== appliesTo.fastener) return false
  if (Array.isArray(appliesTo.role_any) && appliesTo.role_any.length > 0) {
    const roles = normalizeRoles(feature.role)
    if (!roles.some((role) => appliesTo.role_any.includes(role))) return false
  }
  return true
}

function findPart(manifest, partId) {
  if (!partId) return null
  return (manifest.parts ?? []).find((part) => part.id === partId) ?? null
}

function axisIndexFromVector(axis) {
  if (!Array.isArray(axis) || axis.length !== 3) return null
  const magnitudes = axis.map((value) =>
    typeof value === "number" ? Math.abs(value) : 0,
  )
  const max = Math.max(...magnitudes)
  if (max <= 0) return null
  return magnitudes.indexOf(max)
}

function deriveCenterToBboxEdgeMm(manifest, feature) {
  const part = findPart(manifest, feature.part)
  const min = part?.bbox_mm?.min
  const max = part?.bbox_mm?.max
  const center = feature.center_mm
  if (
    !Array.isArray(min) ||
    !Array.isArray(max) ||
    !Array.isArray(center) ||
    min.length !== 3 ||
    max.length !== 3 ||
    center.length !== 3
  ) {
    return null
  }

  const holeAxis = axisIndexFromVector(feature.axis)
  const distances = []
  for (let axis = 0; axis < 3; axis += 1) {
    if (axis === holeAxis) continue
    if (
      isFiniteNumber(min[axis]) &&
      isFiniteNumber(max[axis]) &&
      isFiniteNumber(center[axis])
    ) {
      distances.push(center[axis] - min[axis], max[axis] - center[axis])
    }
  }

  const validDistances = distances.filter(
    (distance) => isFiniteNumber(distance) && distance >= 0,
  )
  if (validDistances.length === 0) return null
  return {
    value: Math.min(...validDistances),
    source: `parts[${feature.part}].bbox_mm nearest free edge`,
  }
}

function deriveCenterToEdgeMm(manifest, feature) {
  const bboxDistance = deriveCenterToBboxEdgeMm(manifest, feature)
  if (bboxDistance) return bboxDistance

  if (isFiniteNumber(feature.nearest_free_edge_distance_mm)) {
    return {
      value: feature.nearest_free_edge_distance_mm,
      source: "feature.nearest_free_edge_distance_mm",
    }
  }

  if (
    isFiniteNumber(feature.nearest_free_edge_material_mm) &&
    isFiniteNumber(feature.diameter_mm)
  ) {
    return {
      value: feature.nearest_free_edge_material_mm + feature.diameter_mm / 2,
      source: "feature.nearest_free_edge_material_mm + diameter / 2",
    }
  }

  return { value: undefined, source: "missing" }
}

function checkHoleEdgeDistance(manifest, rulepack, rule, feature) {
  const fullRuleId = `${rulepack.id}:${rule.id}`
  const diameter = feature.diameter_mm
  if (!isFiniteNumber(diameter) || diameter <= 0) {
    return {
      rule_id: fullRuleId,
      status: "fail",
      reason: "missing_hole_diameter",
      feature_id: feature.id ?? null,
      message: "Hole diameter is required for edge-distance linting.",
    }
  }

  const centerToEdge = deriveCenterToEdgeMm(manifest, feature)
  if (!isFiniteNumber(centerToEdge.value)) {
    return {
      rule_id: fullRuleId,
      status: "fail",
      reason: "missing_edge_measurement",
      feature_id: feature.id ?? null,
      measured: { center_to_edge_mm: null, source: centerToEdge.source },
      required: {
        center_to_edge_mm: round(rule.min_center_to_edge_diameter_multiple * diameter),
      },
      message: "Nearest free-edge distance cannot be derived.",
    }
  }

  const requiredCenterToEdge = rule.min_center_to_edge_diameter_multiple * diameter
  const wallToEdge = centerToEdge.value - diameter / 2
  const requiredWallToEdge = requiredCenterToEdge - diameter / 2
  const margin = centerToEdge.value - requiredCenterToEdge
  const pass = margin >= 0

  return {
    rule_id: fullRuleId,
    status: pass ? "pass" : "fail",
    reason: pass ? "ok" : "insufficient_edge_distance",
    feature_id: feature.id ?? null,
    measured: {
      hole_diameter_mm: diameter,
      center_to_edge_mm: round(centerToEdge.value),
      wall_to_edge_mm: round(wallToEdge),
      source: centerToEdge.source,
    },
    required: {
      center_to_edge_mm: round(requiredCenterToEdge),
      wall_to_edge_mm: round(requiredWallToEdge),
      center_to_edge_diameter_multiple: rule.min_center_to_edge_diameter_multiple,
    },
    margin_mm: round(margin),
    message: pass
      ? "Hole edge distance passes rule."
      : `Hole edge distance is short by ${round(Math.abs(margin))} mm.`,
  }
}

function normalizeFileRefs(manifest) {
  const refs = []
  if (manifest.source) refs.push({ group: "source", kind: "source", index: null, ref: manifest.source })
  for (const [index, ref] of (manifest.sources ?? []).entries()) {
    refs.push({ group: "sources", kind: "source", index, ref })
  }
  for (const [index, ref] of (manifest.artifacts ?? []).entries()) {
    refs.push({ group: "artifacts", kind: "artifact", index, ref })
  }
  return refs
}

function isSha256(value) {
  return typeof value === "string" && /^[a-f0-9]{64}$/.test(value)
}

function resolveFileRef(manifestDir, ref) {
  const relPath = ref?.path
  if (!relPath || typeof relPath !== "string") {
    return { error: relPath ? "invalid_path" : "missing_path", labelPath: relPath }
  }

  const manifestRoot = path.resolve(manifestDir)
  const filePath = path.resolve(manifestRoot, relPath)
  if (filePath !== manifestRoot && !filePath.startsWith(`${manifestRoot}${path.sep}`)) {
    return { error: "path_escapes_manifest_dir", filePath, labelPath: relPath }
  }
  return { filePath, labelPath: relPath }
}

function checkFileHashes(manifest, manifestDir) {
  const checks = []
  const refs = normalizeFileRefs(manifest)
  if (refs.length === 0) {
    return [
      {
        rule_id: "burr_manifest:hash_metadata_complete",
        status: "fail",
        reason: "missing_file_refs",
        message: "Manifest must list source/artifact file refs with sha256.",
      },
    ]
  }

  for (const { group, kind, index, ref } of refs) {
    const label = index == null ? group : `${group}[${index}]`
    const existsRule =
      kind === "source" ? "burr_manifest:source_file_exists" : "burr_manifest:artifact_file_exists"
    const hashRule =
      kind === "source" ? "burr_manifest:source_sha256_matches" : "burr_manifest:artifact_sha256_matches"
    const expectedSha = ref?.sha256
    const resolved = resolveFileRef(manifestDir, ref)

    if (resolved.error) {
      checks.push({
        rule_id: resolved.error === "missing_path" || resolved.error === "invalid_path"
          ? "burr_manifest:hash_metadata_complete"
          : existsRule,
        status: "fail",
        reason: resolved.error,
        file_ref: label,
        path: resolved.labelPath ?? null,
        message: "File ref path is invalid.",
      })
      continue
    }

    if (!isSha256(expectedSha)) {
      checks.push({
        rule_id: "burr_manifest:hash_metadata_complete",
        status: "fail",
        reason: expectedSha ? "invalid_sha256" : "missing_sha256",
        file_ref: label,
        path: resolved.labelPath,
        message: "File ref sha256 must be lowercase 64-character hex.",
      })
      continue
    }

    if (!fs.existsSync(resolved.filePath)) {
      checks.push({
        rule_id: existsRule,
        status: "fail",
        reason: kind === "source" ? "source_file_missing" : "artifact_file_missing",
        file_ref: label,
        path: resolved.labelPath,
        message: "File ref path does not exist.",
      })
      continue
    }

    checks.push({
      rule_id: existsRule,
      status: "pass",
      reason: "ok",
      file_ref: label,
      path: resolved.labelPath,
      message: "File ref path exists.",
    })

    const actualSha = sha256File(resolved.filePath)
    checks.push({
      rule_id: hashRule,
      status: actualSha === expectedSha ? "pass" : "fail",
      reason:
        actualSha === expectedSha
          ? "ok"
          : kind === "source"
            ? "source_hash_mismatch"
            : "artifact_hash_mismatch",
      file_ref: label,
      path: resolved.labelPath,
      measured: { sha256: actualSha },
      required: { sha256: expectedSha },
      message:
        actualSha === expectedSha
          ? "File hash matches manifest."
          : "File hash does not match manifest; metadata is stale.",
    })
  }

  return checks
}

export function lintManifest(manifest, rulepack, options = {}) {
  const checks = []
  const warnings = []
  const manifestDir = options.manifestDir ?? process.cwd()

  checks.push(...checkFileHashes(manifest, manifestDir))

  if (manifest.units && manifest.units !== "mm") {
    checks.push({
      rule_id: `${rulepack.id}:manifest_units_mm`,
      status: "fail",
      reason: "unsupported_units",
      message: "Burr currently expects millimeter manifests.",
      measured: { units: manifest.units },
      required: { units: "mm" },
    })
  }

  if (rulepack.artifact_type && manifest.artifact_type !== rulepack.artifact_type) {
    warnings.push({
      rule_id: `${rulepack.id}:artifact_type`,
      status: "warn",
      reason: "artifact_type_not_targeted",
      message: `Skipping artifact_type ${manifest.artifact_type ?? "<missing>"}.`,
    })
  } else {
    for (const rule of rulepack.rules ?? []) {
      if (rule.kind !== "hole_edge_distance") {
        warnings.push({
          rule_id: `${rulepack.id}:${rule.id}`,
          status: "warn",
          reason: "unsupported_rule_kind",
          message: `Unsupported rule kind ${rule.kind}.`,
        })
        continue
      }

      const features = (manifest.features ?? []).filter((feature) =>
        featureApplies(feature, rule.applies_to),
      )

      if (features.length === 0) {
        checks.push({
          rule_id: `${rulepack.id}:${rule.id}`,
          status: "fail",
          reason: "no_applicable_features",
          message: "No applicable features were found for this rule.",
        })
        continue
      }

      for (const feature of features) {
        checks.push(checkHoleEdgeDistance(manifest, rulepack, rule, feature))
      }
    }
  }

  const failures = checks.filter((check) => check.status === "fail")
  return {
    schema_version: "burr.receipt.v1",
    status: failures.length === 0 ? "pass" : "fail",
    artifact_id: manifest.artifact_id ?? null,
    artifact_type: manifest.artifact_type ?? null,
    rulepack_id: rulepack.id,
    source_manifest: options.sourceManifest ?? null,
    checks,
    warnings,
    summary: {
      checks: checks.length,
      failures: failures.length,
      warnings: warnings.length,
    },
  }
}

export function findManifestPaths(inputs, options = {}) {
  const cwd = options.cwd ?? process.cwd()
  const results = []
  const seen = new Set()

  for (const input of inputs) {
    const resolved = path.resolve(cwd, input)
    if (!fs.existsSync(resolved)) throw new Error(`Input does not exist: ${input}`)
    const stat = fs.statSync(resolved)
    if (stat.isFile()) {
      if (path.basename(resolved) !== "fray-cad.json") {
        throw new Error(`Input file is not fray-cad.json: ${input}`)
      }
      addManifest(results, seen, resolved)
      continue
    }

    const directManifest = path.join(resolved, "fray-cad.json")
    if (fs.existsSync(directManifest)) {
      addManifest(results, seen, directManifest)
    } else {
      walkForManifests(resolved, results, seen)
    }
  }

  return results
}

function addManifest(results, seen, manifestPath) {
  const normalized = path.resolve(manifestPath)
  if (seen.has(normalized)) return
  seen.add(normalized)
  results.push(normalized)
}

function walkForManifests(dir, results, seen) {
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    if (!entry.isDirectory()) continue
    if (skipDirs.has(entry.name)) continue
    const child = path.join(dir, entry.name)
    const manifest = path.join(child, "fray-cad.json")
    if (fs.existsSync(manifest)) addManifest(results, seen, manifest)
    else walkForManifests(child, results, seen)
  }
}

export function lintManifestFile(manifestPath, options = {}) {
  const rulepack = readJson(options.rulepackPath ?? defaultRulepackPath)
  const manifest = readJson(manifestPath)
  const receipt = lintManifest(manifest, rulepack, {
    sourceManifest: path.relative(process.cwd(), manifestPath),
    manifestDir: path.dirname(manifestPath),
  })
  const receiptPath =
    options.receiptPath ?? path.join(path.dirname(manifestPath), "burr-receipt.json")
  if (options.writeReceipt !== false) writeJson(receiptPath, receipt)
  return { receipt, receiptPath, manifestPath }
}

export function lintTargets(inputs, options = {}) {
  const manifestPaths = findManifestPaths(inputs, { cwd: options.cwd })
  if (manifestPaths.length === 0) throw new Error("No fray-cad.json manifests found.")
  return manifestPaths.map((manifestPath) => lintManifestFile(manifestPath, options))
}

function stampRef(manifestDir, ref) {
  const resolved = resolveFileRef(manifestDir, ref)
  if (resolved.error) throw new Error(`Cannot stamp invalid path: ${resolved.labelPath ?? resolved.error}`)
  if (!fs.existsSync(resolved.filePath)) throw new Error(`Ref path does not exist: ${resolved.labelPath}`)
  ref.sha256 = sha256File(resolved.filePath)
  ref.size_bytes = fs.statSync(resolved.filePath).size
}

export function stampManifestFile(manifestPath) {
  const manifest = readJson(manifestPath)
  const manifestDir = path.dirname(manifestPath)
  if (manifest.source) stampRef(manifestDir, manifest.source)
  for (const source of manifest.sources ?? []) stampRef(manifestDir, source)
  for (const artifact of manifest.artifacts ?? []) stampRef(manifestDir, artifact)
  writeJson(manifestPath, manifest)
  return manifestPath
}

export function stampTargets(inputs, options = {}) {
  const manifestPaths = findManifestPaths(inputs, { cwd: options.cwd })
  if (manifestPaths.length === 0) throw new Error("No fray-cad.json manifests found.")
  return manifestPaths.map(stampManifestFile)
}
