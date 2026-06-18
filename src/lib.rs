use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::fs;
use std::path::{Component, Path, PathBuf};

pub const BURR_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const DESIGN_DATA_FILE_NAME: &str = "burr-design-data.json";
pub const LEGACY_DESIGN_DATA_FILE_NAMES: [&str; 1] = ["fray-cad.json"];
pub const SUPPORTED_DESIGN_DATA_SCHEMA_VERSIONS: [&str; 1] = ["burr.design-data.v1"];
pub const SUPPORTED_LEGACY_DESIGN_DATA_SCHEMA_VERSIONS: [&str; 1] = ["fray.cad.artifact.v1"];
pub const SUPPORTED_RULEPACK_SCHEMA_VERSIONS: [&str; 1] = ["burr.rulepack.v1"];
pub const RECEIPT_SCHEMA_VERSION: &str = "burr.receipt.v1";

const DEFAULT_RULEPACK: &str = include_str!("../rules/actuator_mount.rulepack.json");
const SKIP_DIRS: [&str; 7] = [
    ".git",
    ".jj",
    "node_modules",
    ".next",
    "dist",
    "build",
    "target",
];

#[derive(Debug, Clone)]
pub struct LintOptions {
    pub rulepack_path: Option<PathBuf>,
    pub write_receipt: bool,
    pub cwd: PathBuf,
}

impl Default for LintOptions {
    fn default() -> Self {
        Self {
            rulepack_path: None,
            write_receipt: true,
            cwd: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        }
    }
}

#[derive(Debug, Clone)]
pub struct LintResult {
    pub receipt: Value,
    pub receipt_path: PathBuf,
    pub design_data_path: PathBuf,
}

pub fn default_rulepack() -> Result<Value, String> {
    read_json_str(DEFAULT_RULEPACK)
}

pub fn sha256_file(path: impl AsRef<Path>) -> Result<String, String> {
    let bytes = fs::read(path.as_ref())
        .map_err(|error| format!("Failed to read {}: {error}", path.as_ref().display()))?;
    let digest = Sha256::digest(bytes);
    Ok(digest.iter().map(|byte| format!("{byte:02x}")).collect())
}

pub fn find_design_data_paths(inputs: &[String], cwd: &Path) -> Result<Vec<PathBuf>, String> {
    let mut results = Vec::new();
    let mut seen = HashSet::new();

    for input in inputs {
        let resolved = normalize_path(&cwd.join(input));
        if !resolved.exists() {
            return Err(format!("Input does not exist: {input}"));
        }

        if resolved.is_file() {
            let file_name = resolved
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("");
            if !is_design_data_file_name(file_name) {
                return Err(format!(
                    "Input file is not {DESIGN_DATA_FILE_NAME}: {input}"
                ));
            }
            add_manifest(&mut results, &mut seen, resolved);
            continue;
        }

        let direct = find_direct_design_data_file(&resolved);
        if direct.exists() {
            add_manifest(&mut results, &mut seen, direct);
        } else {
            walk_for_manifests(&resolved, &mut results, &mut seen)?;
        }
    }

    Ok(results)
}

pub fn lint_targets(inputs: &[String], options: &LintOptions) -> Result<Vec<LintResult>, String> {
    let paths = find_design_data_paths(inputs, &options.cwd)?;
    if paths.is_empty() {
        return Err(format!("No {DESIGN_DATA_FILE_NAME} files found."));
    }
    paths
        .iter()
        .map(|path| lint_design_data_file(path, options))
        .collect()
}

pub fn lint_design_data_file(path: &Path, options: &LintOptions) -> Result<LintResult, String> {
    let rulepack = match &options.rulepack_path {
        Some(path) => read_json_file(path)?,
        None => default_rulepack()?,
    };
    let manifest = read_json_file(path)?;
    let manifest_dir = path.parent().unwrap_or_else(|| Path::new("."));
    let source_manifest = relative_label(&options.cwd, path);
    let receipt = lint_design_data(&manifest, &rulepack, manifest_dir, Some(source_manifest));
    let receipt_path = manifest_dir.join("burr-receipt.json");
    if options.write_receipt {
        write_json_file(&receipt_path, &receipt)?;
    }
    Ok(LintResult {
        receipt,
        receipt_path,
        design_data_path: path.to_path_buf(),
    })
}

pub fn lint_design_data(
    manifest: &Value,
    rulepack: &Value,
    manifest_dir: &Path,
    source_manifest: Option<String>,
) -> Value {
    let mut checks = Vec::new();
    let mut warnings = Vec::new();

    checks.extend(check_schema_versions(manifest, rulepack));
    checks.extend(check_file_hashes(manifest, manifest_dir));

    if string_field(manifest, "units").is_some_and(|units| units != "mm") {
        checks.push(json!({
            "rule_id": format!("{}:design_data_units_mm", string_field(rulepack, "id").unwrap_or("<missing>")),
            "status": "fail",
            "reason": "unsupported_units",
            "message": "Burr currently expects millimeter design data.",
            "measured": { "units": string_field(manifest, "units").unwrap_or("") },
            "required": { "units": "mm" }
        }));
    }

    if let Some(artifact_type) = string_field(rulepack, "artifact_type") {
        if string_field(manifest, "artifact_type") != Some(artifact_type) {
            warnings.push(json!({
                "rule_id": format!("{}:artifact_type", string_field(rulepack, "id").unwrap_or("<missing>")),
                "status": "warn",
                "reason": "artifact_type_not_targeted",
                "message": format!("Skipping artifact_type {}.", string_field(manifest, "artifact_type").unwrap_or("<missing>"))
            }));
        }
    }

    if warnings.is_empty()
        || !warnings
            .iter()
            .any(|warning| string_field(warning, "reason") == Some("artifact_type_not_targeted"))
    {
        for rule in rulepack
            .get("rules")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            if string_field(rule, "kind") != Some("hole_edge_distance") {
                warnings.push(json!({
                    "rule_id": format!("{}:{}", string_field(rulepack, "id").unwrap_or("<missing>"), string_field(rule, "id").unwrap_or("<missing>")),
                    "status": "warn",
                    "reason": "unsupported_rule_kind",
                    "message": format!("Unsupported rule kind {}.", string_field(rule, "kind").unwrap_or("<missing>"))
                }));
                continue;
            }

            let features: Vec<&Value> = manifest
                .get("features")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter(|feature| feature_applies(feature, rule.get("applies_to")))
                .collect();

            if features.is_empty() {
                checks.push(json!({
                    "rule_id": format!("{}:{}", string_field(rulepack, "id").unwrap_or("<missing>"), string_field(rule, "id").unwrap_or("<missing>")),
                    "status": "fail",
                    "reason": "no_applicable_features",
                    "message": "No applicable features were found for this rule."
                }));
                continue;
            }

            for feature in features {
                checks.push(check_hole_edge_distance(manifest, rulepack, rule, feature));
            }
        }
    }

    let failures = checks
        .iter()
        .filter(|check| string_field(check, "status") == Some("fail"))
        .count();

    json!({
        "schema_version": RECEIPT_SCHEMA_VERSION,
        "burr_version": BURR_VERSION,
        "status": if failures == 0 { "pass" } else { "fail" },
        "artifact_id": manifest.get("artifact_id").cloned().unwrap_or(Value::Null),
        "artifact_version": manifest.get("artifact_version").cloned().unwrap_or(Value::Null),
        "artifact_type": manifest.get("artifact_type").cloned().unwrap_or(Value::Null),
        "rulepack_id": rulepack.get("id").cloned().unwrap_or(Value::Null),
        "rulepack_version": rulepack.get("version").cloned().unwrap_or(Value::Null),
        "compatibility": {
            "design_data_schema_version": manifest.get("schema_version").cloned().unwrap_or(Value::Null),
            "supported_design_data_schema_versions": supported_manifest_schema_versions(),
            "manifest_schema_version": manifest.get("schema_version").cloned().unwrap_or(Value::Null),
            "supported_manifest_schema_versions": supported_manifest_schema_versions(),
            "rulepack_schema_version": rulepack.get("schema_version").cloned().unwrap_or(Value::Null),
            "supported_rulepack_schema_versions": SUPPORTED_RULEPACK_SCHEMA_VERSIONS
        },
        "source_design_data": source_manifest.clone().map(Value::String).unwrap_or(Value::Null),
        "source_manifest": source_manifest.map(Value::String).unwrap_or(Value::Null),
        "checks": checks,
        "warnings": warnings,
        "summary": {
            "checks": checks.len(),
            "failures": failures,
            "warnings": warnings.len()
        }
    })
}

pub fn stamp_targets(inputs: &[String], cwd: &Path) -> Result<Vec<PathBuf>, String> {
    let paths = find_design_data_paths(inputs, cwd)?;
    if paths.is_empty() {
        return Err(format!("No {DESIGN_DATA_FILE_NAME} files found."));
    }
    paths
        .iter()
        .map(|path| stamp_design_data_file(path))
        .collect()
}

pub fn stamp_design_data_file(path: &Path) -> Result<PathBuf, String> {
    let mut manifest = read_json_file(path)?;
    let manifest_dir = path.parent().unwrap_or_else(|| Path::new("."));

    if let Some(source) = manifest.get_mut("source") {
        stamp_ref(manifest_dir, source)?;
    }
    for source in manifest
        .get_mut("sources")
        .and_then(Value::as_array_mut)
        .into_iter()
        .flatten()
    {
        stamp_ref(manifest_dir, source)?;
    }
    for artifact in manifest
        .get_mut("artifacts")
        .and_then(Value::as_array_mut)
        .into_iter()
        .flatten()
    {
        stamp_ref(manifest_dir, artifact)?;
    }

    write_json_file(path, &manifest)?;
    Ok(path.to_path_buf())
}

pub fn format_receipt_diagnostics(receipt: &Value) -> Vec<Vec<String>> {
    receipt
        .get("checks")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(format_check_diagnostic)
        .collect()
}

fn format_check_diagnostic(check: &Value) -> Option<Vec<String>> {
    if string_field(check, "status") != Some("fail") {
        return None;
    }

    match string_field(check, "reason") {
        Some("insufficient_edge_distance") => {
            let feature_label = string_field(check, "feature_id")
                .map(|id| format!(" {id}"))
                .unwrap_or_default();
            let measured = check
                .pointer("/measured/center_to_edge_mm")
                .and_then(Value::as_f64);
            let required = check
                .pointer("/required/center_to_edge_mm")
                .and_then(Value::as_f64);
            let short_by = number_field(check, "margin_mm").map(|value| round(value.abs()));
            let mut lines = vec![format!(
                "M3 loaded hole{feature_label} is too close to the edge."
            )];
            if let Some(value) = measured {
                lines.push(format!("Measured center-to-edge: {} mm", trim_float(value)));
            }
            if let Some(value) = required {
                lines.push(format!("Required center-to-edge: {} mm", trim_float(value)));
            }
            if let Some(value) = short_by {
                lines.push(format!("Short by: {} mm", trim_float(value)));
            }
            lines.push(
                "Try moving the hole inward or increasing the surrounding part size.".to_string(),
            );
            Some(lines)
        }
        Some("source_hash_mismatch") | Some("artifact_hash_mismatch") => Some(vec![
            format!(
                "Stale {} hash for {}.",
                string_field(check, "file_ref").unwrap_or("file"),
                string_field(check, "path").unwrap_or("<unknown>")
            ),
            "Run burr stamp after regenerating design data and artifacts.".to_string(),
        ]),
        Some("unsupported_design_data_schema") => Some(vec![
            "Design data schema is not supported by this Burr version.".to_string(),
            format!(
                "Found: {}",
                check
                    .pointer("/measured/schema_version")
                    .and_then(Value::as_str)
                    .unwrap_or("<missing>")
            ),
        ]),
        _ => Some(vec![string_field(check, "message")
            .unwrap_or("Check failed.")
            .to_string()]),
    }
}

fn check_schema_versions(manifest: &Value, rulepack: &Value) -> Vec<Value> {
    let mut checks = Vec::new();
    let schema = string_field(manifest, "schema_version");
    if !supported_manifest_schema_versions()
        .iter()
        .any(|value| Some(*value) == schema)
    {
        checks.push(json!({
            "rule_id": "burr_design_data:schema_version_supported",
            "status": "fail",
            "reason": if schema.is_some() { "unsupported_design_data_schema" } else { "missing_design_data_schema" },
            "measured": { "schema_version": schema },
            "required": { "schema_versions": supported_manifest_schema_versions() },
            "message": "Design data schema version is not supported by this Burr version."
        }));
    } else {
        checks.push(json!({
            "rule_id": "burr_design_data:schema_version_supported",
            "status": "pass",
            "reason": "ok",
            "measured": { "schema_version": schema },
            "message": "Design data schema version is supported."
        }));
    }

    let schema = string_field(rulepack, "schema_version");
    if !SUPPORTED_RULEPACK_SCHEMA_VERSIONS
        .iter()
        .any(|value| Some(*value) == schema)
    {
        checks.push(json!({
            "rule_id": "burr_rulepack:schema_version_supported",
            "status": "fail",
            "reason": if schema.is_some() { "unsupported_rulepack_schema" } else { "missing_rulepack_schema" },
            "measured": { "schema_version": schema },
            "required": { "schema_versions": SUPPORTED_RULEPACK_SCHEMA_VERSIONS },
            "message": "Rulepack schema version is not supported by this Burr version."
        }));
    } else {
        checks.push(json!({
            "rule_id": "burr_rulepack:schema_version_supported",
            "status": "pass",
            "reason": "ok",
            "measured": { "schema_version": schema },
            "message": "Rulepack schema version is supported."
        }));
    }

    checks
}

fn check_file_hashes(manifest: &Value, manifest_dir: &Path) -> Vec<Value> {
    let refs = normalize_file_refs(manifest);
    if refs.is_empty() {
        return vec![json!({
            "rule_id": "burr_design_data:hash_metadata_complete",
            "status": "fail",
            "reason": "missing_file_refs",
            "message": "Design data must list source/artifact file refs with sha256."
        })];
    }

    let mut checks = Vec::new();
    for file_ref in refs {
        let exists_rule = if file_ref.kind == "source" {
            "burr_design_data:source_file_exists"
        } else {
            "burr_design_data:artifact_file_exists"
        };
        let hash_rule = if file_ref.kind == "source" {
            "burr_design_data:source_sha256_matches"
        } else {
            "burr_design_data:artifact_sha256_matches"
        };
        let expected_sha = file_ref.value.get("sha256").and_then(Value::as_str);
        let resolved = resolve_file_ref(manifest_dir, file_ref.value);

        let Ok(resolved) = resolved else {
            let reason = resolved.err().unwrap();
            checks.push(json!({
                "rule_id": if reason == "missing_path" || reason == "invalid_path" { "burr_design_data:hash_metadata_complete" } else { exists_rule },
                "status": "fail",
                "reason": reason,
                "file_ref": file_ref.label,
                "path": file_ref.value.get("path").and_then(Value::as_str),
                "message": "File ref path is invalid."
            }));
            continue;
        };

        if !expected_sha.is_some_and(is_sha256) {
            checks.push(json!({
                "rule_id": "burr_design_data:hash_metadata_complete",
                "status": "fail",
                "reason": if expected_sha.is_some() { "invalid_sha256" } else { "missing_sha256" },
                "file_ref": file_ref.label,
                "path": resolved.label_path,
                "message": "File ref sha256 must be lowercase 64-character hex."
            }));
            continue;
        }

        if !resolved.file_path.exists() {
            checks.push(json!({
                "rule_id": exists_rule,
                "status": "fail",
                "reason": if file_ref.kind == "source" { "source_file_missing" } else { "artifact_file_missing" },
                "file_ref": file_ref.label,
                "path": resolved.label_path,
                "message": "File ref path does not exist."
            }));
            continue;
        }

        checks.push(json!({
            "rule_id": exists_rule,
            "status": "pass",
            "reason": "ok",
            "file_ref": file_ref.label,
            "path": resolved.label_path,
            "message": "File ref path exists."
        }));

        match sha256_file(&resolved.file_path) {
            Ok(actual_sha) => checks.push(json!({
                "rule_id": hash_rule,
                "status": if Some(actual_sha.as_str()) == expected_sha { "pass" } else { "fail" },
                "reason": if Some(actual_sha.as_str()) == expected_sha {
                    "ok"
                } else if file_ref.kind == "source" {
                    "source_hash_mismatch"
                } else {
                    "artifact_hash_mismatch"
                },
                "file_ref": file_ref.label,
                "path": resolved.label_path,
                "measured": { "sha256": actual_sha },
                "required": { "sha256": expected_sha },
                "message": if Some(actual_sha.as_str()) == expected_sha {
                    "File hash matches design data."
                } else {
                    "File hash does not match design data; metadata is stale."
                }
            })),
            Err(error) => checks.push(json!({
                "rule_id": hash_rule,
                "status": "fail",
                "reason": "hash_read_failed",
                "file_ref": file_ref.label,
                "path": resolved.label_path,
                "message": error
            })),
        }
    }

    checks
}

fn check_hole_edge_distance(
    manifest: &Value,
    rulepack: &Value,
    rule: &Value,
    feature: &Value,
) -> Value {
    let full_rule_id = format!(
        "{}:{}",
        string_field(rulepack, "id").unwrap_or("<missing>"),
        string_field(rule, "id").unwrap_or("<missing>")
    );
    let diameter = number_field(feature, "diameter_mm");
    if !diameter.is_some_and(|value| value > 0.0) {
        return json!({
            "rule_id": full_rule_id,
            "status": "fail",
            "reason": "missing_hole_diameter",
            "feature_id": feature.get("id").cloned().unwrap_or(Value::Null),
            "message": "Hole diameter is required for edge-distance linting."
        });
    }
    let diameter = diameter.unwrap();

    let center_to_edge = derive_center_to_edge_mm(manifest, feature);
    let Some(center_to_edge_value) = center_to_edge.value else {
        return json!({
            "rule_id": full_rule_id,
            "status": "fail",
            "reason": "missing_edge_measurement",
            "feature_id": feature.get("id").cloned().unwrap_or(Value::Null),
            "measured": { "center_to_edge_mm": Value::Null, "source": center_to_edge.source },
            "required": {
                "center_to_edge_mm": round(number_field(rule, "min_center_to_edge_diameter_multiple").unwrap_or(0.0) * diameter)
            },
            "message": "Nearest free-edge distance cannot be derived."
        });
    };

    let multiple = number_field(rule, "min_center_to_edge_diameter_multiple").unwrap_or(0.0);
    let required_center_to_edge = multiple * diameter;
    let wall_to_edge = center_to_edge_value - diameter / 2.0;
    let required_wall_to_edge = required_center_to_edge - diameter / 2.0;
    let margin = center_to_edge_value - required_center_to_edge;
    let pass = margin >= 0.0;

    json!({
        "rule_id": full_rule_id,
        "status": if pass { "pass" } else { "fail" },
        "reason": if pass { "ok" } else { "insufficient_edge_distance" },
        "feature_id": feature.get("id").cloned().unwrap_or(Value::Null),
        "measured": {
            "hole_diameter_mm": diameter,
            "center_to_edge_mm": round(center_to_edge_value),
            "wall_to_edge_mm": round(wall_to_edge),
            "source": center_to_edge.source
        },
        "required": {
            "center_to_edge_mm": round(required_center_to_edge),
            "wall_to_edge_mm": round(required_wall_to_edge),
            "center_to_edge_diameter_multiple": multiple
        },
        "margin_mm": round(margin),
        "message": if pass {
            "Hole edge distance passes rule.".to_string()
        } else {
            format!("Hole edge distance is short by {} mm.", trim_float(round(margin.abs())))
        }
    })
}

#[derive(Debug)]
struct FileRef<'a> {
    kind: &'static str,
    label: String,
    value: &'a Value,
}

#[derive(Debug)]
struct ResolvedFileRef {
    file_path: PathBuf,
    label_path: String,
}

#[derive(Debug)]
struct DerivedDistance {
    value: Option<f64>,
    source: &'static str,
}

fn normalize_file_refs(manifest: &Value) -> Vec<FileRef<'_>> {
    let mut refs = Vec::new();
    if let Some(source) = manifest.get("source") {
        refs.push(FileRef {
            kind: "source",
            label: "source".to_string(),
            value: source,
        });
    }
    for (index, source) in manifest
        .get("sources")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .enumerate()
    {
        refs.push(FileRef {
            kind: "source",
            label: format!("sources[{index}]"),
            value: source,
        });
    }
    for (index, artifact) in manifest
        .get("artifacts")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .enumerate()
    {
        refs.push(FileRef {
            kind: "artifact",
            label: format!("artifacts[{index}]"),
            value: artifact,
        });
    }
    refs
}

fn resolve_file_ref(manifest_dir: &Path, file_ref: &Value) -> Result<ResolvedFileRef, String> {
    let Some(rel_path) = file_ref.get("path").and_then(Value::as_str) else {
        return Err("missing_path".to_string());
    };
    if rel_path.is_empty() {
        return Err("invalid_path".to_string());
    }

    let root = normalize_path(manifest_dir);
    let file_path = normalize_path(&root.join(rel_path));
    if file_path != root && !file_path.starts_with(&root) {
        return Err("path_escapes_manifest_dir".to_string());
    }
    Ok(ResolvedFileRef {
        file_path,
        label_path: rel_path.to_string(),
    })
}

fn stamp_ref(manifest_dir: &Path, file_ref: &mut Value) -> Result<(), String> {
    let resolved = resolve_file_ref(manifest_dir, file_ref)?;
    if !resolved.file_path.exists() {
        return Err(format!("Ref path does not exist: {}", resolved.label_path));
    }
    let sha = sha256_file(&resolved.file_path)?;
    let size = fs::metadata(&resolved.file_path)
        .map_err(|error| format!("Failed to stat {}: {error}", resolved.file_path.display()))?
        .len();
    if let Some(object) = file_ref.as_object_mut() {
        object.insert("sha256".to_string(), Value::String(sha));
        object.insert("size_bytes".to_string(), json!(size));
    }
    Ok(())
}

fn feature_applies(feature: &Value, applies_to: Option<&Value>) -> bool {
    let Some(applies_to) = applies_to else {
        return true;
    };
    if let Some(kind) = string_field(applies_to, "kind") {
        if string_field(feature, "kind") != Some(kind) {
            return false;
        }
    }
    if let Some(fastener) = string_field(applies_to, "fastener") {
        if string_field(feature, "fastener") != Some(fastener) {
            return false;
        }
    }
    if let Some(role_any) = applies_to.get("role_any").and_then(Value::as_array) {
        if !role_any.is_empty() {
            let roles = normalize_roles(feature.get("role"));
            let allowed: HashSet<&str> = role_any.iter().filter_map(Value::as_str).collect();
            if !roles.iter().any(|role| allowed.contains(role.as_str())) {
                return false;
            }
        }
    }
    true
}

fn normalize_roles(role: Option<&Value>) -> Vec<String> {
    match role {
        Some(Value::Array(values)) => values
            .iter()
            .filter_map(|value| value.as_str().map(ToString::to_string))
            .collect(),
        Some(Value::String(value)) => vec![value.clone()],
        Some(value) if !value.is_null() => vec![value.to_string()],
        _ => Vec::new(),
    }
}

fn derive_center_to_edge_mm(manifest: &Value, feature: &Value) -> DerivedDistance {
    if let Some(distance) = derive_center_to_bbox_edge_mm(manifest, feature) {
        return distance;
    }
    if let Some(value) = number_field(feature, "nearest_free_edge_distance_mm") {
        return DerivedDistance {
            value: Some(value),
            source: "feature.nearest_free_edge_distance_mm",
        };
    }
    if let (Some(material), Some(diameter)) = (
        number_field(feature, "nearest_free_edge_material_mm"),
        number_field(feature, "diameter_mm"),
    ) {
        return DerivedDistance {
            value: Some(material + diameter / 2.0),
            source: "feature.nearest_free_edge_material_mm + diameter / 2",
        };
    }
    DerivedDistance {
        value: None,
        source: "missing",
    }
}

fn derive_center_to_bbox_edge_mm(manifest: &Value, feature: &Value) -> Option<DerivedDistance> {
    let part = find_part(manifest, string_field(feature, "part")?)?;
    let min = number_array(part.pointer("/bbox_mm/min")?)?;
    let max = number_array(part.pointer("/bbox_mm/max")?)?;
    let center = number_array(feature.get("center_mm")?)?;
    if min.len() != 3 || max.len() != 3 || center.len() != 3 {
        return None;
    }

    let hole_axis = feature
        .get("axis")
        .and_then(number_array)
        .and_then(axis_index_from_vector);
    let mut distances = Vec::new();
    for axis in 0..3 {
        if Some(axis) == hole_axis {
            continue;
        }
        distances.push(center[axis] - min[axis]);
        distances.push(max[axis] - center[axis]);
    }
    let value = distances
        .into_iter()
        .filter(|distance| distance.is_finite() && *distance >= 0.0)
        .fold(None, |best: Option<f64>, distance| {
            Some(best.map_or(distance, |best| best.min(distance)))
        })?;
    Some(DerivedDistance {
        value: Some(value),
        source: "parts[feature.part].bbox_mm nearest free edge",
    })
}

fn find_part<'a>(manifest: &'a Value, part_id: &str) -> Option<&'a Value> {
    manifest
        .get("parts")?
        .as_array()?
        .iter()
        .find(|part| string_field(part, "id") == Some(part_id))
}

fn axis_index_from_vector(axis: Vec<f64>) -> Option<usize> {
    if axis.len() != 3 {
        return None;
    }
    let mut best_index = None;
    let mut best_value = 0.0;
    for (index, value) in axis.iter().enumerate() {
        let value = value.abs();
        if value > best_value {
            best_value = value;
            best_index = Some(index);
        }
    }
    if best_value <= 0.0 {
        None
    } else {
        best_index
    }
}

fn read_json_file(path: &Path) -> Result<Value, String> {
    let text = fs::read_to_string(path)
        .map_err(|error| format!("Failed to read {}: {error}", path.display()))?;
    read_json_str(&text).map_err(|error| format!("Failed to parse {}: {error}", path.display()))
}

fn read_json_str(text: &str) -> Result<Value, String> {
    serde_json::from_str(text).map_err(|error| error.to_string())
}

fn write_json_file(path: &Path, value: &Value) -> Result<(), String> {
    let tmp_path = path.with_extension(format!(
        "{}tmp",
        path.extension()
            .and_then(|extension| extension.to_str())
            .map(|extension| format!("{extension}."))
            .unwrap_or_default()
    ));
    let text = serde_json::to_string_pretty(value).map_err(|error| error.to_string())? + "\n";
    fs::write(&tmp_path, text)
        .map_err(|error| format!("Failed to write {}: {error}", tmp_path.display()))?;
    fs::rename(&tmp_path, path)
        .map_err(|error| format!("Failed to replace {}: {error}", path.display()))
}

fn is_design_data_file_name(name: &str) -> bool {
    name == DESIGN_DATA_FILE_NAME || LEGACY_DESIGN_DATA_FILE_NAMES.contains(&name)
}

fn find_direct_design_data_file(dir: &Path) -> PathBuf {
    let preferred = dir.join(DESIGN_DATA_FILE_NAME);
    if preferred.exists() {
        return preferred;
    }
    for file_name in LEGACY_DESIGN_DATA_FILE_NAMES {
        let path = dir.join(file_name);
        if path.exists() {
            return path;
        }
    }
    preferred
}

fn add_manifest(results: &mut Vec<PathBuf>, seen: &mut HashSet<PathBuf>, path: PathBuf) {
    let path = normalize_path(&path);
    if seen.insert(path.clone()) {
        results.push(path);
    }
}

fn walk_for_manifests(
    dir: &Path,
    results: &mut Vec<PathBuf>,
    seen: &mut HashSet<PathBuf>,
) -> Result<(), String> {
    for entry in
        fs::read_dir(dir).map_err(|error| format!("Failed to read {}: {error}", dir.display()))?
    {
        let entry = entry.map_err(|error| error.to_string())?;
        let file_type = entry.file_type().map_err(|error| error.to_string())?;
        if !file_type.is_dir() {
            continue;
        }
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if SKIP_DIRS.contains(&name.as_ref()) {
            continue;
        }
        let child = entry.path();
        let manifest = find_direct_design_data_file(&child);
        if manifest.exists() {
            add_manifest(results, seen, manifest);
        } else {
            walk_for_manifests(&child, results, seen)?;
        }
    }
    Ok(())
}

fn supported_manifest_schema_versions() -> Vec<&'static str> {
    SUPPORTED_DESIGN_DATA_SCHEMA_VERSIONS
        .into_iter()
        .chain(SUPPORTED_LEGACY_DESIGN_DATA_SCHEMA_VERSIONS)
        .collect()
}

fn string_field<'a>(value: &'a Value, field: &str) -> Option<&'a str> {
    value.get(field).and_then(Value::as_str)
}

fn number_field(value: &Value, field: &str) -> Option<f64> {
    value
        .get(field)
        .and_then(Value::as_f64)
        .filter(|value| value.is_finite())
}

fn number_array(value: &Value) -> Option<Vec<f64>> {
    value
        .as_array()?
        .iter()
        .map(|item| item.as_f64().filter(|value| value.is_finite()))
        .collect()
}

fn round(value: f64) -> f64 {
    (value * 10_000.0).round() / 10_000.0
}

fn trim_float(value: f64) -> String {
    let text = format!("{value:.4}");
    text.trim_end_matches('0').trim_end_matches('.').to_string()
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn relative_label(cwd: &Path, path: &Path) -> String {
    let cwd = normalize_path(cwd);
    let path = normalize_path(path);
    path.strip_prefix(&cwd)
        .map(|path| path.to_string_lossy().to_string())
        .unwrap_or_else(|_| path.to_string_lossy().to_string())
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut normalized = if path.is_absolute() {
        PathBuf::new()
    } else {
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
    };

    for component in path.components() {
        match component {
            Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            Component::RootDir => normalized.push(Path::new("/")),
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            Component::Normal(part) => normalized.push(part),
        }
    }
    normalized
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn actuator_examples_match_expected_results() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let temp = tempfile::tempdir().unwrap();
        let temp_examples = temp.path().join("examples");
        copy_dir_all(repo_root.join("examples"), &temp_examples).unwrap();

        let bad_dir = temp_examples.join("linear-actuator-bad");
        let good_dir = temp_examples.join("linear-actuator-good");
        let bad_path = bad_dir.join(DESIGN_DATA_FILE_NAME);
        let good_path = good_dir.join(DESIGN_DATA_FILE_NAME);
        let cwd = temp.path().to_path_buf();

        stamp_targets(
            &[
                bad_dir.to_string_lossy().to_string(),
                good_dir.to_string_lossy().to_string(),
            ],
            &cwd,
        )
        .unwrap();

        let options = LintOptions {
            cwd,
            write_receipt: true,
            rulepack_path: None,
        };
        let bad = lint_design_data_file(&bad_path, &options).unwrap();
        assert_eq!(string_field(&bad.receipt, "status"), Some("fail"));
        assert!(bad.receipt["checks"]
            .as_array()
            .unwrap()
            .iter()
            .any(|check| {
                string_field(check, "rule_id")
                    == Some("actuator_mount:m3_loaded_hole_edge_distance")
                    && string_field(check, "reason") == Some("insufficient_edge_distance")
                    && check
                        .pointer("/measured/center_to_edge_mm")
                        .and_then(Value::as_f64)
                        == Some(8.0)
                    && check
                        .pointer("/required/center_to_edge_mm")
                        .and_then(Value::as_f64)
                        == Some(10.2)
            }));
        assert!(format_receipt_diagnostics(&bad.receipt)
            .iter()
            .flatten()
            .any(|line| line.contains("Short by: 2.2 mm")));

        let good = lint_design_data_file(&good_path, &options).unwrap();
        assert_eq!(string_field(&good.receipt, "status"), Some("pass"));
        assert_eq!(
            string_field(&good.receipt, "schema_version"),
            Some(RECEIPT_SCHEMA_VERSION)
        );
        assert_eq!(
            string_field(&good.receipt, "burr_version"),
            Some(BURR_VERSION)
        );
        assert_eq!(
            string_field(&good.receipt, "artifact_version"),
            Some("0.1.0")
        );
        assert_eq!(
            string_field(&good.receipt, "rulepack_version"),
            Some("0.1.0")
        );
    }

    fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> std::io::Result<()> {
        fs::create_dir_all(&dst)?;
        for entry in fs::read_dir(src)? {
            let entry = entry?;
            let ty = entry.file_type()?;
            if ty.is_dir() {
                copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
            } else {
                fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
            }
        }
        Ok(())
    }
}
