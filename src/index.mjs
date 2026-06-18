import crypto from "node:crypto"
import fs from "node:fs"
import path from "node:path"
import { fileURLToPath } from "node:url"

const skipDirs = new Set([".git", ".jj", "node_modules", ".next", "dist", "build"])
const __filename = fileURLToPath(import.meta.url)
const packageRoot = path.resolve(path.dirname(__filename), "..")

export const defaultRulepackPath = path.join(packageRoot, "rules/actuator_mount.rulepack.json")
export const burrVersion = readJson(path.join(packageRoot, "package.json")).version
export const designDataFileName = "burr-design-data.json"
export const legacyDesignDataFileNames = ["fray-cad.json"]
export const designDataFileNames = [designDataFileName, ...legacyDesignDataFileNames]
export const supportedDesignDataSchemaVersions = ["burr.design-data.v1"]
export const supportedLegacyDesignDataSchemaVersions = ["fray.cad.artifact.v1"]
export const supportedManifestSchemaVersions = [
  ...supportedDesignDataSchemaVersions,
  ...supportedLegacyDesignDataSchemaVersions,
]
export const supportedRulepackSchemaVersions = ["burr.rulepack.v1"]
export const receiptSchemaVersion = "burr.receipt.v1"

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
        rule_id: "burr_design_data:hash_metadata_complete",
        status: "fail",
        reason: "missing_file_refs",
        message: "Design data must list source/artifact file refs with sha256.",
      },
    ]
  }

  for (const { group, kind, index, ref } of refs) {
    const label = index == null ? group : `${group}[${index}]`
    const existsRule =
      kind === "source" ? "burr_design_data:source_file_exists" : "burr_design_data:artifact_file_exists"
    const hashRule =
      kind === "source" ? "burr_design_data:source_sha256_matches" : "burr_design_data:artifact_sha256_matches"
    const expectedSha = ref?.sha256
    const resolved = resolveFileRef(manifestDir, ref)

    if (resolved.error) {
      checks.push({
        rule_id: resolved.error === "missing_path" || resolved.error === "invalid_path"
          ? "burr_design_data:hash_metadata_complete"
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
        rule_id: "burr_design_data:hash_metadata_complete",
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
          ? "File hash matches design data."
          : "File hash does not match design data; metadata is stale.",
    })
  }

  return checks
}

function checkSchemaVersions(manifest, rulepack) {
  const checks = []
  if (!supportedManifestSchemaVersions.includes(manifest.schema_version)) {
    checks.push({
      rule_id: "burr_design_data:schema_version_supported",
      status: "fail",
      reason: manifest.schema_version ? "unsupported_design_data_schema" : "missing_design_data_schema",
      measured: { schema_version: manifest.schema_version ?? null },
      required: { schema_versions: supportedManifestSchemaVersions },
      message: "Design data schema version is not supported by this Burr version.",
    })
  } else {
    checks.push({
      rule_id: "burr_design_data:schema_version_supported",
      status: "pass",
      reason: "ok",
      measured: { schema_version: manifest.schema_version },
      message: "Design data schema version is supported.",
    })
  }

  if (!supportedRulepackSchemaVersions.includes(rulepack.schema_version)) {
    checks.push({
      rule_id: "burr_rulepack:schema_version_supported",
      status: "fail",
      reason: rulepack.schema_version ? "unsupported_rulepack_schema" : "missing_rulepack_schema",
      measured: { schema_version: rulepack.schema_version ?? null },
      required: { schema_versions: supportedRulepackSchemaVersions },
      message: "Rulepack schema version is not supported by this Burr version.",
    })
  } else {
    checks.push({
      rule_id: "burr_rulepack:schema_version_supported",
      status: "pass",
      reason: "ok",
      measured: { schema_version: rulepack.schema_version },
      message: "Rulepack schema version is supported.",
    })
  }

  return checks
}

export function lintDesignData(manifest, rulepack, options = {}) {
  const checks = []
  const warnings = []
  const manifestDir = options.manifestDir ?? process.cwd()

  checks.push(...checkSchemaVersions(manifest, rulepack))
  checks.push(...checkFileHashes(manifest, manifestDir))

  if (manifest.units && manifest.units !== "mm") {
    checks.push({
      rule_id: `${rulepack.id}:design_data_units_mm`,
      status: "fail",
      reason: "unsupported_units",
      message: "Burr currently expects millimeter design data.",
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
    schema_version: receiptSchemaVersion,
    burr_version: burrVersion,
    status: failures.length === 0 ? "pass" : "fail",
    artifact_id: manifest.artifact_id ?? null,
    artifact_version: manifest.artifact_version ?? null,
    artifact_type: manifest.artifact_type ?? null,
    rulepack_id: rulepack.id,
    rulepack_version: rulepack.version ?? null,
    compatibility: {
      design_data_schema_version: manifest.schema_version ?? null,
      supported_design_data_schema_versions: supportedManifestSchemaVersions,
      manifest_schema_version: manifest.schema_version ?? null,
      supported_manifest_schema_versions: supportedManifestSchemaVersions,
      rulepack_schema_version: rulepack.schema_version ?? null,
      supported_rulepack_schema_versions: supportedRulepackSchemaVersions,
    },
    source_design_data: options.sourceManifest ?? null,
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

export function formatCheckDiagnostic(check) {
  if (check.status !== "fail") return null

  if (check.reason === "insufficient_edge_distance") {
    const featureLabel = check.feature_id ? ` ${check.feature_id}` : ""
    const measured = check.measured?.center_to_edge_mm
    const required = check.required?.center_to_edge_mm
    const shortBy = isFiniteNumber(check.margin_mm) ? round(Math.abs(check.margin_mm)) : null
    const lines = [
      `M3 loaded hole${featureLabel} is too close to the edge.`,
    ]
    if (isFiniteNumber(measured)) lines.push(`Measured center-to-edge: ${measured} mm`)
    if (isFiniteNumber(required)) lines.push(`Required center-to-edge: ${required} mm`)
    if (isFiniteNumber(shortBy)) lines.push(`Short by: ${shortBy} mm`)
    lines.push("Try moving the hole inward or increasing the surrounding part size.")
    return lines
  }

  if (check.reason === "source_hash_mismatch" || check.reason === "artifact_hash_mismatch") {
    return [
      `Stale ${check.file_ref ?? "file"} hash for ${check.path ?? "<unknown>"}.`,
      "Run burr stamp after regenerating design data and artifacts.",
    ]
  }

  if (check.reason === "unsupported_design_data_schema") {
    return [
      "Design data schema is not supported by this Burr version.",
      `Found: ${check.measured?.schema_version ?? "<missing>"}`,
    ]
  }

  return [check.message ?? `${check.rule_id} failed.`]
}

export function formatReceiptDiagnostics(receipt) {
  const diagnostics = []
  for (const check of receipt.checks ?? []) {
    const lines = formatCheckDiagnostic(check)
    if (!lines) continue
    diagnostics.push({
      rule_id: check.rule_id,
      reason: check.reason,
      feature_id: check.feature_id ?? null,
      lines,
    })
  }
  return diagnostics
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
      if (!designDataFileNames.includes(path.basename(resolved))) {
        throw new Error(
          `Input file is not ${designDataFileName}: ${input}`,
        )
      }
      addManifest(results, seen, resolved)
      continue
    }

    const directManifest = findDirectDesignDataFile(resolved)
    if (fs.existsSync(directManifest)) {
      addManifest(results, seen, directManifest)
    } else {
      walkForManifests(resolved, results, seen)
    }
  }

  return results
}

export const findDesignDataPaths = findManifestPaths

function findDirectDesignDataFile(dir) {
  for (const fileName of designDataFileNames) {
    const filePath = path.join(dir, fileName)
    if (fs.existsSync(filePath)) return filePath
  }
  return path.join(dir, designDataFileName)
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
    const manifest = findDirectDesignDataFile(child)
    if (fs.existsSync(manifest)) addManifest(results, seen, manifest)
    else walkForManifests(child, results, seen)
  }
}

export function lintDesignDataFile(manifestPath, options = {}) {
  const rulepack = readJson(options.rulepackPath ?? defaultRulepackPath)
  const manifest = readJson(manifestPath)
  const receipt = lintDesignData(manifest, rulepack, {
    sourceManifest: path.relative(process.cwd(), manifestPath),
    manifestDir: path.dirname(manifestPath),
  })
  const receiptPath =
    options.receiptPath ?? path.join(path.dirname(manifestPath), "burr-receipt.json")
  if (options.writeReceipt !== false) writeJson(receiptPath, receipt)
  return { receipt, receiptPath, designDataPath: manifestPath, manifestPath }
}

export const lintManifest = lintDesignData
export const lintManifestFile = lintDesignDataFile

export function lintTargets(inputs, options = {}) {
  const manifestPaths = findManifestPaths(inputs, { cwd: options.cwd })
  if (manifestPaths.length === 0) throw new Error(`No ${designDataFileName} files found.`)
  return manifestPaths.map((manifestPath) => lintDesignDataFile(manifestPath, options))
}

function stampRef(manifestDir, ref) {
  const resolved = resolveFileRef(manifestDir, ref)
  if (resolved.error) throw new Error(`Cannot stamp invalid path: ${resolved.labelPath ?? resolved.error}`)
  if (!fs.existsSync(resolved.filePath)) throw new Error(`Ref path does not exist: ${resolved.labelPath}`)
  ref.sha256 = sha256File(resolved.filePath)
  ref.size_bytes = fs.statSync(resolved.filePath).size
}

export function stampDesignDataFile(manifestPath) {
  const manifest = readJson(manifestPath)
  const manifestDir = path.dirname(manifestPath)
  if (manifest.source) stampRef(manifestDir, manifest.source)
  for (const source of manifest.sources ?? []) stampRef(manifestDir, source)
  for (const artifact of manifest.artifacts ?? []) stampRef(manifestDir, artifact)
  writeJson(manifestPath, manifest)
  return manifestPath
}

export const stampManifestFile = stampDesignDataFile

export function stampTargets(inputs, options = {}) {
  const manifestPaths = findManifestPaths(inputs, { cwd: options.cwd })
  if (manifestPaths.length === 0) throw new Error(`No ${designDataFileName} files found.`)
  return manifestPaths.map(stampDesignDataFile)
}
