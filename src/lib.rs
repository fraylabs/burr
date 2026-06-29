use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

pub const BURR_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const DESIGN_DATA_FILE_NAME: &str = "burr-design-data.json";
pub const LEGACY_DESIGN_DATA_FILE_NAMES: [&str; 1] = ["fray-cad.json"];
pub const SUPPORTED_DESIGN_DATA_SCHEMA_VERSIONS: [&str; 1] = ["burr.design-data.v1"];
pub const SUPPORTED_LEGACY_DESIGN_DATA_SCHEMA_VERSIONS: [&str; 1] = ["fray.cad.artifact.v1"];
pub const SUPPORTED_RULEPACK_SCHEMA_VERSIONS: [&str; 1] = ["burr.rulepack.v1"];
pub const RECEIPT_SCHEMA_VERSION: &str = "burr.receipt.v1";
pub const BURR_BUILD123D_PYPI_DEPENDENCY: &str = "burr-build123d==0.10.0";

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

pub fn init_project(project_dir: &Path) -> Result<Vec<PathBuf>, String> {
    if project_dir.exists() && !project_dir.is_dir() {
        return Err(format!(
            "Init target exists and is not a directory: {}",
            project_dir.display()
        ));
    }

    fs::create_dir_all(project_dir)
        .map_err(|error| format!("Failed to create {}: {error}", project_dir.display()))?;

    let project_name = project_name_from_dir(project_dir);
    let files = [
        (
            project_dir.join("pyproject.toml"),
            starter_pyproject(&project_name),
        ),
        (project_dir.join("design.py"), starter_design(&project_name)),
        (project_dir.join(".gitignore"), starter_gitignore()),
    ];

    for (path, _) in &files {
        if path.exists() {
            return Err(format!(
                "Refusing to overwrite existing file: {}",
                path.display()
            ));
        }
    }

    let mut written = Vec::new();
    for (path, contents) in files {
        fs::write(&path, contents)
            .map_err(|error| format!("Failed to write {}: {error}", path.display()))?;
        written.push(path);
    }

    Ok(written)
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
    let manifest = read_json_file(path)?;
    let manifest_dir = path.parent().unwrap_or_else(|| Path::new("."));
    let rulepack = match &options.rulepack_path {
        Some(path) => read_json_file(path)?,
        None => match design_data_rulepack_path(&manifest, manifest_dir)? {
            Some(path) => read_json_file(&path)?,
            None => default_rulepack()?,
        },
    };
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
            let rule_kind = string_field(rule, "kind");

            if !matches!(
                rule_kind,
                Some("hole_edge_distance")
                    | Some("minimum_wall_thickness")
                    | Some("fastener_support_wall_thickness")
                    | Some("blind_pocket_back_wall_thickness")
                    | Some("standoff_boss_support_link")
                    | Some("feature_presence")
                    | Some("feature_count")
                    | Some("feature_edge_distance")
                    | Some("feature_pair_spacing")
                    | Some("numeric_range")
            ) {
                warnings.push(json!({
                    "rule_id": format!("{}:{}", string_field(rulepack, "id").unwrap_or("<missing>"), string_field(rule, "id").unwrap_or("<missing>")),
                    "status": "warn",
                    "reason": "unsupported_rule_kind",
                    "message": format!("Unsupported rule kind {}.", rule_kind.unwrap_or("<missing>"))
                }));
                continue;
            }

            if rule_kind == Some("feature_count") {
                checks.push(check_feature_count(manifest, rulepack, rule));
                continue;
            }

            if rule_kind == Some("numeric_range") {
                checks.push(check_numeric_range(manifest, rulepack, rule));
                continue;
            }

            if rule_kind == Some("feature_pair_spacing") {
                checks.push(check_feature_pair_spacing(manifest, rulepack, rule));
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
                warnings.push(json!({
                    "rule_id": format!("{}:{}", string_field(rulepack, "id").unwrap_or("<missing>"), string_field(rule, "id").unwrap_or("<missing>")),
                    "status": "warn",
                    "reason": "no_applicable_features",
                    "message": "No applicable features were found for this rule."
                }));
                continue;
            }

            for feature in features {
                match rule_kind {
                    Some("hole_edge_distance") => {
                        checks.push(check_hole_edge_distance(manifest, rulepack, rule, feature));
                    }
                    Some("minimum_wall_thickness") => {
                        checks.push(check_minimum_wall_thickness(
                            manifest, rulepack, rule, feature,
                        ));
                    }
                    Some("fastener_support_wall_thickness") => {
                        checks.push(check_fastener_support_wall_thickness(
                            rulepack, rule, feature,
                        ));
                    }
                    Some("blind_pocket_back_wall_thickness") => {
                        checks.push(check_blind_pocket_back_wall_thickness(
                            manifest, rulepack, rule, feature,
                        ));
                    }
                    Some("standoff_boss_support_link") => {
                        checks.push(check_standoff_boss_support_link(
                            manifest, rulepack, rule, feature,
                        ));
                    }
                    Some("feature_edge_distance") => {
                        checks.push(check_feature_edge_distance(
                            manifest, rulepack, rule, feature,
                        ));
                    }
                    Some("feature_presence") => {
                        checks.push(check_feature_presence(
                            manifest,
                            manifest_dir,
                            rulepack,
                            rule,
                            feature,
                        ));
                    }
                    _ => unreachable!(),
                }
            }
        }
    }

    let failures = checks
        .iter()
        .filter(|check| string_field(check, "status") == Some("fail"))
        .count();
    let feature_summary = summarize_features(manifest, &checks);

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
            "warnings": warnings.len(),
            "features": feature_summary
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

pub fn format_receipt_explanations(receipt: &Value) -> Vec<Vec<String>> {
    let mut explanations: Vec<_> = receipt
        .get("checks")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .enumerate()
        .filter_map(|(index, check)| {
            format_check_explanation(check).map(|lines| (explanation_rank(check), index, lines))
        })
        .collect();

    explanations.sort_by_key(|(rank, index, _)| (*rank, *index));
    explanations
        .into_iter()
        .map(|(_, _, lines)| lines)
        .collect()
}

pub fn build_receipt_repair_packet(receipt: &Value) -> Value {
    let mut failures: Vec<_> = receipt
        .get("checks")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .enumerate()
        .filter(|(_, check)| string_field(check, "status") == Some("fail"))
        .map(|(index, check)| {
            let rule_id = string_field(check, "rule_id").unwrap_or("<unknown>");
            let reason = string_field(check, "reason").unwrap_or("<unknown>");
            let feature_kind = feature_kind_from_rule(rule_id);
            let source_hint = check.get("source_hint").cloned().unwrap_or(Value::Null);
            let has_exact_source_edit = source_hint.as_object().is_some_and(|hint| {
                hint.get("before_text").is_some() && hint.get("after_text").is_some()
            });

            json!({
                "rank": explanation_rank(check),
                "input_order": index,
                "rule_id": rule_id,
                "feature_id": check.get("feature_id").cloned().unwrap_or(Value::Null),
                "reason": reason,
                "category": explanation_category(reason),
                "headline": explanation_headline(reason, feature_kind),
                "problem": explanation_problem(check, reason, feature_kind),
                "evidence": explanation_evidence(check, reason)
                    .into_iter()
                    .map(|line| line.strip_prefix("Evidence: ").unwrap_or(&line).to_string())
                    .collect::<Vec<_>>(),
                "why": explanation_why(reason, feature_kind),
                "fix": explanation_fix(reason, feature_kind),
                "source_hint": source_hint,
                "exact_source_edit_available": has_exact_source_edit
            })
        })
        .collect();

    failures.sort_by_key(|failure| {
        (
            failure.get("rank").and_then(Value::as_u64).unwrap_or(9),
            failure
                .get("input_order")
                .and_then(Value::as_u64)
                .unwrap_or(u64::MAX),
        )
    });

    let exact_source_edits = failures
        .iter()
        .filter(|failure| {
            failure
                .get("exact_source_edit_available")
                .and_then(Value::as_bool)
                == Some(true)
        })
        .count();

    json!({
        "schema_version": "burr.repair-packet.v1",
        "burr_version": BURR_VERSION,
        "source_kind": "receipt",
        "source_design_data": receipt.get("source_design_data").cloned().unwrap_or(Value::Null),
        "status": receipt.get("status").and_then(Value::as_str).unwrap_or("<unknown>"),
        "summary": {
            "failure_count": failures.len(),
            "exact_source_edit_count": exact_source_edits,
            "exact_source_edits_available": exact_source_edits > 0
        },
        "failures": failures
    })
}

pub fn build_repair_report_packet(report: &Value) -> Value {
    let actions: Vec<Value> = report
        .get("repair_actions")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .cloned()
        .collect();
    let exact_source_edits = actions
        .iter()
        .filter(|action| {
            action
                .get("source_hint")
                .and_then(Value::as_object)
                .is_some_and(|hint| {
                    hint.get("before_text").is_some() && hint.get("after_text").is_some()
                })
        })
        .count();

    json!({
        "schema_version": "burr.repair-packet.v1",
        "burr_version": BURR_VERSION,
        "source_kind": "repair_report",
        "repair_report_id": report.get("report_id").or_else(|| report.get("id")).cloned().unwrap_or(Value::Null),
        "focus_rule_id": report.get("focus_rule_id").cloned().unwrap_or(Value::Null),
        "status": report.get("status").and_then(Value::as_str).unwrap_or("<unknown>"),
        "summary": {
            "failure_count": report.pointer("/summary/before_failures").cloned().unwrap_or(Value::Null),
            "repair_action_count": actions.len(),
            "exact_source_edit_count": exact_source_edits,
            "exact_source_edits_available": exact_source_edits > 0
        },
        "first_fix": report.get("first_fix").cloned().unwrap_or(Value::Null),
        "failures": report.get("failures").cloned().unwrap_or_else(|| json!([])),
        "repair_actions": actions
    })
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
        Some("insufficient_feature_edge_distance") => {
            let feature_label = string_field(check, "feature_id")
                .map(|id| format!(" {id}"))
                .unwrap_or_default();
            let measured = check
                .pointer("/measured/wall_to_edge_mm")
                .and_then(Value::as_f64);
            let required = check
                .pointer("/required/min_wall_to_edge_mm")
                .and_then(Value::as_f64);
            let short_by = number_field(check, "margin_mm").map(|value| round(value.abs()));
            let mut lines = vec![format!("Feature{feature_label} is too close to the edge.")];
            if let Some(value) = measured {
                lines.push(format!(
                    "Measured feature-to-edge: {} mm",
                    trim_float(value)
                ));
            }
            if let Some(value) = required {
                lines.push(format!(
                    "Required feature-to-edge: {} mm",
                    trim_float(value)
                ));
            }
            if let Some(value) = short_by {
                lines.push(format!("Short by: {} mm", trim_float(value)));
            }
            lines.push(
                "Try moving the feature inward, shortening it, or increasing the surrounding part size."
                    .to_string(),
            );
            Some(lines)
        }
        Some("insufficient_wall_thickness") => {
            let feature_label = string_field(check, "feature_id")
                .map(|id| format!(" {id}"))
                .unwrap_or_default();
            let measured = check
                .pointer("/measured/wall_thickness_mm")
                .and_then(Value::as_f64);
            let required = check
                .pointer("/required/wall_thickness_mm")
                .and_then(Value::as_f64);
            let short_by = number_field(check, "margin_mm").map(|value| round(value.abs()));
            let mut lines = vec![format!(
                "M3 clearance hole{feature_label} leaves too little wall."
            )];
            if let Some(value) = measured {
                lines.push(format!("Measured wall thickness: {} mm", trim_float(value)));
            }
            if let Some(value) = required {
                lines.push(format!("Required wall thickness: {} mm", trim_float(value)));
            }
            if let Some(value) = short_by {
                lines.push(format!("Short by: {} mm", trim_float(value)));
            }
            lines.push("Try moving the hole inward or increasing part width.".to_string());
            Some(lines)
        }
        Some("insufficient_fastener_support_wall") => {
            let feature_label = string_field(check, "feature_id")
                .map(|id| format!(" {id}"))
                .unwrap_or_default();
            let measured = check
                .pointer("/measured/support_wall_thickness_mm")
                .and_then(Value::as_f64);
            let required = check
                .pointer("/required/wall_thickness_mm")
                .and_then(Value::as_f64);
            let short_by = number_field(check, "margin_mm").map(|value| round(value.abs()));
            let mut lines = vec![format!(
                "Fastener support{feature_label} leaves too little boss material."
            )];
            if let Some(value) = measured {
                lines.push(format!("Measured support wall: {} mm", trim_float(value)));
            }
            if let Some(value) = required {
                lines.push(format!("Required support wall: {} mm", trim_float(value)));
            }
            if let Some(value) = short_by {
                lines.push(format!("Short by: {} mm", trim_float(value)));
            }
            lines.push(
                "Try increasing the support or boss diameter around the fastener.".to_string(),
            );
            Some(lines)
        }
        Some("insufficient_blind_pocket_back_wall") => {
            let feature_label = string_field(check, "feature_id")
                .map(|id| format!(" {id}"))
                .unwrap_or_default();
            let measured = check
                .pointer("/measured/back_wall_thickness_mm")
                .and_then(Value::as_f64);
            let required = check
                .pointer("/required/min_back_wall_thickness_mm")
                .and_then(Value::as_f64);
            let short_by = number_field(check, "margin_mm").map(|value| round(value.abs()));
            let mut lines = vec![format!(
                "Blind pocket{feature_label} leaves too little back wall."
            )];
            if let Some(value) = measured {
                lines.push(format!("Measured back wall: {} mm", trim_float(value)));
            }
            if let Some(value) = required {
                lines.push(format!("Required back wall: {} mm", trim_float(value)));
            }
            if let Some(value) = short_by {
                lines.push(format!("Short by: {} mm", trim_float(value)));
            }
            lines.push(
                "Try making the host deeper, reducing pocket depth, or moving the pocket shallower."
                    .to_string(),
            );
            Some(lines)
        }
        Some("standoff_boss_support_mismatch") => {
            let feature_label = string_field(check, "feature_id")
                .map(|id| format!(" {id}"))
                .unwrap_or_default();
            let supported_label = check
                .pointer("/measured/supported_feature_id")
                .and_then(Value::as_str)
                .map(|id| format!(" to {id}"))
                .unwrap_or_default();
            let centerline_distance = check
                .pointer("/measured/centerline_distance_mm")
                .and_then(Value::as_f64);
            let axis_dot = check.pointer("/measured/axis_dot").and_then(Value::as_f64);
            let mut lines = vec![format!(
                "Standoff boss{feature_label} is not aligned{supported_label}."
            )];
            if let Some(value) = centerline_distance {
                lines.push(format!(
                    "Measured centerline offset: {} mm",
                    trim_float(value)
                ));
            }
            if let Some(value) = check
                .pointer("/required/centerline_tolerance_mm")
                .and_then(Value::as_f64)
            {
                lines.push(format!(
                    "Allowed centerline offset: {} mm",
                    trim_float(value)
                ));
            }
            if let Some(value) = axis_dot {
                lines.push(format!("Measured axis alignment: {}", trim_float(value)));
            }
            lines.push(
                "Try aligning the standoff boss center and axis to the supported hole or insert."
                    .to_string(),
            );
            Some(lines)
        }
        Some("missing_standoff_support_link")
        | Some("missing_supported_feature")
        | Some("unsupported_standoff_support_kind") => {
            let feature_label = string_field(check, "feature_id")
                .map(|id| format!(" {id}"))
                .unwrap_or_default();
            let mut lines = vec![format!(
                "Standoff boss{feature_label} is not linked to a supported feature."
            )];
            if let Some(id) = check
                .pointer("/measured/supported_feature_id")
                .and_then(Value::as_str)
                .or_else(|| string_field(check, "related_feature_id"))
            {
                lines.push(format!("Declared support link: {id}"));
            }
            if let Some(kind) = check
                .pointer("/measured/supported_feature_kind")
                .and_then(Value::as_str)
            {
                lines.push(format!("Linked feature kind: {kind}"));
            }
            if let Some(message) = string_field(check, "message") {
                lines.push(message.to_string());
            }
            Some(lines)
        }
        Some("missing_declared_feature") => {
            let feature_label = string_field(check, "feature_id")
                .map(|id| format!(" {id}"))
                .unwrap_or_default();
            let feature_kind = string_field(check, "rule_id").and_then(|rule_id| {
                if rule_id.contains("straight_slot") {
                    Some("straight slot")
                } else if rule_id.contains("counterbore") {
                    Some("counterbore")
                } else if rule_id.contains("heat_set_insert_pocket") {
                    Some("heat-set insert pocket")
                } else if rule_id.contains("bearing_seat") {
                    Some("bearing seat")
                } else if rule_id.contains("standoff_boss") {
                    Some("standoff boss")
                } else {
                    None
                }
            });
            let artifact = check
                .pointer("/measured/artifact_path")
                .and_then(Value::as_str)
                .unwrap_or("<missing STEP>");
            let candidates = check
                .pointer("/measured/candidate_cylinders")
                .and_then(Value::as_u64);
            let mut lines = if let Some(feature_kind) = feature_kind {
                vec![format!(
                    "Declared {feature_kind}{feature_label} is missing from the STEP artifact."
                )]
            } else {
                vec![format!(
                    "Declared clearance hole{feature_label} is missing from the STEP artifact."
                )]
            };
            lines.push(format!("Checked artifact: {artifact}"));
            if let Some(value) = candidates {
                lines.push(format!("Candidate cylinders found: {value}"));
            }
            if let Some(value) = check
                .pointer("/measured/candidate_planes")
                .and_then(Value::as_u64)
            {
                lines.push(format!("Candidate planes found: {value}"));
            }
            if let Some(value) = check
                .pointer("/measured/matched_slot_side_planes")
                .and_then(Value::as_u64)
            {
                lines.push(format!("Matched slot side planes: {value}"));
            }
            lines.push(
                "Regenerate the STEP from the same helper that emitted the design data."
                    .to_string(),
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

fn format_check_explanation(check: &Value) -> Option<Vec<String>> {
    if string_field(check, "status") != Some("fail") {
        return None;
    }

    let rule_id = string_field(check, "rule_id").unwrap_or("<unknown>");
    let reason = string_field(check, "reason").unwrap_or("<unknown>");
    let feature_id = string_field(check, "feature_id").unwrap_or("<none>");
    let feature_kind = feature_kind_from_rule(rule_id);

    let mut lines = vec![
        explanation_headline(reason, feature_kind).to_string(),
        format!("Feature: {feature_id}"),
        format!("Rule: {rule_id}"),
        format!("Category: {}", explanation_category(reason)),
        format!(
            "Problem: {}",
            explanation_problem(check, reason, feature_kind)
        ),
    ];

    lines.extend(explanation_evidence(check, reason));
    lines.push(format!(
        "Why it matters: {}",
        explanation_why(reason, feature_kind)
    ));
    lines.push(format!("Fix: {}", explanation_fix(reason, feature_kind)));
    Some(lines)
}

fn explanation_rank(check: &Value) -> u8 {
    explanation_rank_for_reason(string_field(check, "reason").unwrap_or("<unknown>"))
}

fn explanation_category(reason: &str) -> &'static str {
    match reason {
        "source_hash_mismatch"
        | "artifact_hash_mismatch"
        | "hash_read_failed"
        | "missing_file_refs"
        | "invalid_sha256"
        | "missing_sha256"
        | "source_file_missing"
        | "artifact_file_missing"
        | "missing_step_artifact_ref" => "stale artifact",
        "unsupported_design_data_schema"
        | "missing_design_data_schema"
        | "unsupported_rulepack_schema"
        | "missing_rulepack_schema" => "unsupported metadata",
        "missing_declared_feature" | "step_geometry_unreadable" => "missing geometry",
        "insufficient_edge_distance"
        | "insufficient_feature_edge_distance"
        | "insufficient_feature_pair_spacing"
        | "insufficient_wall_thickness"
        | "insufficient_fastener_support_wall"
        | "insufficient_blind_pocket_back_wall"
        | "standoff_boss_support_mismatch"
        | "missing_edge_measurement"
        | "missing_hole_diameter"
        | "missing_pair_spacing_geometry"
        | "missing_supported_feature"
        | "unsupported_standoff_support_kind"
        | "missing_standoff_support_link"
        | "missing_standoff_boss_diameter"
        | "missing_supported_feature_diameter"
        | "missing_fastener_support_inner_diameter"
        | "missing_fastener_support_diameter"
        | "missing_blind_pocket_back_wall_geometry"
        | "missing_feature_center"
        | "missing_feature_axis"
        | "missing_feature_edge_geometry"
        | "missing_feature_part_bbox"
        | "unsupported_blind_pocket_axis"
        | "invalid_rule_min_back_wall_thickness"
        | "invalid_feature_edge_rule_clearance"
        | "invalid_pair_spacing_rule_clearance"
        | "invalid_counterbore_dimensions" => "unsafe dimension",
        "feature_count_out_of_range" | "numeric_value_out_of_range" | "missing_numeric_value" => {
            "declared measurement"
        }
        _ => "other",
    }
}

fn explanation_headline(reason: &str, feature_kind: &str) -> &'static str {
    match explanation_rank_for_reason(reason) {
        0 => "Fix first: regenerate or restamp stale CAD artifacts.",
        1 => match feature_kind {
            "bearing seat" => "Fix geometry: regenerate the missing bearing seat.",
            "counterbore" => "Fix geometry: regenerate the missing counterbore.",
            "heat-set insert pocket" => {
                "Fix geometry: regenerate the missing heat-set insert pocket."
            }
            "straight slot" => "Fix geometry: regenerate the missing straight slot.",
            _ => "Fix geometry: regenerate the missing declared feature.",
        },
        2 => "Fix dimension: move or resize unsafe geometry.",
        3 => "Fix declared measurement: update the CAD or rule range.",
        _ => "Fix check input: inspect the failed rule.",
    }
}

fn explanation_rank_for_reason(reason: &str) -> u8 {
    match reason {
        "source_hash_mismatch"
        | "artifact_hash_mismatch"
        | "hash_read_failed"
        | "missing_file_refs"
        | "invalid_sha256"
        | "missing_sha256"
        | "source_file_missing"
        | "artifact_file_missing"
        | "missing_step_artifact_ref"
        | "unsupported_design_data_schema"
        | "missing_design_data_schema"
        | "unsupported_rulepack_schema"
        | "missing_rulepack_schema" => 0,
        "missing_declared_feature" | "step_geometry_unreadable" => 1,
        "insufficient_edge_distance"
        | "insufficient_feature_edge_distance"
        | "insufficient_feature_pair_spacing"
        | "insufficient_wall_thickness"
        | "insufficient_fastener_support_wall"
        | "insufficient_blind_pocket_back_wall"
        | "standoff_boss_support_mismatch"
        | "missing_edge_measurement"
        | "missing_hole_diameter"
        | "missing_pair_spacing_geometry"
        | "missing_supported_feature"
        | "unsupported_standoff_support_kind"
        | "missing_standoff_support_link"
        | "missing_standoff_boss_diameter"
        | "missing_supported_feature_diameter"
        | "missing_fastener_support_inner_diameter"
        | "missing_fastener_support_diameter"
        | "missing_blind_pocket_back_wall_geometry"
        | "missing_feature_center"
        | "missing_feature_axis"
        | "missing_feature_edge_geometry"
        | "missing_feature_part_bbox"
        | "unsupported_blind_pocket_axis"
        | "invalid_rule_min_back_wall_thickness"
        | "invalid_feature_edge_rule_clearance"
        | "invalid_pair_spacing_rule_clearance"
        | "invalid_counterbore_dimensions" => 2,
        "feature_count_out_of_range" | "numeric_value_out_of_range" | "missing_numeric_value" => 3,
        _ => 9,
    }
}

fn feature_kind_from_rule(rule_id: &str) -> &'static str {
    if rule_id.contains("straight_slot") || rule_id.contains("_slot") {
        "straight slot"
    } else if rule_id.contains("counterbore") {
        "counterbore"
    } else if rule_id.contains("heat_set_insert_pocket") {
        "heat-set insert pocket"
    } else if rule_id.contains("bearing_seat") {
        "bearing seat"
    } else if rule_id.contains("standoff_boss") {
        "standoff boss"
    } else if rule_id.contains("clearance_hole") || rule_id.contains("loaded_hole") {
        "clearance hole"
    } else {
        "feature"
    }
}

fn explanation_problem(check: &Value, reason: &str, feature_kind: &str) -> String {
    match reason {
        "insufficient_edge_distance" => {
            "the loaded M3 hole is too close to a free edge.".to_string()
        }
        "insufficient_feature_edge_distance" => {
            format!("the declared {feature_kind} is too close to a free edge.")
        }
        "insufficient_feature_pair_spacing" => {
            "two declared features leave too little material between them.".to_string()
        }
        "insufficient_wall_thickness" => {
            "the M3 clearance hole leaves too little printable wall.".to_string()
        }
        "insufficient_fastener_support_wall" => {
            "the fastener support or boss leaves too little radial material around the hole."
                .to_string()
        }
        "insufficient_blind_pocket_back_wall" => {
            "the blind insert pocket leaves too little material behind its bottom.".to_string()
        }
        "standoff_boss_support_mismatch" => {
            "the standoff boss is not aligned with the hole or insert it claims to support."
                .to_string()
        }
        "missing_standoff_support_link" => {
            "the standoff boss does not declare supports_feature_id.".to_string()
        }
        "missing_supported_feature" => {
            "the standoff boss points to a supported feature id that does not exist.".to_string()
        }
        "unsupported_standoff_support_kind" => {
            "the standoff boss points to a feature kind this rule cannot compare.".to_string()
        }
        "missing_standoff_boss_diameter" => {
            "the standoff boss is missing boss_diameter_mm.".to_string()
        }
        "missing_supported_feature_diameter" => {
            "the supported hole or insert is missing its inner diameter.".to_string()
        }
        "missing_declared_feature" => {
            format!("the design data declares a {feature_kind}, but Burr cannot find matching STEP geometry.")
        }
        "source_hash_mismatch" | "artifact_hash_mismatch" => {
            "the receipt was made from stale file hashes.".to_string()
        }
        "unsupported_design_data_schema" | "missing_design_data_schema" => {
            "the design data schema is not supported by this Burr version.".to_string()
        }
        "unsupported_rulepack_schema" | "missing_rulepack_schema" => {
            "the rulepack schema is not supported by this Burr version.".to_string()
        }
        "missing_hole_diameter" => "the feature is missing a valid hole diameter.".to_string(),
        "missing_pair_spacing_geometry" => {
            "a feature in a pair-spacing rule is missing center_mm or diameter_mm.".to_string()
        }
        "missing_feature_edge_geometry" => {
            "the feature is missing a checkable edge-distance envelope.".to_string()
        }
        "missing_feature_part_bbox" => {
            "the feature's part is missing a bbox_mm free-edge envelope.".to_string()
        }
        "invalid_feature_edge_rule_clearance" => {
            "the edge-distance rule is missing a valid min_wall_to_edge_mm.".to_string()
        }
        "invalid_pair_spacing_rule_clearance" => {
            "the pair-spacing rule is missing a valid min_clearance_mm.".to_string()
        }
        "missing_fastener_support_inner_diameter" => {
            "the feature is missing the hole, pocket, or bore diameter used inside the support."
                .to_string()
        }
        "missing_fastener_support_diameter" => {
            "the feature is missing the declared support or boss outer diameter.".to_string()
        }
        "missing_blind_pocket_back_wall_geometry" => {
            "the pocket is missing part, axis, pocket_center_mm, bottom_center_mm, or bbox metadata."
                .to_string()
        }
        "unsupported_blind_pocket_axis" => {
            "the blind pocket bottom direction is not aligned to a supported bbox axis.".to_string()
        }
        "invalid_rule_min_back_wall_thickness" => {
            "the rule is missing a valid min_back_wall_thickness_mm.".to_string()
        }
        "missing_feature_center" => "the feature is missing center_mm.".to_string(),
        "missing_feature_axis" => "the feature is missing axis.".to_string(),
        "step_geometry_unreadable" => "Burr could not read STEP geometry evidence.".to_string(),
        "invalid_counterbore_dimensions" => {
            "the counterbore dimensions are internally invalid.".to_string()
        }
        "feature_count_out_of_range" => {
            "the number of matching declared features is outside the allowed range.".to_string()
        }
        "numeric_value_out_of_range" => {
            "the declared numeric measurement is outside the allowed range.".to_string()
        }
        "missing_numeric_value" => {
            "the rule points at a numeric measurement that is missing or invalid.".to_string()
        }
        _ => string_field(check, "message")
            .unwrap_or("the check failed.")
            .to_string(),
    }
}

fn explanation_evidence(check: &Value, reason: &str) -> Vec<String> {
    let mut lines = Vec::new();
    match reason {
        "insufficient_edge_distance" => {
            push_measure(
                &mut lines,
                check,
                "/measured/center_to_edge_mm",
                "Measured center-to-edge",
            );
            push_measure(
                &mut lines,
                check,
                "/required/center_to_edge_mm",
                "Required center-to-edge",
            );
            push_margin(&mut lines, check);
        }
        "insufficient_feature_edge_distance" => {
            push_measure(
                &mut lines,
                check,
                "/measured/wall_to_edge_mm",
                "Measured feature-to-edge",
            );
            push_measure(
                &mut lines,
                check,
                "/required/min_wall_to_edge_mm",
                "Required feature-to-edge",
            );
            push_margin(&mut lines, check);
        }
        "insufficient_feature_pair_spacing" => {
            push_measure(
                &mut lines,
                check,
                "/measured/closest_pair/center_distance_mm",
                "Closest center distance",
            );
            push_measure(
                &mut lines,
                check,
                "/measured/closest_pair/clearance_mm",
                "Closest ligament",
            );
            push_measure(
                &mut lines,
                check,
                "/required/min_clearance_mm",
                "Required ligament",
            );
            push_margin(&mut lines, check);
        }
        "insufficient_wall_thickness" => {
            push_measure(
                &mut lines,
                check,
                "/measured/wall_thickness_mm",
                "Measured wall thickness",
            );
            push_measure(
                &mut lines,
                check,
                "/required/wall_thickness_mm",
                "Required wall thickness",
            );
            push_margin(&mut lines, check);
        }
        "insufficient_fastener_support_wall" => {
            push_measure(
                &mut lines,
                check,
                "/measured/inner_diameter_mm",
                "Measured inner diameter",
            );
            push_measure(
                &mut lines,
                check,
                "/measured/support_diameter_mm",
                "Measured support diameter",
            );
            push_measure(
                &mut lines,
                check,
                "/measured/support_wall_thickness_mm",
                "Measured support wall",
            );
            push_measure(
                &mut lines,
                check,
                "/required/wall_thickness_mm",
                "Required support wall",
            );
            push_margin(&mut lines, check);
        }
        "insufficient_blind_pocket_back_wall" => {
            push_measure(
                &mut lines,
                check,
                "/measured/back_wall_thickness_mm",
                "Measured back wall",
            );
            push_measure(
                &mut lines,
                check,
                "/required/min_back_wall_thickness_mm",
                "Required back wall",
            );
            push_margin(&mut lines, check);
        }
        "standoff_boss_support_mismatch" => {
            if let Some(id) = check
                .pointer("/measured/supported_feature_id")
                .and_then(Value::as_str)
            {
                lines.push(format!("Evidence: supported feature id = {id}."));
            }
            push_measure(
                &mut lines,
                check,
                "/measured/centerline_distance_mm",
                "Measured centerline offset",
            );
            push_measure(
                &mut lines,
                check,
                "/required/centerline_tolerance_mm",
                "Allowed centerline offset",
            );
            push_measure(&mut lines, check, "/measured/axis_dot", "Measured axis dot");
            push_measure(
                &mut lines,
                check,
                "/measured/support_diameter_delta_mm",
                "Measured support diameter delta",
            );
            push_measure(
                &mut lines,
                check,
                "/required/support_diameter_tolerance_mm",
                "Allowed support diameter delta",
            );
            push_margin(&mut lines, check);
        }
        "missing_declared_feature" => {
            if let Some(artifact) = check
                .pointer("/measured/artifact_path")
                .and_then(Value::as_str)
            {
                lines.push(format!("Evidence: checked STEP artifact {artifact}."));
            }
            if let Some(value) = check
                .pointer("/measured/candidate_cylinders")
                .and_then(Value::as_u64)
            {
                lines.push(format!("Evidence: candidate cylinders found = {value}."));
            }
            if let Some(value) = check
                .pointer("/measured/candidate_planes")
                .and_then(Value::as_u64)
            {
                lines.push(format!("Evidence: candidate planes found = {value}."));
            }
            push_bool_evidence(check, &mut lines, "matched_hole", "matched hole cylinder");
            push_bool_evidence(
                check,
                &mut lines,
                "matched_slot_endpoints",
                "matched slot endpoints",
            );
            push_bool_evidence(
                check,
                &mut lines,
                "matched_bore_cylinder",
                "matched bore cylinder",
            );
            push_bool_evidence(
                check,
                &mut lines,
                "matched_counterbore_cylinder",
                "matched counterbore cylinder",
            );
            push_bool_evidence(
                check,
                &mut lines,
                "matched_shoulder_plane",
                "matched shoulder plane",
            );
            push_bool_evidence(
                check,
                &mut lines,
                "matched_pocket_cylinder",
                "matched pocket cylinder",
            );
            push_bool_evidence(
                check,
                &mut lines,
                "matched_bottom_plane",
                "matched bottom plane",
            );
            push_bool_evidence(
                check,
                &mut lines,
                "matched_seat_cylinder",
                "matched seat cylinder",
            );
            push_bool_evidence(
                check,
                &mut lines,
                "matched_seat_shoulder_plane",
                "matched bearing shoulder plane",
            );
            push_bool_evidence(
                check,
                &mut lines,
                "matched_boss_cylinder",
                "matched boss cylinder",
            );
            push_bool_evidence(
                check,
                &mut lines,
                "matched_boss_top_plane",
                "matched boss top plane",
            );
        }
        "source_hash_mismatch" | "artifact_hash_mismatch" => {
            if let Some(path) = string_field(check, "path") {
                lines.push(format!("Evidence: stale path {path}."));
            }
        }
        "feature_count_out_of_range" => {
            push_count(&mut lines, check, "/measured/count", "Measured count");
            push_count(&mut lines, check, "/required/min_count", "Minimum count");
            push_count(&mut lines, check, "/required/max_count", "Maximum count");
        }
        "numeric_value_out_of_range" | "missing_numeric_value" => {
            if let Some(path) = string_field(check, "path") {
                lines.push(format!("Evidence: checked numeric path {path}."));
            }
            push_measure(&mut lines, check, "/measured/value", "Measured value");
            push_measure(&mut lines, check, "/required/min", "Minimum value");
            push_measure(&mut lines, check, "/required/max", "Maximum value");
        }
        _ => {
            if let Some(message) = string_field(check, "message") {
                lines.push(format!("Evidence: {message}"));
            }
        }
    }
    lines
}

fn explanation_why(reason: &str, feature_kind: &str) -> &'static str {
    match reason {
        "insufficient_edge_distance" => {
            "thin edge material can crack, delaminate, or fail when the fastener is loaded."
        }
        "insufficient_feature_edge_distance" => {
            "thin edge material can crack, delaminate, warp, or disappear around functional cuts."
        }
        "insufficient_feature_pair_spacing" => {
            "thin ligaments between holes can crack, delaminate, or disappear during printing."
        }
        "insufficient_wall_thickness" => {
            "FDM prints need enough material around holes to form reliable perimeters."
        }
        "insufficient_fastener_support_wall" => {
            "a boss or local support with too little radial material can split around the fastener or insert."
        }
        "insufficient_blind_pocket_back_wall" => {
            "too little material behind a blind insert pocket can crack, bulge, or break through during insert installation."
        }
        "standoff_boss_support_mismatch" => {
            "a boss only supports a fastener if its centerline and axis line up with the fastener feature."
        }
        "missing_declared_feature" => {
            match feature_kind {
                "bearing seat" => "a declared bearing fit is only trustworthy if the exported STEP contains the seat cylinder and shoulder.",
                "counterbore" => "a screw head fit is only trustworthy if the exported STEP contains the bore, counterbore, and shoulder.",
                "heat-set insert pocket" => "an insert fit is only trustworthy if the exported STEP contains the blind pocket wall and bottom.",
                "standoff boss" => "a support boss is only trustworthy if the exported STEP contains the raised boss cylinder and top face.",
                "straight slot" => "an adjustable slot is only trustworthy if the exported STEP contains the slot endpoints and side faces.",
                _ => "metadata alone is not enough; the exported STEP must contain the declared mechanical feature.",
            }
        }
        "source_hash_mismatch" | "artifact_hash_mismatch" => {
            "stale hashes mean the receipt may not describe the files currently on disk."
        }
        "feature_count_out_of_range" => {
            "declared feature inventory is part of the design contract for this artifact."
        }
        "numeric_value_out_of_range" | "missing_numeric_value" => {
            "Burr cannot trust a clearance, engagement, or other derived claim unless the source declares it in range."
        }
        _ => "Burr cannot trust this mechanical claim until the failing rule is fixed.",
    }
}

fn explanation_fix(reason: &str, feature_kind: &str) -> &'static str {
    match reason {
        "insufficient_edge_distance" => "move the hole inward or make the surrounding part larger.",
        "insufficient_feature_edge_distance" => match feature_kind {
            "counterbore" => {
                "move the counterbore inward, reduce the counterbore diameter, or make the surrounding part larger."
            }
            _ => {
                "move the feature inward, shorten the feature, or make the surrounding part larger."
            }
        },
        "insufficient_feature_pair_spacing" => "move the features farther apart, reduce their diameters, or remove one feature.",
        "insufficient_wall_thickness" => "move the hole inward, reduce the hole size, or increase the local wall.",
        "insufficient_fastener_support_wall" => "increase support_diameter_mm or boss_diameter_mm, reduce the inner hole/pocket size, or change the support intent.",
        "insufficient_blind_pocket_back_wall" => "make the host deeper, reduce pocket_depth_mm, or move the pocket shallower.",
        "standoff_boss_support_mismatch" => "align the standoff boss center_mm, boss_center_mm, and top_center_mm to the supported feature center, or correct supports_feature_id.",
        "missing_standoff_support_link" => "set supports_feature_id on the standoff_boss feature.",
        "missing_supported_feature" => "declare the supported hole or insert feature, or correct supports_feature_id.",
        "unsupported_standoff_support_kind" => "point supports_feature_id at a clearance_hole or heat_set_insert_pocket.",
        "missing_standoff_boss_diameter" => "set boss_diameter_mm on the standoff_boss feature.",
        "missing_supported_feature_diameter" => "set diameter_mm or pocket_diameter_mm on the supported feature.",
        "missing_declared_feature" => match feature_kind {
            "bearing seat" => "regenerate the STEP from the bearing_seat helper or update the declared seat center/diameter/depth.",
            "counterbore" => "regenerate the STEP from the counterbore helper or update the declared bore/counterbore dimensions.",
            "heat-set insert pocket" => "regenerate the STEP from the heat_set_insert_pocket helper or update the declared pocket dimensions.",
            "standoff boss" => "regenerate the STEP from the standoff_boss helper or update the declared boss diameter/height/center.",
            "straight slot" => "regenerate the STEP from the straight_slot helper or update the declared slot width/length/center.",
            _ => "regenerate the STEP from the same helper that emitted the design data, then rerun burr check.",
        },
        "source_hash_mismatch" | "artifact_hash_mismatch" => "rerun the CAD generator or burr stamp, then rerun burr check.",
        "unsupported_design_data_schema" | "missing_design_data_schema" => {
            "regenerate design data with a supported burr-build123d version."
        }
        "unsupported_rulepack_schema" | "missing_rulepack_schema" => {
            "use a rulepack schema supported by this Burr release."
        }
        "missing_hole_diameter" => "add a positive diameter_mm to the feature metadata.",
        "missing_pair_spacing_geometry" => {
            "add center_mm and diameter_mm to every feature selected by the pair-spacing rule."
        }
        "missing_feature_edge_geometry" => {
            "add center_mm plus diameter_mm, slot width/length/span_axis, or spacing_envelope metadata."
        }
        "missing_feature_part_bbox" => "add part and parts[].bbox_mm metadata for the feature's host part.",
        "invalid_feature_edge_rule_clearance" => {
            "set min_wall_to_edge_mm to the minimum material between the feature envelope and a free edge."
        }
        "invalid_pair_spacing_rule_clearance" => {
            "set min_clearance_mm to the minimum material bridge this rule should require."
        }
        "missing_fastener_support_inner_diameter" => {
            "add diameter_mm, pocket_diameter_mm, or bore_diameter_mm to the feature metadata."
        }
        "missing_fastener_support_diameter" => {
            "add support_diameter_mm, boss_diameter_mm, or outer_diameter_mm to the feature metadata."
        }
        "missing_feature_center" => "add center_mm to the feature metadata.",
        "missing_feature_axis" => "add axis to the feature metadata.",
        "feature_count_out_of_range" => {
            "add/remove declared features or adjust the rulepack range if the design intent changed."
        }
        "numeric_value_out_of_range" | "missing_numeric_value" => {
            "fix the CAD dimensions or emit the expected measurement in burr-design-data.json."
        }
        "step_geometry_unreadable" => "export a valid STEP artifact and make sure the design data points to it.",
        "invalid_counterbore_dimensions" => {
            "make counterbore_diameter_mm greater than bore_diameter_mm and use positive depths."
        }
        _ => "fix the rule input or generated geometry, then rerun burr check.",
    }
}

fn push_measure(lines: &mut Vec<String>, check: &Value, pointer: &str, label: &str) {
    if let Some(value) = check.pointer(pointer).and_then(Value::as_f64) {
        lines.push(format!("Evidence: {label} = {} mm.", trim_float(value)));
    }
}

fn push_count(lines: &mut Vec<String>, check: &Value, pointer: &str, label: &str) {
    if let Some(value) = check.pointer(pointer).and_then(Value::as_u64) {
        lines.push(format!("Evidence: {label} = {value}."));
    }
}

fn push_margin(lines: &mut Vec<String>, check: &Value) {
    if let Some(value) = number_field(check, "margin_mm") {
        lines.push(format!(
            "Evidence: short by {} mm.",
            trim_float(round(value.abs()))
        ));
    }
}

fn push_bool_evidence(check: &Value, lines: &mut Vec<String>, key: &str, label: &str) {
    if let Some(value) = check
        .pointer(&format!("/measured/{key}"))
        .and_then(Value::as_bool)
    {
        lines.push(format!("Evidence: {label} = {value}."));
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

fn check_minimum_wall_thickness(
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
            "message": "Hole diameter is required for wall-thickness linting."
        });
    }
    let diameter = diameter.unwrap();

    let required_wall_thickness = number_field(rule, "min_wall_thickness_mm");
    if !required_wall_thickness.is_some_and(|value| value > 0.0) {
        return json!({
            "rule_id": full_rule_id,
            "status": "fail",
            "reason": "invalid_rule_min_wall_thickness",
            "feature_id": feature.get("id").cloned().unwrap_or(Value::Null),
            "message": "Rule min_wall_thickness_mm must be a positive number."
        });
    }
    let required_wall_thickness = required_wall_thickness.unwrap();

    let center_to_edge = derive_center_to_edge_mm(manifest, feature);
    let Some(center_to_edge_value) = center_to_edge.value else {
        return json!({
            "rule_id": full_rule_id,
            "status": "fail",
            "reason": "missing_wall_thickness_measurement",
            "feature_id": feature.get("id").cloned().unwrap_or(Value::Null),
            "measured": { "wall_thickness_mm": Value::Null, "source": center_to_edge.source },
            "required": {
                "wall_thickness_mm": round(required_wall_thickness)
            },
            "message": "Nearest free-edge distance cannot be derived."
        });
    };

    let wall_thickness = center_to_edge_value - diameter / 2.0;
    let margin = wall_thickness - required_wall_thickness;
    let pass = margin >= 0.0;

    json!({
        "rule_id": full_rule_id,
        "status": if pass { "pass" } else { "fail" },
        "reason": if pass { "ok" } else { "insufficient_wall_thickness" },
        "feature_id": feature.get("id").cloned().unwrap_or(Value::Null),
        "measured": {
            "hole_diameter_mm": diameter,
            "center_to_edge_mm": round(center_to_edge_value),
            "wall_thickness_mm": round(wall_thickness),
            "source": center_to_edge.source
        },
        "required": {
            "wall_thickness_mm": round(required_wall_thickness)
        },
        "margin_mm": round(margin),
        "message": if pass {
            "Hole wall thickness passes rule.".to_string()
        } else {
            format!("Hole wall thickness is short by {} mm.", trim_float(round(margin.abs())))
        }
    })
}

fn check_fastener_support_wall_thickness(rulepack: &Value, rule: &Value, feature: &Value) -> Value {
    let full_rule_id = format!(
        "{}:{}",
        string_field(rulepack, "id").unwrap_or("<missing>"),
        string_field(rule, "id").unwrap_or("<missing>")
    );
    let feature_id = feature.get("id").cloned().unwrap_or(Value::Null);
    let fastener_diameter = number_field(feature, "diameter_mm")
        .or_else(|| number_field(feature, "pocket_diameter_mm"))
        .or_else(|| number_field(feature, "bore_diameter_mm"));
    if !fastener_diameter.is_some_and(|value| value > 0.0) {
        return json!({
            "rule_id": full_rule_id,
            "status": "fail",
            "reason": "missing_fastener_support_inner_diameter",
            "feature_id": feature_id,
            "message": "Feature diameter, pocket_diameter_mm, or bore_diameter_mm is required for fastener support linting."
        });
    }
    let fastener_diameter = fastener_diameter.unwrap();

    let support_diameter = number_field(feature, "support_diameter_mm")
        .or_else(|| number_field(feature, "boss_diameter_mm"))
        .or_else(|| number_field(feature, "outer_diameter_mm"));
    if !support_diameter.is_some_and(|value| value > 0.0) {
        return json!({
            "rule_id": full_rule_id,
            "status": "fail",
            "reason": "missing_fastener_support_diameter",
            "feature_id": feature_id,
            "measured": {
                "inner_diameter_mm": round(fastener_diameter),
                "support_diameter_mm": Value::Null
            },
            "message": "Declared fastener-support features must include support_diameter_mm, boss_diameter_mm, or outer_diameter_mm."
        });
    }
    let support_diameter = support_diameter.unwrap();

    let required_wall_thickness = number_field(rule, "min_wall_thickness_mm");
    if !required_wall_thickness.is_some_and(|value| value > 0.0) {
        return json!({
            "rule_id": full_rule_id,
            "status": "fail",
            "reason": "invalid_rule_min_wall_thickness",
            "feature_id": feature_id,
            "message": "Rule min_wall_thickness_mm must be a positive number."
        });
    }
    let required_wall_thickness = required_wall_thickness.unwrap();

    let support_wall_thickness = (support_diameter - fastener_diameter) / 2.0;
    let margin = support_wall_thickness - required_wall_thickness;
    let pass = margin >= 0.0;

    json!({
        "rule_id": full_rule_id,
        "status": if pass { "pass" } else { "fail" },
        "reason": if pass { "ok" } else { "insufficient_fastener_support_wall" },
        "feature_id": feature_id,
        "measured": {
            "inner_diameter_mm": round(fastener_diameter),
            "support_diameter_mm": round(support_diameter),
            "support_wall_thickness_mm": round(support_wall_thickness)
        },
        "required": {
            "wall_thickness_mm": round(required_wall_thickness)
        },
        "margin_mm": round(margin),
        "message": if pass {
            "Fastener support wall thickness passes rule.".to_string()
        } else {
            format!("Fastener support wall thickness is short by {} mm.", trim_float(round(margin.abs())))
        }
    })
}

fn check_blind_pocket_back_wall_thickness(
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
    let feature_id = feature.get("id").cloned().unwrap_or(Value::Null);

    let required_back_wall = number_field(rule, "min_back_wall_thickness_mm");
    if !required_back_wall.is_some_and(|value| value > 0.0) {
        return json!({
            "rule_id": full_rule_id,
            "status": "fail",
            "reason": "invalid_rule_min_back_wall_thickness",
            "feature_id": feature_id,
            "message": "Rule min_back_wall_thickness_mm must be a positive number."
        });
    }
    let required_back_wall = required_back_wall.unwrap();

    let part = string_field(feature, "part").and_then(|part_id| find_part(manifest, part_id));
    let bbox_min = part
        .and_then(|part| part.pointer("/bbox_mm/min"))
        .and_then(number_array)
        .and_then(Vec3::from_values);
    let bbox_max = part
        .and_then(|part| part.pointer("/bbox_mm/max"))
        .and_then(number_array)
        .and_then(Vec3::from_values);
    let axis = feature
        .get("axis")
        .and_then(number_array)
        .and_then(Vec3::from_values)
        .and_then(Vec3::normalized);
    let pocket_center = feature
        .get("pocket_center_mm")
        .and_then(number_array)
        .and_then(Vec3::from_values);
    let bottom_center = feature
        .get("bottom_center_mm")
        .and_then(number_array)
        .and_then(Vec3::from_values);

    let (Some(bbox_min), Some(bbox_max), Some(axis), Some(pocket_center), Some(bottom_center)) =
        (bbox_min, bbox_max, axis, pocket_center, bottom_center)
    else {
        return json!({
            "rule_id": full_rule_id,
            "status": "fail",
            "reason": "missing_blind_pocket_back_wall_geometry",
            "feature_id": feature_id,
            "measured": {
                "part": feature.get("part").cloned().unwrap_or(Value::Null),
                "has_bbox": part.is_some(),
                "has_axis": feature.get("axis").is_some(),
                "has_pocket_center_mm": feature.get("pocket_center_mm").is_some(),
                "has_bottom_center_mm": feature.get("bottom_center_mm").is_some()
            },
            "required": {
                "min_back_wall_thickness_mm": round(required_back_wall)
            },
            "message": "Blind-pocket back-wall checks require feature.part, parts[].bbox_mm, axis, pocket_center_mm, and bottom_center_mm."
        });
    };

    let Some(bottom_direction) = bottom_center.sub(pocket_center).normalized() else {
        return json!({
            "rule_id": full_rule_id,
            "status": "fail",
            "reason": "missing_blind_pocket_back_wall_geometry",
            "feature_id": feature_id,
            "measured": {
                "pocket_center_mm": pocket_center.to_json(),
                "bottom_center_mm": bottom_center.to_json()
            },
            "required": {
                "min_back_wall_thickness_mm": round(required_back_wall)
            },
            "message": "Blind pocket bottom_center_mm must be offset from pocket_center_mm."
        });
    };

    let axis_dot = axis.dot(bottom_direction).abs();
    let components = [
        (
            "x",
            bottom_direction.x,
            bottom_center.x,
            bbox_min.x,
            bbox_max.x,
        ),
        (
            "y",
            bottom_direction.y,
            bottom_center.y,
            bbox_min.y,
            bbox_max.y,
        ),
        (
            "z",
            bottom_direction.z,
            bottom_center.z,
            bbox_min.z,
            bbox_max.z,
        ),
    ];
    let Some((axis_name, direction, bottom_position, bbox_low, bbox_high)) = components
        .into_iter()
        .max_by(|left, right| left.1.abs().total_cmp(&right.1.abs()))
    else {
        unreachable!();
    };
    if direction.abs() < 0.99 || axis_dot < 0.99 {
        return json!({
            "rule_id": full_rule_id,
            "status": "fail",
            "reason": "unsupported_blind_pocket_axis",
            "feature_id": feature_id,
            "measured": {
                "axis": axis.to_json(),
                "bottom_direction": bottom_direction.to_json(),
                "axis_dot": round(axis_dot)
            },
            "required": {
                "axis_dot_min": 0.99,
                "bottom_direction_axis_aligned_min": 0.99
            },
            "message": "Blind-pocket back-wall checks currently require an axis-aligned pocket bottom direction."
        });
    }

    let (side, back_wall_thickness) = if direction >= 0.0 {
        ("max", bbox_high - bottom_position)
    } else {
        ("min", bottom_position - bbox_low)
    };
    let margin = back_wall_thickness - required_back_wall;
    let pass = margin >= 0.0;

    json!({
        "rule_id": full_rule_id,
        "status": if pass { "pass" } else { "fail" },
        "reason": if pass { "ok" } else { "insufficient_blind_pocket_back_wall" },
        "feature_id": feature_id,
        "measured": {
            "pocket_center_mm": pocket_center.to_json(),
            "bottom_center_mm": bottom_center.to_json(),
            "bottom_direction": bottom_direction.to_json(),
            "back_wall_thickness_mm": round(back_wall_thickness),
            "back_face": {
                "axis": axis_name,
                "side": side
            },
            "source": "parts[feature.part].bbox_mm minus bottom_center_mm"
        },
        "required": {
            "min_back_wall_thickness_mm": round(required_back_wall)
        },
        "margin_mm": round(margin),
        "message": if pass {
            "Blind pocket back-wall thickness passes rule.".to_string()
        } else {
            format!("Blind pocket back-wall thickness is short by {} mm.", trim_float(round(margin.abs())))
        }
    })
}

fn check_standoff_boss_support_link(
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
    let feature_id = feature.get("id").cloned().unwrap_or(Value::Null);
    let feature_id_string = string_field(feature, "id").unwrap_or("<missing>");

    let Some(supported_feature_id) = string_field(feature, "supports_feature_id") else {
        return json!({
            "rule_id": full_rule_id,
            "status": "fail",
            "reason": "missing_standoff_support_link",
            "feature_id": feature_id,
            "message": "Declared standoff bosses must name the feature they support with supports_feature_id."
        });
    };

    let Some(supported_feature) = find_feature(manifest, supported_feature_id) else {
        return json!({
            "rule_id": full_rule_id,
            "status": "fail",
            "reason": "missing_supported_feature",
            "feature_id": feature_id,
            "related_feature_id": supported_feature_id,
            "measured": {
                "supported_feature_id": supported_feature_id
            },
            "message": "Standoff boss supports_feature_id does not match any declared feature."
        });
    };

    let supported_kind = string_field(supported_feature, "kind").unwrap_or("<missing>");
    if !matches!(supported_kind, "clearance_hole" | "heat_set_insert_pocket") {
        return json!({
            "rule_id": full_rule_id,
            "status": "fail",
            "reason": "unsupported_standoff_support_kind",
            "feature_id": feature_id,
            "related_feature_id": supported_feature_id,
            "measured": {
                "supported_feature_id": supported_feature_id,
                "supported_feature_kind": supported_kind
            },
            "required": {
                "supported_feature_kind_any": ["clearance_hole", "heat_set_insert_pocket"]
            },
            "message": "Standoff boss support-link checks currently compare clearance holes and heat-set insert pockets."
        });
    }

    let Some(boss_center) = feature
        .get("boss_center_mm")
        .and_then(number_array)
        .and_then(Vec3::from_values)
        .or_else(|| {
            feature
                .get("center_mm")
                .and_then(number_array)
                .and_then(Vec3::from_values)
        })
    else {
        return json!({
            "rule_id": full_rule_id,
            "status": "fail",
            "reason": "missing_feature_center",
            "feature_id": feature_id,
            "related_feature_id": supported_feature_id,
            "message": "Standoff boss boss_center_mm or center_mm is required for support-link checking."
        });
    };
    let Some(boss_axis) = feature
        .get("axis")
        .and_then(number_array)
        .and_then(Vec3::from_values)
        .and_then(Vec3::normalized)
    else {
        return json!({
            "rule_id": full_rule_id,
            "status": "fail",
            "reason": "missing_feature_axis",
            "feature_id": feature_id,
            "related_feature_id": supported_feature_id,
            "message": "Standoff boss axis is required for support-link checking."
        });
    };
    let Some(supported_center) = supported_feature
        .get("center_mm")
        .and_then(number_array)
        .and_then(Vec3::from_values)
    else {
        return json!({
            "rule_id": full_rule_id,
            "status": "fail",
            "reason": "missing_feature_center",
            "feature_id": feature_id,
            "related_feature_id": supported_feature_id,
            "message": "Supported feature center_mm is required for support-link checking."
        });
    };
    let Some(supported_axis) = supported_feature
        .get("axis")
        .and_then(number_array)
        .and_then(Vec3::from_values)
        .and_then(Vec3::normalized)
    else {
        return json!({
            "rule_id": full_rule_id,
            "status": "fail",
            "reason": "missing_feature_axis",
            "feature_id": feature_id,
            "related_feature_id": supported_feature_id,
            "message": "Supported feature axis is required for support-link checking."
        });
    };

    let Some(boss_diameter) = number_field(feature, "boss_diameter_mm")
        .or_else(|| number_field(feature, "support_diameter_mm"))
        .or_else(|| number_field(feature, "outer_diameter_mm"))
        .filter(|value| *value > 0.0)
    else {
        return json!({
            "rule_id": full_rule_id,
            "status": "fail",
            "reason": "missing_standoff_boss_diameter",
            "feature_id": feature_id,
            "related_feature_id": supported_feature_id,
            "message": "Standoff boss boss_diameter_mm is required for support-link checking."
        });
    };
    let Some(supported_diameter) = number_field(supported_feature, "support_diameter_mm")
        .or_else(|| number_field(supported_feature, "boss_diameter_mm"))
        .or_else(|| number_field(supported_feature, "outer_diameter_mm"))
        .or_else(|| number_field(supported_feature, "diameter_mm"))
        .or_else(|| number_field(supported_feature, "pocket_diameter_mm"))
        .filter(|value| *value > 0.0)
    else {
        return json!({
            "rule_id": full_rule_id,
            "status": "fail",
            "reason": "missing_supported_feature_diameter",
            "feature_id": feature_id,
            "related_feature_id": supported_feature_id,
            "measured": {
                "supported_feature_id": supported_feature_id,
                "supported_feature_kind": supported_kind
            },
            "message": "Supported feature needs support_diameter_mm, diameter_mm, or pocket_diameter_mm for support-link checking."
        });
    };

    let centerline_tolerance = number_field(rule, "centerline_tolerance_mm")
        .unwrap_or(0.25)
        .max(0.0);
    let diameter_tolerance = number_field(rule, "support_diameter_tolerance_mm")
        .or_else(|| number_field(rule, "diameter_tolerance_mm"))
        .unwrap_or(0.05)
        .max(0.0);
    let axis_dot_min = number_field(rule, "axis_dot_min")
        .unwrap_or(0.99)
        .clamp(0.0, 1.0);

    let centerline_distance = boss_center.sub(supported_center).length();
    let axis_dot = boss_axis.dot(supported_axis).abs();
    let support_diameter_delta = (boss_diameter - supported_diameter).abs();
    let pass = centerline_distance <= centerline_tolerance
        && axis_dot >= axis_dot_min
        && support_diameter_delta <= diameter_tolerance;
    let margin = centerline_tolerance - centerline_distance;

    json!({
        "rule_id": full_rule_id,
        "status": if pass { "pass" } else { "fail" },
        "reason": if pass { "ok" } else { "standoff_boss_support_mismatch" },
        "feature_id": feature_id,
        "related_feature_id": supported_feature_id,
        "measured": {
            "supported_feature_id": supported_feature_id,
            "supported_feature_kind": supported_kind,
            "centerline_distance_mm": round(centerline_distance),
            "axis_dot": round(axis_dot),
            "boss_diameter_mm": round(boss_diameter),
            "supported_support_diameter_mm": round(supported_diameter),
            "support_diameter_delta_mm": round(support_diameter_delta)
        },
        "required": {
            "centerline_tolerance_mm": round(centerline_tolerance),
            "axis_dot_min": round(axis_dot_min),
            "support_diameter_tolerance_mm": round(diameter_tolerance)
        },
        "margin_mm": round(margin),
        "repair": {
            "action": "align_standoff_boss_to_supported_feature",
            "target_feature_id": feature_id_string,
            "related_feature_id": supported_feature_id,
            "value_paths": [
                format!("features[id={feature_id_string}].center_mm"),
                format!("features[id={feature_id_string}].boss_center_mm"),
                format!("features[id={feature_id_string}].top_center_mm")
            ]
        },
        "message": if pass {
            "Standoff boss is aligned with the feature it supports.".to_string()
        } else {
            format!(
                "Standoff boss is offset from supported feature by {} mm.",
                trim_float(round(centerline_distance))
            )
        }
    })
}

fn check_feature_count(manifest: &Value, rulepack: &Value, rule: &Value) -> Value {
    let full_rule_id = format!(
        "{}:{}",
        string_field(rulepack, "id").unwrap_or("<missing>"),
        string_field(rule, "id").unwrap_or("<missing>")
    );
    let features: Vec<&Value> = manifest
        .get("features")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|feature| feature_applies(feature, rule.get("applies_to")))
        .collect();
    let count = features.len() as u64;
    let min_count = rule.get("min_count").and_then(Value::as_u64);
    let max_count = rule.get("max_count").and_then(Value::as_u64);
    if min_count.is_none() && max_count.is_none() {
        return json!({
            "rule_id": full_rule_id,
            "status": "fail",
            "reason": "invalid_feature_count_rule_bounds",
            "message": "feature_count rules must declare min_count, max_count, or both."
        });
    }
    let min_pass = min_count.map_or(true, |value| count >= value);
    let max_pass = max_count.map_or(true, |value| count <= value);
    let pass = min_pass && max_pass;
    let feature_ids: Vec<Value> = features
        .iter()
        .filter_map(|feature| string_field(feature, "id").map(|id| Value::String(id.to_string())))
        .collect();

    json!({
        "rule_id": full_rule_id,
        "status": if pass { "pass" } else { "fail" },
        "reason": if pass { "ok" } else { "feature_count_out_of_range" },
        "feature_ids": feature_ids,
        "measured": { "count": count },
        "required": {
            "min_count": min_count,
            "max_count": max_count
        },
        "message": if pass {
            "Feature count passes rule.".to_string()
        } else {
            format!("Feature count {count} is outside declared range.")
        }
    })
}

fn check_numeric_range(manifest: &Value, rulepack: &Value, rule: &Value) -> Value {
    let full_rule_id = format!(
        "{}:{}",
        string_field(rulepack, "id").unwrap_or("<missing>"),
        string_field(rule, "id").unwrap_or("<missing>")
    );
    let path = string_field(rule, "path").unwrap_or("");
    let value = value_at_path(manifest, path).and_then(Value::as_f64);
    let value = value.filter(|value| value.is_finite());
    let min = number_field(rule, "min");
    let max = number_field(rule, "max");
    if min.is_none() && max.is_none() {
        return json!({
            "rule_id": full_rule_id,
            "status": "fail",
            "reason": "invalid_numeric_range_rule_bounds",
            "path": path,
            "message": "numeric_range rules must declare min, max, or both."
        });
    }

    let Some(value) = value else {
        return json!({
            "rule_id": full_rule_id,
            "status": "fail",
            "reason": "missing_numeric_value",
            "path": path,
            "measured": { "value": Value::Null },
            "required": {
                "min": min,
                "max": max
            },
            "message": "Numeric design value cannot be derived."
        });
    };

    let min_pass = min.map_or(true, |minimum| value >= minimum);
    let max_pass = max.map_or(true, |maximum| value <= maximum);
    let pass = min_pass && max_pass;

    json!({
        "rule_id": full_rule_id,
        "status": if pass { "pass" } else { "fail" },
        "reason": if pass { "ok" } else { "numeric_value_out_of_range" },
        "path": path,
        "measured": { "value": round(value) },
        "required": {
            "min": min,
            "max": max
        },
        "message": if pass {
            "Numeric design value passes rule.".to_string()
        } else {
            format!("Numeric design value {} is outside declared range.", trim_float(round(value)))
        }
    })
}

#[derive(Debug, Clone)]
struct EdgeDistanceCandidate {
    axis: &'static str,
    side: &'static str,
    wall_to_edge_mm: f64,
}

fn check_feature_edge_distance(
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
    let min_wall_to_edge = number_field(rule, "min_wall_to_edge_mm");
    if !min_wall_to_edge.is_some_and(|value| value >= 0.0) {
        return json!({
            "rule_id": full_rule_id,
            "status": "fail",
            "reason": "invalid_feature_edge_rule_clearance",
            "feature_id": feature.get("id").cloned().unwrap_or(Value::Null),
            "message": "feature_edge_distance rules must declare min_wall_to_edge_mm as a non-negative number."
        });
    }
    let min_wall_to_edge = min_wall_to_edge.unwrap();

    let center_field = string_field(rule, "center_field").unwrap_or("center_mm");
    let diameter_field = string_field(rule, "diameter_field").unwrap_or("diameter_mm");
    let width_field = string_field(rule, "width_field").unwrap_or("width_mm");
    let length_field = string_field(rule, "length_field").unwrap_or("length_mm");
    let span_axis_field = string_field(rule, "span_axis_field").unwrap_or("span_axis");
    let Some(shape) = pair_spacing_shape(
        feature,
        center_field,
        diameter_field,
        width_field,
        length_field,
        span_axis_field,
    ) else {
        return json!({
            "rule_id": full_rule_id,
            "status": "fail",
            "reason": "missing_feature_edge_geometry",
            "feature_id": feature.get("id").cloned().unwrap_or(Value::Null),
            "measured": {
                "center_field": center_field,
                "diameter_field": diameter_field,
                "width_field": width_field,
                "length_field": length_field,
                "span_axis_field": span_axis_field,
                "center_present": feature.get(center_field).is_some(),
                "diameter_present": feature.get(diameter_field).is_some(),
                "width_present": feature.get(width_field).is_some(),
                "length_present": feature.get(length_field).is_some(),
                "span_axis_present": feature.get(span_axis_field).is_some(),
                "spacing_envelope_present": feature.get("spacing_envelope").is_some()
            },
            "required": {
                "circle": "center_mm and diameter_mm > 0",
                "slot_capsule": "straight_slot with center_mm, width_mm > 0, length_mm >= width_mm, and span_axis",
                "spacing_envelope": "optional circle or capsule spacing_envelope"
            },
            "message": "Feature edge-distance checks require circle or slot/capsule geometry metadata."
        });
    };

    let Some(through_axis) = feature
        .get("axis")
        .and_then(number_array)
        .and_then(axis_index_from_vector)
    else {
        return json!({
            "rule_id": full_rule_id,
            "status": "fail",
            "reason": "missing_feature_axis",
            "feature_id": feature.get("id").cloned().unwrap_or(Value::Null),
            "message": "Feature edge-distance checks require an axis-aligned axis."
        });
    };

    let Some(part) = string_field(feature, "part").and_then(|part_id| find_part(manifest, part_id))
    else {
        return json!({
            "rule_id": full_rule_id,
            "status": "fail",
            "reason": "missing_feature_part_bbox",
            "feature_id": feature.get("id").cloned().unwrap_or(Value::Null),
            "message": "Feature edge-distance checks require feature.part to reference a part with bbox_mm."
        });
    };
    let bbox_min = part.pointer("/bbox_mm/min").and_then(number_array);
    let bbox_max = part.pointer("/bbox_mm/max").and_then(number_array);
    let Some(closest) =
        bbox_min
            .as_deref()
            .zip(bbox_max.as_deref())
            .and_then(|(bbox_min, bbox_max)| {
                feature_edge_distance_to_bbox(&shape, bbox_min, bbox_max, through_axis)
            })
    else {
        return json!({
            "rule_id": full_rule_id,
            "status": "fail",
            "reason": "missing_feature_part_bbox",
            "feature_id": feature.get("id").cloned().unwrap_or(Value::Null),
            "message": "Feature edge-distance checks require parts[feature.part].bbox_mm min/max vectors."
        });
    };

    let margin = closest.wall_to_edge_mm - min_wall_to_edge;
    let pass = margin >= 0.0;

    json!({
        "rule_id": full_rule_id,
        "status": if pass { "pass" } else { "fail" },
        "reason": if pass { "ok" } else { "insufficient_feature_edge_distance" },
        "feature_id": feature.get("id").cloned().unwrap_or(Value::Null),
        "measured": {
            "feature_shape": shape.kind,
            "wall_to_edge_mm": round(closest.wall_to_edge_mm),
            "closest_edge": {
                "axis": closest.axis,
                "side": closest.side
            },
            "source": "parts[feature.part].bbox_mm minus feature envelope"
        },
        "required": {
            "min_wall_to_edge_mm": round(min_wall_to_edge),
            "center_field": center_field,
            "diameter_field": diameter_field,
            "width_field": width_field,
            "length_field": length_field,
            "span_axis_field": span_axis_field
        },
        "margin_mm": round(margin),
        "message": if pass {
            "Feature edge distance passes rule.".to_string()
        } else {
            format!("Feature edge distance is short by {} mm.", trim_float(round(margin.abs())))
        }
    })
}

fn feature_edge_distance_to_bbox(
    shape: &PairSpacingShape,
    bbox_min: &[f64],
    bbox_max: &[f64],
    through_axis: usize,
) -> Option<EdgeDistanceCandidate> {
    if bbox_min.len() != 3 || bbox_max.len() != 3 || through_axis > 2 {
        return None;
    }

    let mut closest: Option<EdgeDistanceCandidate> = None;
    for axis in 0..3 {
        if axis == through_axis {
            continue;
        }
        let segment_min = shape
            .segment_start
            .coordinate(axis)
            .min(shape.segment_end.coordinate(axis));
        let segment_max = shape
            .segment_start
            .coordinate(axis)
            .max(shape.segment_end.coordinate(axis));
        let envelope_min = segment_min - shape.radius_mm;
        let envelope_max = segment_max + shape.radius_mm;
        let candidates = [
            EdgeDistanceCandidate {
                axis: axis_label(axis),
                side: "min",
                wall_to_edge_mm: envelope_min - bbox_min[axis],
            },
            EdgeDistanceCandidate {
                axis: axis_label(axis),
                side: "max",
                wall_to_edge_mm: bbox_max[axis] - envelope_max,
            },
        ];

        for candidate in candidates {
            if !candidate.wall_to_edge_mm.is_finite() {
                continue;
            }
            if closest
                .as_ref()
                .is_none_or(|closest| candidate.wall_to_edge_mm < closest.wall_to_edge_mm)
            {
                closest = Some(candidate);
            }
        }
    }
    closest
}

fn axis_label(axis: usize) -> &'static str {
    match axis {
        0 => "x",
        1 => "y",
        2 => "z",
        _ => "<invalid>",
    }
}

#[derive(Debug, Clone)]
struct PairSpacingFeature {
    id: String,
    shape: PairSpacingShape,
}

#[derive(Debug, Clone)]
struct PairSpacingShape {
    kind: &'static str,
    center: Vec3,
    segment_start: Vec3,
    segment_end: Vec3,
    radius_mm: f64,
}

#[derive(Debug, Clone)]
struct PairSpacingCandidate {
    a_id: String,
    b_id: String,
    a_shape: &'static str,
    b_shape: &'static str,
    center_distance_mm: f64,
    shape_distance_mm: f64,
    clearance_mm: f64,
    margin_mm: f64,
}

fn check_feature_pair_spacing(manifest: &Value, rulepack: &Value, rule: &Value) -> Value {
    let full_rule_id = format!(
        "{}:{}",
        string_field(rulepack, "id").unwrap_or("<missing>"),
        string_field(rule, "id").unwrap_or("<missing>")
    );
    let min_clearance = number_field(rule, "min_clearance_mm");
    if !min_clearance.is_some_and(|value| value >= 0.0) {
        return json!({
            "rule_id": full_rule_id,
            "status": "fail",
            "reason": "invalid_pair_spacing_rule_clearance",
            "message": "feature_pair_spacing rules must declare min_clearance_mm as a non-negative number."
        });
    }
    let min_clearance = min_clearance.unwrap();

    let center_field = string_field(rule, "center_field").unwrap_or("center_mm");
    let diameter_field = string_field(rule, "diameter_field").unwrap_or("diameter_mm");
    let width_field = string_field(rule, "width_field").unwrap_or("width_mm");
    let length_field = string_field(rule, "length_field").unwrap_or("length_mm");
    let span_axis_field = string_field(rule, "span_axis_field").unwrap_or("span_axis");
    let features: Vec<&Value> = manifest
        .get("features")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|feature| feature_applies(feature, rule.get("applies_to")))
        .collect();

    let mut checked_features = Vec::new();
    for feature in features {
        let feature_id = string_field(feature, "id")
            .unwrap_or("<missing>")
            .to_string();
        let Some(shape) = pair_spacing_shape(
            feature,
            center_field,
            diameter_field,
            width_field,
            length_field,
            span_axis_field,
        ) else {
            return json!({
                "rule_id": full_rule_id,
                "status": "fail",
                "reason": "missing_pair_spacing_geometry",
                "feature_id": feature.get("id").cloned().unwrap_or(Value::Null),
                "measured": {
                    "center_field": center_field,
                    "diameter_field": diameter_field,
                    "width_field": width_field,
                    "length_field": length_field,
                    "span_axis_field": span_axis_field,
                    "center_present": feature.get(center_field).is_some(),
                    "diameter_present": feature.get(diameter_field).is_some(),
                    "width_present": feature.get(width_field).is_some(),
                    "length_present": feature.get(length_field).is_some(),
                    "span_axis_present": feature.get(span_axis_field).is_some(),
                    "spacing_envelope_present": feature.get("spacing_envelope").is_some()
                },
                "required": {
                    "center_field": center_field,
                    "diameter_field": diameter_field,
                    "circle": "center_mm and diameter_mm > 0",
                    "slot_capsule": "straight_slot with center_mm, width_mm > 0, length_mm >= width_mm, and span_axis",
                    "spacing_envelope": "optional circle or capsule spacing_envelope"
                },
                "message": "Pair-spacing checks require circle or slot/capsule geometry metadata for every selected feature."
            });
        };
        checked_features.push(PairSpacingFeature {
            id: feature_id,
            shape,
        });
    }

    if checked_features.len() < 2 {
        let feature_ids: Vec<Value> = checked_features
            .iter()
            .map(|feature| Value::String(feature.id.clone()))
            .collect();
        return json!({
            "rule_id": full_rule_id,
            "status": "pass",
            "reason": "ok",
            "feature_ids": feature_ids,
            "measured": {
                "pair_count": 0,
                "closest_pair": Value::Null
            },
            "required": {
                "min_clearance_mm": round(min_clearance)
            },
            "message": "Fewer than two matching features; no pair spacing to check."
        });
    }

    let mut closest: Option<PairSpacingCandidate> = None;
    let mut violations = Vec::new();
    for left_index in 0..checked_features.len() {
        for right_index in (left_index + 1)..checked_features.len() {
            let left = &checked_features[left_index];
            let right = &checked_features[right_index];
            let center_distance = left.shape.center.sub(right.shape.center).length();
            let shape_distance = segment_segment_distance(
                left.shape.segment_start,
                left.shape.segment_end,
                right.shape.segment_start,
                right.shape.segment_end,
            );
            let clearance = shape_distance - left.shape.radius_mm - right.shape.radius_mm;
            let margin = clearance - min_clearance;
            let candidate = PairSpacingCandidate {
                a_id: left.id.clone(),
                b_id: right.id.clone(),
                a_shape: left.shape.kind,
                b_shape: right.shape.kind,
                center_distance_mm: center_distance,
                shape_distance_mm: shape_distance,
                clearance_mm: clearance,
                margin_mm: margin,
            };
            if closest
                .as_ref()
                .is_none_or(|closest| candidate.clearance_mm < closest.clearance_mm)
            {
                closest = Some(candidate.clone());
            }
            if margin < 0.0 {
                violations.push(json!({
                    "feature_ids": [left.id.clone(), right.id.clone()],
                    "feature_shapes": [left.shape.kind, right.shape.kind],
                    "center_distance_mm": round(center_distance),
                    "shape_distance_mm": round(shape_distance),
                    "clearance_mm": round(clearance),
                    "margin_mm": round(margin)
                }));
            }
        }
    }

    let closest_pair = closest
        .as_ref()
        .map(|pair| {
            json!({
                "feature_ids": [pair.a_id.clone(), pair.b_id.clone()],
                "feature_shapes": [pair.a_shape, pair.b_shape],
                "center_distance_mm": round(pair.center_distance_mm),
                "shape_distance_mm": round(pair.shape_distance_mm),
                "clearance_mm": round(pair.clearance_mm),
                "margin_mm": round(pair.margin_mm)
            })
        })
        .unwrap_or(Value::Null);
    let pass = violations.is_empty();
    let feature_ids: Vec<Value> = checked_features
        .iter()
        .map(|feature| Value::String(feature.id.clone()))
        .collect();
    let margin = closest
        .as_ref()
        .map(|pair| pair.margin_mm)
        .unwrap_or(min_clearance);

    json!({
        "rule_id": full_rule_id,
        "status": if pass { "pass" } else { "fail" },
        "reason": if pass { "ok" } else { "insufficient_feature_pair_spacing" },
        "feature_ids": feature_ids,
        "measured": {
            "pair_count": checked_features.len() * (checked_features.len() - 1) / 2,
            "closest_pair": closest_pair,
            "violating_pairs": violations
        },
        "required": {
            "min_clearance_mm": round(min_clearance),
            "center_field": center_field,
            "diameter_field": diameter_field,
            "width_field": width_field,
            "length_field": length_field,
            "span_axis_field": span_axis_field
        },
        "margin_mm": round(margin),
        "message": if pass {
            "Feature pair spacing passes rule.".to_string()
        } else {
            format!("Feature pair spacing is short by {} mm.", trim_float(round(margin.abs())))
        }
    })
}

fn pair_spacing_shape(
    feature: &Value,
    center_field: &str,
    diameter_field: &str,
    width_field: &str,
    length_field: &str,
    span_axis_field: &str,
) -> Option<PairSpacingShape> {
    if let Some(envelope) = feature.get("spacing_envelope") {
        return pair_spacing_envelope_shape(envelope);
    }

    if string_field(feature, "kind") == Some("straight_slot") {
        return pair_spacing_slot_shape(
            feature,
            center_field,
            width_field,
            length_field,
            span_axis_field,
        );
    }

    pair_spacing_circle_shape(feature, center_field, diameter_field)
}

fn pair_spacing_circle_shape(
    feature: &Value,
    center_field: &str,
    diameter_field: &str,
) -> Option<PairSpacingShape> {
    let center = feature
        .get(center_field)
        .and_then(number_array)
        .and_then(Vec3::from_values)?;
    let diameter = number_field(feature, diameter_field).filter(|value| *value > 0.0)?;

    Some(PairSpacingShape {
        kind: "circle",
        center,
        segment_start: center,
        segment_end: center,
        radius_mm: diameter / 2.0,
    })
}

fn pair_spacing_slot_shape(
    feature: &Value,
    center_field: &str,
    width_field: &str,
    length_field: &str,
    span_axis_field: &str,
) -> Option<PairSpacingShape> {
    let center = feature
        .get(center_field)
        .and_then(number_array)
        .and_then(Vec3::from_values)?;
    let width = number_field(feature, width_field).filter(|value| *value > 0.0)?;
    let length = number_field(feature, length_field).filter(|value| *value >= width)?;
    let span_axis = feature
        .get(span_axis_field)
        .and_then(number_array)
        .and_then(Vec3::from_values)
        .and_then(Vec3::normalized)?;
    let half_segment = ((length - width) / 2.0).max(0.0);
    Some(PairSpacingShape {
        kind: "capsule",
        center,
        segment_start: center.add(span_axis.scale(-half_segment)),
        segment_end: center.add(span_axis.scale(half_segment)),
        radius_mm: width / 2.0,
    })
}

fn pair_spacing_envelope_shape(envelope: &Value) -> Option<PairSpacingShape> {
    match string_field(envelope, "kind") {
        Some("circle") => {
            let center = envelope
                .get("center_mm")
                .and_then(number_array)
                .and_then(Vec3::from_values)?;
            let radius = number_field(envelope, "radius_mm").filter(|value| *value > 0.0)?;
            Some(PairSpacingShape {
                kind: "circle",
                center,
                segment_start: center,
                segment_end: center,
                radius_mm: radius,
            })
        }
        Some("capsule") => {
            let segment_start = envelope
                .get("segment_start_mm")
                .and_then(number_array)
                .and_then(Vec3::from_values)?;
            let segment_end = envelope
                .get("segment_end_mm")
                .and_then(number_array)
                .and_then(Vec3::from_values)?;
            let radius = number_field(envelope, "radius_mm").filter(|value| *value > 0.0)?;
            Some(PairSpacingShape {
                kind: "capsule",
                center: segment_start.add(segment_end).scale(0.5),
                segment_start,
                segment_end,
                radius_mm: radius,
            })
        }
        _ => None,
    }
}

fn segment_segment_distance(a0: Vec3, a1: Vec3, b0: Vec3, b1: Vec3) -> f64 {
    // Closest distance between finite 3D segments. Handles point segments too.
    const EPSILON: f64 = 1e-9;
    let u = a1.sub(a0);
    let v = b1.sub(b0);
    let w = a0.sub(b0);
    let a = u.dot(u);
    let b = u.dot(v);
    let c = v.dot(v);
    let d = u.dot(w);
    let e = v.dot(w);
    let denominator = a * c - b * b;

    if a <= EPSILON && c <= EPSILON {
        return a0.sub(b0).length();
    }
    if a <= EPSILON {
        let t = (e / c).clamp(0.0, 1.0);
        return a0.sub(b0.add(v.scale(t))).length();
    }
    if c <= EPSILON {
        let s = (-d / a).clamp(0.0, 1.0);
        return a0.add(u.scale(s)).sub(b0).length();
    }

    let mut s = if denominator.abs() > EPSILON {
        ((b * e - c * d) / denominator).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let mut t = (b * s + e) / c;

    if t < 0.0 {
        t = 0.0;
        s = (-d / a).clamp(0.0, 1.0);
    } else if t > 1.0 {
        t = 1.0;
        s = ((b - d) / a).clamp(0.0, 1.0);
    }

    let closest_a = a0.add(u.scale(s));
    let closest_b = b0.add(v.scale(t));
    closest_a.sub(closest_b).length()
}

fn check_feature_presence(
    manifest: &Value,
    manifest_dir: &Path,
    rulepack: &Value,
    rule: &Value,
    feature: &Value,
) -> Value {
    let full_rule_id = format!(
        "{}:{}",
        string_field(rulepack, "id").unwrap_or("<missing>"),
        string_field(rule, "id").unwrap_or("<missing>")
    );
    let feature_id = feature.get("id").cloned().unwrap_or(Value::Null);
    let artifact_kind = string_field(rule, "artifact_kind").unwrap_or("step");
    let Some(artifact) = find_artifact(manifest, artifact_kind) else {
        return json!({
            "rule_id": full_rule_id,
            "status": "fail",
            "reason": "missing_step_artifact_ref",
            "feature_id": feature_id,
            "message": "Design data must list a STEP artifact for feature-presence checking."
        });
    };
    let resolved_artifact = match resolve_file_ref(manifest_dir, artifact) {
        Ok(value) => value,
        Err(reason) => {
            return json!({
                "rule_id": full_rule_id,
                "status": "fail",
                "reason": reason,
                "feature_id": feature_id,
                "path": artifact.get("path").and_then(Value::as_str),
                "message": "STEP artifact path is invalid."
            });
        }
    };

    if string_field(feature, "kind") == Some("counterbore") {
        return check_counterbore_presence(
            full_rule_id,
            feature_id,
            resolved_artifact,
            rule,
            feature,
        );
    }

    if string_field(feature, "kind") == Some("heat_set_insert_pocket") {
        return check_heat_set_insert_pocket_presence(
            full_rule_id,
            feature_id,
            resolved_artifact,
            rule,
            feature,
        );
    }

    if string_field(feature, "kind") == Some("bearing_seat") {
        return check_bearing_seat_presence(
            full_rule_id,
            feature_id,
            resolved_artifact,
            rule,
            feature,
        );
    }

    if string_field(feature, "kind") == Some("standoff_boss") {
        return check_standoff_boss_presence(
            full_rule_id,
            feature_id,
            resolved_artifact,
            rule,
            feature,
        );
    }

    if string_field(feature, "kind") == Some("straight_slot") {
        return check_straight_slot_presence(
            full_rule_id,
            feature_id,
            resolved_artifact,
            rule,
            feature,
        );
    }

    let Some(diameter) = number_field(feature, "diameter_mm").filter(|value| *value > 0.0) else {
        return json!({
            "rule_id": full_rule_id,
            "status": "fail",
            "reason": "missing_hole_diameter",
            "feature_id": feature_id,
            "message": "Hole diameter is required for STEP feature-presence checking."
        });
    };
    let Some(center) = feature
        .get("center_mm")
        .and_then(number_array)
        .and_then(Vec3::from_values)
    else {
        return json!({
            "rule_id": full_rule_id,
            "status": "fail",
            "reason": "missing_feature_center",
            "feature_id": feature_id,
            "message": "Feature center_mm is required for STEP feature-presence checking."
        });
    };
    let Some(axis) = feature
        .get("axis")
        .and_then(number_array)
        .and_then(Vec3::from_values)
        .and_then(Vec3::normalized)
    else {
        return json!({
            "rule_id": full_rule_id,
            "status": "fail",
            "reason": "missing_feature_axis",
            "feature_id": feature_id,
            "message": "Feature axis is required for STEP feature-presence checking."
        });
    };

    let evidence = match parse_step_evidence(&resolved_artifact.file_path) {
        Ok(evidence) => evidence,
        Err(error) => {
            return json!({
                "rule_id": full_rule_id,
                "status": "fail",
                "reason": "step_geometry_unreadable",
                "feature_id": feature_id,
                "measured": {
                    "artifact_path": resolved_artifact.label_path
                },
                "message": error
            });
        }
    };

    let diameter_tolerance = number_field(rule, "diameter_tolerance_mm")
        .unwrap_or(0.05)
        .max(0.0);
    let centerline_tolerance = number_field(rule, "centerline_tolerance_mm")
        .unwrap_or(0.25)
        .max(0.0);
    let axis_dot_min = number_field(rule, "axis_dot_min")
        .unwrap_or(0.99)
        .clamp(0.0, 1.0);
    let mut best: Option<CylinderMatch> = None;

    for cylinder in &evidence.cylinders {
        let axis_dot = axis.dot(cylinder.axis).abs();
        let diameter_delta = (cylinder.radius_mm * 2.0 - diameter).abs();
        let centerline_distance = cylinder.point.distance_to_line(center, cylinder.axis);
        let candidate = CylinderMatch {
            axis_dot,
            diameter_delta_mm: diameter_delta,
            centerline_distance_mm: centerline_distance,
        };
        if best
            .as_ref()
            .is_none_or(|best| candidate.score() < best.score())
        {
            best = Some(candidate);
        }
        if diameter_delta <= diameter_tolerance
            && axis_dot >= axis_dot_min
            && centerline_distance <= centerline_tolerance
        {
            return json!({
                "rule_id": full_rule_id,
                "status": "pass",
                "reason": "ok",
                "feature_id": feature_id,
                "measured": {
                    "artifact_path": resolved_artifact.label_path,
                    "candidate_cylinders": evidence.cylinders.len(),
                    "diameter_delta_mm": round(diameter_delta),
                    "axis_dot": round(axis_dot),
                    "centerline_distance_mm": round(centerline_distance)
                },
                "required": {
                    "diameter_tolerance_mm": diameter_tolerance,
                    "axis_dot_min": axis_dot_min,
                    "centerline_tolerance_mm": centerline_tolerance
                },
                "message": "Declared clearance-hole geometry exists in the STEP artifact."
            });
        }
    }

    json!({
        "rule_id": full_rule_id,
        "status": "fail",
        "reason": "missing_declared_feature",
        "feature_id": feature_id,
        "measured": {
            "artifact_path": resolved_artifact.label_path,
            "candidate_cylinders": evidence.cylinders.len(),
            "best_diameter_delta_mm": best.as_ref().map(|value| round(value.diameter_delta_mm)),
            "best_axis_dot": best.as_ref().map(|value| round(value.axis_dot)),
            "best_centerline_distance_mm": best.as_ref().map(|value| round(value.centerline_distance_mm))
        },
        "required": {
            "diameter_mm": diameter,
            "center_mm": center.to_json(),
            "axis": axis.to_json(),
            "diameter_tolerance_mm": diameter_tolerance,
            "axis_dot_min": axis_dot_min,
            "centerline_tolerance_mm": centerline_tolerance
        },
        "message": "Design data declares a clearance hole, but no matching cylindrical STEP geometry was found."
    })
}

fn check_bearing_seat_presence(
    full_rule_id: String,
    feature_id: Value,
    resolved_artifact: ResolvedFileRef,
    rule: &Value,
    feature: &Value,
) -> Value {
    let Some(seat_diameter) =
        number_field(feature, "seat_diameter_mm").filter(|value| *value > 0.0)
    else {
        return json!({
            "rule_id": full_rule_id,
            "status": "fail",
            "reason": "missing_seat_diameter",
            "feature_id": feature_id,
            "message": "Bearing seat_diameter_mm is required for STEP feature-presence checking."
        });
    };
    let Some(seat_depth) = number_field(feature, "seat_depth_mm").filter(|value| *value > 0.0)
    else {
        return json!({
            "rule_id": full_rule_id,
            "status": "fail",
            "reason": "missing_seat_depth",
            "feature_id": feature_id,
            "message": "Bearing seat_depth_mm is required for STEP feature-presence checking."
        });
    };
    let Some(center) = feature
        .get("center_mm")
        .and_then(number_array)
        .and_then(Vec3::from_values)
    else {
        return json!({
            "rule_id": full_rule_id,
            "status": "fail",
            "reason": "missing_feature_center",
            "feature_id": feature_id,
            "message": "Feature center_mm is required for STEP feature-presence checking."
        });
    };
    let Some(axis) = feature
        .get("axis")
        .and_then(number_array)
        .and_then(Vec3::from_values)
        .and_then(Vec3::normalized)
    else {
        return json!({
            "rule_id": full_rule_id,
            "status": "fail",
            "reason": "missing_feature_axis",
            "feature_id": feature_id,
            "message": "Feature axis is required for STEP feature-presence checking."
        });
    };
    let Some(seat_center) = feature
        .get("seat_center_mm")
        .and_then(number_array)
        .and_then(Vec3::from_values)
    else {
        return json!({
            "rule_id": full_rule_id,
            "status": "fail",
            "reason": "missing_seat_center",
            "feature_id": feature_id,
            "message": "Bearing seat_center_mm is required for STEP feature-presence checking."
        });
    };
    let Some(shoulder_center) = feature
        .get("shoulder_center_mm")
        .and_then(number_array)
        .and_then(Vec3::from_values)
    else {
        return json!({
            "rule_id": full_rule_id,
            "status": "fail",
            "reason": "missing_seat_shoulder_center",
            "feature_id": feature_id,
            "message": "Bearing shoulder_center_mm is required for STEP feature-presence checking."
        });
    };

    let evidence = match parse_step_evidence(&resolved_artifact.file_path) {
        Ok(evidence) => evidence,
        Err(error) => {
            return json!({
                "rule_id": full_rule_id,
                "status": "fail",
                "reason": "step_geometry_unreadable",
                "feature_id": feature_id,
                "measured": {
                    "artifact_path": resolved_artifact.label_path
                },
                "message": error
            });
        }
    };

    let seat_diameter_tolerance = number_field(rule, "seat_diameter_tolerance_mm")
        .or_else(|| number_field(rule, "diameter_tolerance_mm"))
        .unwrap_or(0.05)
        .max(0.0);
    let centerline_tolerance = number_field(rule, "centerline_tolerance_mm")
        .unwrap_or(0.25)
        .max(0.0);
    let seat_center_tolerance = number_field(rule, "seat_center_tolerance_mm")
        .unwrap_or(0.5)
        .max(0.0);
    let shoulder_plane_tolerance = number_field(rule, "shoulder_plane_tolerance_mm")
        .or_else(|| number_field(rule, "plane_tolerance_mm"))
        .unwrap_or(0.25)
        .max(0.0);
    let axis_dot_min = number_field(rule, "axis_dot_min")
        .unwrap_or(0.99)
        .clamp(0.0, 1.0);

    let mut best_seat: Option<CounterboreCylinderMatch> = None;
    let mut matched_seat = false;
    for cylinder in &evidence.cylinders {
        let axis_dot = axis.dot(cylinder.axis).abs();
        let diameter_delta = (cylinder.radius_mm * 2.0 - seat_diameter).abs();
        let centerline_distance = cylinder.point.distance_to_line(center, cylinder.axis);
        let axial_distance = cylinder.point.sub(seat_center).dot(axis).abs();
        let candidate = CounterboreCylinderMatch {
            axis_dot,
            diameter_delta_mm: diameter_delta,
            centerline_distance_mm: centerline_distance,
            axial_distance_mm: axial_distance,
        };
        if best_seat
            .as_ref()
            .is_none_or(|best| candidate.score() < best.score())
        {
            best_seat = Some(candidate);
        }
        if diameter_delta <= seat_diameter_tolerance
            && axis_dot >= axis_dot_min
            && centerline_distance <= centerline_tolerance
            && axial_distance <= (seat_depth / 2.0 + seat_center_tolerance)
        {
            matched_seat = true;
        }
    }

    let mut best_shoulder: Option<PlaneMatch> = None;
    let mut matched_shoulder = false;
    for plane in &evidence.planes {
        let normal_dot = axis.dot(plane.normal).abs();
        let distance = shoulder_center.sub(plane.point).dot(plane.normal).abs();
        let candidate = PlaneMatch {
            normal_dot,
            distance_mm: distance,
        };
        if best_shoulder
            .as_ref()
            .is_none_or(|best| candidate.score() < best.score())
        {
            best_shoulder = Some(candidate);
        }
        if normal_dot >= axis_dot_min && distance <= shoulder_plane_tolerance {
            matched_shoulder = true;
        }
    }

    let pass = matched_seat && matched_shoulder;

    json!({
        "rule_id": full_rule_id,
        "status": if pass { "pass" } else { "fail" },
        "reason": if pass { "ok" } else { "missing_declared_feature" },
        "feature_id": feature_id,
        "measured": {
            "artifact_path": resolved_artifact.label_path,
            "candidate_cylinders": evidence.cylinders.len(),
            "candidate_planes": evidence.planes.len(),
            "matched_seat_cylinder": matched_seat,
            "matched_seat_shoulder_plane": matched_shoulder,
            "best_seat_diameter_delta_mm": best_seat.as_ref().map(|value| round(value.diameter_delta_mm)),
            "best_seat_axis_dot": best_seat.as_ref().map(|value| round(value.axis_dot)),
            "best_seat_centerline_distance_mm": best_seat.as_ref().map(|value| round(value.centerline_distance_mm)),
            "best_seat_axial_distance_mm": best_seat.as_ref().map(|value| round(value.axial_distance_mm)),
            "best_shoulder_plane_normal_dot": best_shoulder.as_ref().map(|value| round(value.normal_dot)),
            "best_shoulder_plane_distance_mm": best_shoulder.as_ref().map(|value| round(value.distance_mm))
        },
        "required": {
            "seat_diameter_mm": seat_diameter,
            "seat_depth_mm": seat_depth,
            "center_mm": center.to_json(),
            "axis": axis.to_json(),
            "seat_center_mm": seat_center.to_json(),
            "shoulder_center_mm": shoulder_center.to_json(),
            "seat_diameter_tolerance_mm": seat_diameter_tolerance,
            "centerline_tolerance_mm": centerline_tolerance,
            "seat_center_tolerance_mm": seat_center_tolerance,
            "seat_axial_tolerance_mm": seat_depth / 2.0 + seat_center_tolerance,
            "shoulder_plane_tolerance_mm": shoulder_plane_tolerance,
            "axis_dot_min": axis_dot_min
        },
        "message": if pass {
            "Declared bearing seat cylinder and shoulder-plane geometry exists in the STEP artifact."
        } else {
            "Design data declares a bearing seat, but matching seated pocket geometry was not found."
        }
    })
}

fn check_standoff_boss_presence(
    full_rule_id: String,
    feature_id: Value,
    resolved_artifact: ResolvedFileRef,
    rule: &Value,
    feature: &Value,
) -> Value {
    let Some(boss_diameter) = number_field(feature, "boss_diameter_mm")
        .or_else(|| number_field(feature, "support_diameter_mm"))
        .or_else(|| number_field(feature, "outer_diameter_mm"))
        .filter(|value| *value > 0.0)
    else {
        return json!({
            "rule_id": full_rule_id,
            "status": "fail",
            "reason": "missing_boss_diameter",
            "feature_id": feature_id,
            "message": "Boss diameter is required for STEP feature-presence checking."
        });
    };
    let Some(boss_height) = number_field(feature, "boss_height_mm")
        .or_else(|| number_field(feature, "height_mm"))
        .filter(|value| *value > 0.0)
    else {
        return json!({
            "rule_id": full_rule_id,
            "status": "fail",
            "reason": "missing_boss_height",
            "feature_id": feature_id,
            "message": "Boss height is required for STEP feature-presence checking."
        });
    };
    let Some(axis) = feature
        .get("axis")
        .and_then(number_array)
        .and_then(Vec3::from_values)
        .and_then(Vec3::normalized)
    else {
        return json!({
            "rule_id": full_rule_id,
            "status": "fail",
            "reason": "missing_feature_axis",
            "feature_id": feature_id,
            "message": "Feature axis is required for STEP feature-presence checking."
        });
    };
    let Some(boss_center) = feature
        .get("boss_center_mm")
        .and_then(number_array)
        .and_then(Vec3::from_values)
        .or_else(|| {
            feature
                .get("center_mm")
                .and_then(number_array)
                .and_then(Vec3::from_values)
        })
    else {
        return json!({
            "rule_id": full_rule_id,
            "status": "fail",
            "reason": "missing_boss_center",
            "feature_id": feature_id,
            "message": "Boss boss_center_mm or center_mm is required for STEP feature-presence checking."
        });
    };
    let top_center = feature
        .get("top_center_mm")
        .and_then(number_array)
        .and_then(Vec3::from_values)
        .unwrap_or_else(|| boss_center.add(axis.scale(boss_height / 2.0)));

    let evidence = match parse_step_evidence(&resolved_artifact.file_path) {
        Ok(evidence) => evidence,
        Err(error) => {
            return json!({
                "rule_id": full_rule_id,
                "status": "fail",
                "reason": "step_geometry_unreadable",
                "feature_id": feature_id,
                "measured": {
                    "artifact_path": resolved_artifact.label_path
                },
                "message": error
            });
        }
    };

    let boss_diameter_tolerance = number_field(rule, "boss_diameter_tolerance_mm")
        .or_else(|| number_field(rule, "diameter_tolerance_mm"))
        .unwrap_or(0.05)
        .max(0.0);
    let centerline_tolerance = number_field(rule, "centerline_tolerance_mm")
        .unwrap_or(0.25)
        .max(0.0);
    let boss_center_tolerance = number_field(rule, "boss_center_tolerance_mm")
        .unwrap_or(0.5)
        .max(0.0);
    let top_plane_tolerance = number_field(rule, "top_plane_tolerance_mm")
        .or_else(|| number_field(rule, "plane_tolerance_mm"))
        .unwrap_or(0.25)
        .max(0.0);
    let axis_dot_min = number_field(rule, "axis_dot_min")
        .unwrap_or(0.99)
        .clamp(0.0, 1.0);

    let mut best_boss: Option<CounterboreCylinderMatch> = None;
    let mut matched_boss = false;
    for cylinder in &evidence.cylinders {
        let axis_dot = axis.dot(cylinder.axis).abs();
        let diameter_delta = (cylinder.radius_mm * 2.0 - boss_diameter).abs();
        let centerline_distance = cylinder.point.distance_to_line(boss_center, cylinder.axis);
        let axial_distance = cylinder.point.sub(boss_center).dot(axis).abs();
        let candidate = CounterboreCylinderMatch {
            axis_dot,
            diameter_delta_mm: diameter_delta,
            centerline_distance_mm: centerline_distance,
            axial_distance_mm: axial_distance,
        };
        if best_boss
            .as_ref()
            .is_none_or(|best| candidate.score() < best.score())
        {
            best_boss = Some(candidate);
        }
        if diameter_delta <= boss_diameter_tolerance
            && axis_dot >= axis_dot_min
            && centerline_distance <= centerline_tolerance
            && axial_distance <= (boss_height / 2.0 + boss_center_tolerance)
        {
            matched_boss = true;
        }
    }

    let mut best_top: Option<PlaneMatch> = None;
    let mut matched_top = false;
    for plane in &evidence.planes {
        let normal_dot = axis.dot(plane.normal).abs();
        let distance = top_center.sub(plane.point).dot(plane.normal).abs();
        let candidate = PlaneMatch {
            normal_dot,
            distance_mm: distance,
        };
        if best_top
            .as_ref()
            .is_none_or(|best| candidate.score() < best.score())
        {
            best_top = Some(candidate);
        }
        if normal_dot >= axis_dot_min && distance <= top_plane_tolerance {
            matched_top = true;
        }
    }

    let pass = matched_boss && matched_top;

    json!({
        "rule_id": full_rule_id,
        "status": if pass { "pass" } else { "fail" },
        "reason": if pass { "ok" } else { "missing_declared_feature" },
        "feature_id": feature_id,
        "measured": {
            "artifact_path": resolved_artifact.label_path,
            "candidate_cylinders": evidence.cylinders.len(),
            "candidate_planes": evidence.planes.len(),
            "matched_boss_cylinder": matched_boss,
            "matched_boss_top_plane": matched_top,
            "best_boss_diameter_delta_mm": best_boss.as_ref().map(|value| round(value.diameter_delta_mm)),
            "best_boss_axis_dot": best_boss.as_ref().map(|value| round(value.axis_dot)),
            "best_boss_centerline_distance_mm": best_boss.as_ref().map(|value| round(value.centerline_distance_mm)),
            "best_boss_axial_distance_mm": best_boss.as_ref().map(|value| round(value.axial_distance_mm)),
            "best_top_plane_normal_dot": best_top.as_ref().map(|value| round(value.normal_dot)),
            "best_top_plane_distance_mm": best_top.as_ref().map(|value| round(value.distance_mm))
        },
        "required": {
            "boss_diameter_mm": boss_diameter,
            "boss_height_mm": boss_height,
            "axis": axis.to_json(),
            "boss_center_mm": boss_center.to_json(),
            "top_center_mm": top_center.to_json(),
            "boss_diameter_tolerance_mm": boss_diameter_tolerance,
            "centerline_tolerance_mm": centerline_tolerance,
            "boss_center_tolerance_mm": boss_center_tolerance,
            "boss_axial_tolerance_mm": boss_height / 2.0 + boss_center_tolerance,
            "top_plane_tolerance_mm": top_plane_tolerance,
            "axis_dot_min": axis_dot_min
        },
        "message": if pass {
            "Declared standoff boss cylinder and top-plane geometry exists in the STEP artifact."
        } else {
            "Design data declares a standoff boss, but matching raised boss geometry was not found."
        }
    })
}

fn check_heat_set_insert_pocket_presence(
    full_rule_id: String,
    feature_id: Value,
    resolved_artifact: ResolvedFileRef,
    rule: &Value,
    feature: &Value,
) -> Value {
    let Some(pocket_diameter) = number_field(feature, "pocket_diameter_mm")
        .or_else(|| number_field(feature, "pilot_diameter_mm"))
        .filter(|value| *value > 0.0)
    else {
        return json!({
            "rule_id": full_rule_id,
            "status": "fail",
            "reason": "missing_pocket_diameter",
            "feature_id": feature_id,
            "message": "Heat-set insert pocket_diameter_mm is required for STEP feature-presence checking."
        });
    };
    let Some(pocket_depth) = number_field(feature, "pocket_depth_mm").filter(|value| *value > 0.0)
    else {
        return json!({
            "rule_id": full_rule_id,
            "status": "fail",
            "reason": "missing_pocket_depth",
            "feature_id": feature_id,
            "message": "Heat-set insert pocket_depth_mm is required for STEP feature-presence checking."
        });
    };
    let Some(center) = feature
        .get("center_mm")
        .and_then(number_array)
        .and_then(Vec3::from_values)
    else {
        return json!({
            "rule_id": full_rule_id,
            "status": "fail",
            "reason": "missing_feature_center",
            "feature_id": feature_id,
            "message": "Feature center_mm is required for STEP feature-presence checking."
        });
    };
    let Some(axis) = feature
        .get("axis")
        .and_then(number_array)
        .and_then(Vec3::from_values)
        .and_then(Vec3::normalized)
    else {
        return json!({
            "rule_id": full_rule_id,
            "status": "fail",
            "reason": "missing_feature_axis",
            "feature_id": feature_id,
            "message": "Feature axis is required for STEP feature-presence checking."
        });
    };
    let Some(pocket_center) = feature
        .get("pocket_center_mm")
        .and_then(number_array)
        .and_then(Vec3::from_values)
    else {
        return json!({
            "rule_id": full_rule_id,
            "status": "fail",
            "reason": "missing_pocket_center",
            "feature_id": feature_id,
            "message": "Heat-set insert pocket_center_mm is required for STEP feature-presence checking."
        });
    };
    let Some(bottom_center) = feature
        .get("bottom_center_mm")
        .and_then(number_array)
        .and_then(Vec3::from_values)
    else {
        return json!({
            "rule_id": full_rule_id,
            "status": "fail",
            "reason": "missing_pocket_bottom_center",
            "feature_id": feature_id,
            "message": "Heat-set insert bottom_center_mm is required for STEP feature-presence checking."
        });
    };

    let evidence = match parse_step_evidence(&resolved_artifact.file_path) {
        Ok(evidence) => evidence,
        Err(error) => {
            return json!({
                "rule_id": full_rule_id,
                "status": "fail",
                "reason": "step_geometry_unreadable",
                "feature_id": feature_id,
                "measured": {
                    "artifact_path": resolved_artifact.label_path
                },
                "message": error
            });
        }
    };

    let pocket_diameter_tolerance = number_field(rule, "pocket_diameter_tolerance_mm")
        .or_else(|| number_field(rule, "pilot_diameter_tolerance_mm"))
        .or_else(|| number_field(rule, "diameter_tolerance_mm"))
        .unwrap_or(0.05)
        .max(0.0);
    let centerline_tolerance = number_field(rule, "centerline_tolerance_mm")
        .unwrap_or(0.25)
        .max(0.0);
    let pocket_center_tolerance = number_field(rule, "pocket_center_tolerance_mm")
        .unwrap_or(0.5)
        .max(0.0);
    let bottom_plane_tolerance = number_field(rule, "bottom_plane_tolerance_mm")
        .or_else(|| number_field(rule, "plane_tolerance_mm"))
        .unwrap_or(0.25)
        .max(0.0);
    let axis_dot_min = number_field(rule, "axis_dot_min")
        .unwrap_or(0.99)
        .clamp(0.0, 1.0);

    let mut best_pocket: Option<CounterboreCylinderMatch> = None;
    let mut matched_pocket = false;
    for cylinder in &evidence.cylinders {
        let axis_dot = axis.dot(cylinder.axis).abs();
        let diameter_delta = (cylinder.radius_mm * 2.0 - pocket_diameter).abs();
        let centerline_distance = cylinder.point.distance_to_line(center, cylinder.axis);
        let axial_distance = cylinder.point.sub(pocket_center).dot(axis).abs();
        let candidate = CounterboreCylinderMatch {
            axis_dot,
            diameter_delta_mm: diameter_delta,
            centerline_distance_mm: centerline_distance,
            axial_distance_mm: axial_distance,
        };
        if best_pocket
            .as_ref()
            .is_none_or(|best| candidate.score() < best.score())
        {
            best_pocket = Some(candidate);
        }
        if diameter_delta <= pocket_diameter_tolerance
            && axis_dot >= axis_dot_min
            && centerline_distance <= centerline_tolerance
            && axial_distance <= (pocket_depth / 2.0 + pocket_center_tolerance)
        {
            matched_pocket = true;
        }
    }

    let mut best_bottom: Option<PlaneMatch> = None;
    let mut matched_bottom = false;
    for plane in &evidence.planes {
        let normal_dot = axis.dot(plane.normal).abs();
        let distance = bottom_center.sub(plane.point).dot(plane.normal).abs();
        let candidate = PlaneMatch {
            normal_dot,
            distance_mm: distance,
        };
        if best_bottom
            .as_ref()
            .is_none_or(|best| candidate.score() < best.score())
        {
            best_bottom = Some(candidate);
        }
        if normal_dot >= axis_dot_min && distance <= bottom_plane_tolerance {
            matched_bottom = true;
        }
    }

    let pass = matched_pocket && matched_bottom;

    json!({
        "rule_id": full_rule_id,
        "status": if pass { "pass" } else { "fail" },
        "reason": if pass { "ok" } else { "missing_declared_feature" },
        "feature_id": feature_id,
        "measured": {
            "artifact_path": resolved_artifact.label_path,
            "candidate_cylinders": evidence.cylinders.len(),
            "candidate_planes": evidence.planes.len(),
            "matched_pocket_cylinder": matched_pocket,
            "matched_pocket_bottom_plane": matched_bottom,
            "best_pocket_diameter_delta_mm": best_pocket.as_ref().map(|value| round(value.diameter_delta_mm)),
            "best_pocket_axis_dot": best_pocket.as_ref().map(|value| round(value.axis_dot)),
            "best_pocket_centerline_distance_mm": best_pocket.as_ref().map(|value| round(value.centerline_distance_mm)),
            "best_pocket_axial_distance_mm": best_pocket.as_ref().map(|value| round(value.axial_distance_mm)),
            "best_bottom_plane_normal_dot": best_bottom.as_ref().map(|value| round(value.normal_dot)),
            "best_bottom_plane_distance_mm": best_bottom.as_ref().map(|value| round(value.distance_mm))
        },
        "required": {
            "pocket_diameter_mm": pocket_diameter,
            "pocket_depth_mm": pocket_depth,
            "center_mm": center.to_json(),
            "axis": axis.to_json(),
            "pocket_center_mm": pocket_center.to_json(),
            "bottom_center_mm": bottom_center.to_json(),
            "pocket_diameter_tolerance_mm": pocket_diameter_tolerance,
            "centerline_tolerance_mm": centerline_tolerance,
            "pocket_center_tolerance_mm": pocket_center_tolerance,
            "pocket_axial_tolerance_mm": pocket_depth / 2.0 + pocket_center_tolerance,
            "bottom_plane_tolerance_mm": bottom_plane_tolerance,
            "axis_dot_min": axis_dot_min
        },
        "message": if pass {
            "Declared heat-set insert pocket cylinder and bottom-plane geometry exists in the STEP artifact."
        } else {
            "Design data declares a heat-set insert pocket, but matching blind pocket geometry was not found."
        }
    })
}

fn check_counterbore_presence(
    full_rule_id: String,
    feature_id: Value,
    resolved_artifact: ResolvedFileRef,
    rule: &Value,
    feature: &Value,
) -> Value {
    let Some(bore_diameter) = number_field(feature, "bore_diameter_mm")
        .or_else(|| number_field(feature, "diameter_mm"))
        .filter(|value| *value > 0.0)
    else {
        return json!({
            "rule_id": full_rule_id,
            "status": "fail",
            "reason": "missing_bore_diameter",
            "feature_id": feature_id,
            "message": "Counterbore bore_diameter_mm is required for STEP feature-presence checking."
        });
    };
    let Some(counterbore_diameter) =
        number_field(feature, "counterbore_diameter_mm").filter(|value| *value > 0.0)
    else {
        return json!({
            "rule_id": full_rule_id,
            "status": "fail",
            "reason": "missing_counterbore_diameter",
            "feature_id": feature_id,
            "message": "Counterbore counterbore_diameter_mm is required for STEP feature-presence checking."
        });
    };
    let Some(counterbore_depth) =
        number_field(feature, "counterbore_depth_mm").filter(|value| *value > 0.0)
    else {
        return json!({
            "rule_id": full_rule_id,
            "status": "fail",
            "reason": "missing_counterbore_depth",
            "feature_id": feature_id,
            "message": "Counterbore counterbore_depth_mm is required for STEP feature-presence checking."
        });
    };
    if counterbore_diameter <= bore_diameter {
        return json!({
            "rule_id": full_rule_id,
            "status": "fail",
            "reason": "invalid_counterbore_dimensions",
            "feature_id": feature_id,
            "message": "Counterbore diameter must be greater than bore diameter."
        });
    }
    let Some(center) = feature
        .get("center_mm")
        .and_then(number_array)
        .and_then(Vec3::from_values)
    else {
        return json!({
            "rule_id": full_rule_id,
            "status": "fail",
            "reason": "missing_feature_center",
            "feature_id": feature_id,
            "message": "Feature center_mm is required for STEP feature-presence checking."
        });
    };
    let Some(axis) = feature
        .get("axis")
        .and_then(number_array)
        .and_then(Vec3::from_values)
        .and_then(Vec3::normalized)
    else {
        return json!({
            "rule_id": full_rule_id,
            "status": "fail",
            "reason": "missing_feature_axis",
            "feature_id": feature_id,
            "message": "Feature axis is required for STEP feature-presence checking."
        });
    };
    let Some(counterbore_center) = feature
        .get("counterbore_center_mm")
        .and_then(number_array)
        .and_then(Vec3::from_values)
    else {
        return json!({
            "rule_id": full_rule_id,
            "status": "fail",
            "reason": "missing_counterbore_center",
            "feature_id": feature_id,
            "message": "Counterbore counterbore_center_mm is required for STEP feature-presence checking."
        });
    };
    let Some(shoulder_center) = feature
        .get("shoulder_center_mm")
        .and_then(number_array)
        .and_then(Vec3::from_values)
    else {
        return json!({
            "rule_id": full_rule_id,
            "status": "fail",
            "reason": "missing_counterbore_shoulder_center",
            "feature_id": feature_id,
            "message": "Counterbore shoulder_center_mm is required for STEP feature-presence checking."
        });
    };

    let evidence = match parse_step_evidence(&resolved_artifact.file_path) {
        Ok(evidence) => evidence,
        Err(error) => {
            return json!({
                "rule_id": full_rule_id,
                "status": "fail",
                "reason": "step_geometry_unreadable",
                "feature_id": feature_id,
                "measured": {
                    "artifact_path": resolved_artifact.label_path
                },
                "message": error
            });
        }
    };

    let bore_diameter_tolerance = number_field(rule, "bore_diameter_tolerance_mm")
        .or_else(|| number_field(rule, "diameter_tolerance_mm"))
        .unwrap_or(0.05)
        .max(0.0);
    let counterbore_diameter_tolerance = number_field(rule, "counterbore_diameter_tolerance_mm")
        .or_else(|| number_field(rule, "diameter_tolerance_mm"))
        .unwrap_or(0.05)
        .max(0.0);
    let centerline_tolerance = number_field(rule, "centerline_tolerance_mm")
        .unwrap_or(0.25)
        .max(0.0);
    let counterbore_center_tolerance = number_field(rule, "counterbore_center_tolerance_mm")
        .unwrap_or(0.5)
        .max(0.0);
    let shoulder_plane_tolerance = number_field(rule, "shoulder_plane_tolerance_mm")
        .or_else(|| number_field(rule, "plane_tolerance_mm"))
        .unwrap_or(0.25)
        .max(0.0);
    let axis_dot_min = number_field(rule, "axis_dot_min")
        .unwrap_or(0.99)
        .clamp(0.0, 1.0);

    let mut best_bore: Option<CylinderMatch> = None;
    let mut matched_bore = false;
    let mut best_counterbore: Option<CounterboreCylinderMatch> = None;
    let mut matched_counterbore = false;

    for cylinder in &evidence.cylinders {
        let axis_dot = axis.dot(cylinder.axis).abs();
        let bore_diameter_delta = (cylinder.radius_mm * 2.0 - bore_diameter).abs();
        let centerline_distance = cylinder.point.distance_to_line(center, cylinder.axis);
        let bore_candidate = CylinderMatch {
            axis_dot,
            diameter_delta_mm: bore_diameter_delta,
            centerline_distance_mm: centerline_distance,
        };
        if best_bore
            .as_ref()
            .is_none_or(|best| bore_candidate.score() < best.score())
        {
            best_bore = Some(bore_candidate);
        }
        if bore_diameter_delta <= bore_diameter_tolerance
            && axis_dot >= axis_dot_min
            && centerline_distance <= centerline_tolerance
        {
            matched_bore = true;
        }

        let counterbore_diameter_delta = (cylinder.radius_mm * 2.0 - counterbore_diameter).abs();
        let counterbore_centerline_distance =
            cylinder.point.distance_to_line(center, cylinder.axis);
        let counterbore_axial_distance = cylinder.point.sub(counterbore_center).dot(axis).abs();
        let counterbore_candidate = CounterboreCylinderMatch {
            axis_dot,
            diameter_delta_mm: counterbore_diameter_delta,
            centerline_distance_mm: counterbore_centerline_distance,
            axial_distance_mm: counterbore_axial_distance,
        };
        if best_counterbore
            .as_ref()
            .is_none_or(|best| counterbore_candidate.score() < best.score())
        {
            best_counterbore = Some(counterbore_candidate);
        }
        if counterbore_diameter_delta <= counterbore_diameter_tolerance
            && axis_dot >= axis_dot_min
            && counterbore_centerline_distance <= centerline_tolerance
            && counterbore_axial_distance
                <= (counterbore_depth / 2.0 + counterbore_center_tolerance)
        {
            matched_counterbore = true;
        }
    }

    let mut best_shoulder: Option<PlaneMatch> = None;
    let mut matched_shoulder = false;
    for plane in &evidence.planes {
        let normal_dot = axis.dot(plane.normal).abs();
        let distance = shoulder_center.sub(plane.point).dot(plane.normal).abs();
        let candidate = PlaneMatch {
            normal_dot,
            distance_mm: distance,
        };
        if best_shoulder
            .as_ref()
            .is_none_or(|best| candidate.score() < best.score())
        {
            best_shoulder = Some(candidate);
        }
        if normal_dot >= axis_dot_min && distance <= shoulder_plane_tolerance {
            matched_shoulder = true;
        }
    }

    let pass = matched_bore && matched_counterbore && matched_shoulder;

    json!({
        "rule_id": full_rule_id,
        "status": if pass { "pass" } else { "fail" },
        "reason": if pass { "ok" } else { "missing_declared_feature" },
        "feature_id": feature_id,
        "measured": {
            "artifact_path": resolved_artifact.label_path,
            "candidate_cylinders": evidence.cylinders.len(),
            "candidate_planes": evidence.planes.len(),
            "matched_bore_cylinder": matched_bore,
            "matched_counterbore_cylinder": matched_counterbore,
            "matched_counterbore_shoulder_plane": matched_shoulder,
            "best_bore_diameter_delta_mm": best_bore.as_ref().map(|value| round(value.diameter_delta_mm)),
            "best_bore_axis_dot": best_bore.as_ref().map(|value| round(value.axis_dot)),
            "best_bore_centerline_distance_mm": best_bore.as_ref().map(|value| round(value.centerline_distance_mm)),
            "best_counterbore_diameter_delta_mm": best_counterbore.as_ref().map(|value| round(value.diameter_delta_mm)),
            "best_counterbore_axis_dot": best_counterbore.as_ref().map(|value| round(value.axis_dot)),
            "best_counterbore_centerline_distance_mm": best_counterbore.as_ref().map(|value| round(value.centerline_distance_mm)),
            "best_counterbore_axial_distance_mm": best_counterbore.as_ref().map(|value| round(value.axial_distance_mm)),
            "best_shoulder_plane_normal_dot": best_shoulder.as_ref().map(|value| round(value.normal_dot)),
            "best_shoulder_plane_distance_mm": best_shoulder.as_ref().map(|value| round(value.distance_mm))
        },
        "required": {
            "bore_diameter_mm": bore_diameter,
            "counterbore_diameter_mm": counterbore_diameter,
            "counterbore_depth_mm": counterbore_depth,
            "center_mm": center.to_json(),
            "axis": axis.to_json(),
            "counterbore_center_mm": counterbore_center.to_json(),
            "shoulder_center_mm": shoulder_center.to_json(),
            "bore_diameter_tolerance_mm": bore_diameter_tolerance,
            "counterbore_diameter_tolerance_mm": counterbore_diameter_tolerance,
            "centerline_tolerance_mm": centerline_tolerance,
            "counterbore_center_tolerance_mm": counterbore_center_tolerance,
            "counterbore_axial_tolerance_mm": counterbore_depth / 2.0 + counterbore_center_tolerance,
            "shoulder_plane_tolerance_mm": shoulder_plane_tolerance,
            "axis_dot_min": axis_dot_min
        },
        "message": if pass {
            "Declared counterbore bore, counterbore, and shoulder geometry exists in the STEP artifact."
        } else {
            "Design data declares a counterbore, but matching bore, counterbore, and shoulder geometry was not found."
        }
    })
}

fn check_straight_slot_presence(
    full_rule_id: String,
    feature_id: Value,
    resolved_artifact: ResolvedFileRef,
    rule: &Value,
    feature: &Value,
) -> Value {
    let Some(width) = number_field(feature, "width_mm").filter(|value| *value > 0.0) else {
        return json!({
            "rule_id": full_rule_id,
            "status": "fail",
            "reason": "missing_slot_width",
            "feature_id": feature_id,
            "message": "Slot width_mm is required for STEP feature-presence checking."
        });
    };
    let Some(length) = number_field(feature, "length_mm").filter(|value| *value > width) else {
        return json!({
            "rule_id": full_rule_id,
            "status": "fail",
            "reason": "missing_slot_length",
            "feature_id": feature_id,
            "message": "Slot length_mm must be greater than width_mm for STEP feature-presence checking."
        });
    };
    let Some(center) = feature
        .get("center_mm")
        .and_then(number_array)
        .and_then(Vec3::from_values)
    else {
        return json!({
            "rule_id": full_rule_id,
            "status": "fail",
            "reason": "missing_feature_center",
            "feature_id": feature_id,
            "message": "Feature center_mm is required for STEP feature-presence checking."
        });
    };
    let Some(axis) = feature
        .get("axis")
        .and_then(number_array)
        .and_then(Vec3::from_values)
        .and_then(Vec3::normalized)
    else {
        return json!({
            "rule_id": full_rule_id,
            "status": "fail",
            "reason": "missing_feature_axis",
            "feature_id": feature_id,
            "message": "Feature axis is required for STEP feature-presence checking."
        });
    };
    let Some(span_axis) = feature
        .get("span_axis")
        .and_then(number_array)
        .and_then(Vec3::from_values)
        .and_then(Vec3::normalized)
    else {
        return json!({
            "rule_id": full_rule_id,
            "status": "fail",
            "reason": "missing_slot_span_axis",
            "feature_id": feature_id,
            "message": "Slot span_axis is required for STEP feature-presence checking."
        });
    };

    let evidence = match parse_step_evidence(&resolved_artifact.file_path) {
        Ok(evidence) => evidence,
        Err(error) => {
            return json!({
                "rule_id": full_rule_id,
                "status": "fail",
                "reason": "step_geometry_unreadable",
                "feature_id": feature_id,
                "measured": {
                    "artifact_path": resolved_artifact.label_path
                },
                "message": error
            });
        }
    };

    let width_tolerance = number_field(rule, "width_tolerance_mm")
        .or_else(|| number_field(rule, "diameter_tolerance_mm"))
        .unwrap_or(0.05)
        .max(0.0);
    let endpoint_tolerance = number_field(rule, "endpoint_tolerance_mm")
        .or_else(|| number_field(rule, "centerline_tolerance_mm"))
        .unwrap_or(0.25)
        .max(0.0);
    let axis_dot_min = number_field(rule, "axis_dot_min")
        .unwrap_or(0.99)
        .clamp(0.0, 1.0);
    let side_plane_tolerance = number_field(rule, "side_plane_tolerance_mm")
        .or_else(|| number_field(rule, "endpoint_tolerance_mm"))
        .or_else(|| number_field(rule, "centerline_tolerance_mm"))
        .unwrap_or(0.25)
        .max(0.0);
    let Some(width_axis) = axis.cross(span_axis).normalized() else {
        return json!({
            "rule_id": full_rule_id,
            "status": "fail",
            "reason": "invalid_slot_axes",
            "feature_id": feature_id,
            "message": "Slot axis and span_axis must be perpendicular."
        });
    };
    if axis.dot(span_axis).abs() > 0.001 {
        return json!({
            "rule_id": full_rule_id,
            "status": "fail",
            "reason": "invalid_slot_axes",
            "feature_id": feature_id,
            "message": "Slot axis and span_axis must be perpendicular."
        });
    }
    let endpoint_offset = (length - width) / 2.0;
    let endpoints = [
        center.add(span_axis.scale(-endpoint_offset)),
        center.add(span_axis.scale(endpoint_offset)),
    ];
    let mut endpoint_matches = Vec::new();

    for endpoint in endpoints {
        let mut best: Option<CylinderMatch> = None;
        let mut matched = false;
        for cylinder in &evidence.cylinders {
            let axis_dot = axis.dot(cylinder.axis).abs();
            let width_delta = (cylinder.radius_mm * 2.0 - width).abs();
            let centerline_distance = cylinder.point.distance_to_line(endpoint, cylinder.axis);
            let candidate = CylinderMatch {
                axis_dot,
                diameter_delta_mm: width_delta,
                centerline_distance_mm: centerline_distance,
            };
            if best
                .as_ref()
                .is_none_or(|best| candidate.score() < best.score())
            {
                best = Some(candidate);
            }
            if width_delta <= width_tolerance
                && axis_dot >= axis_dot_min
                && centerline_distance <= endpoint_tolerance
            {
                matched = true;
            }
        }
        endpoint_matches.push(json!({
            "matched": matched,
            "best_width_delta_mm": best.as_ref().map(|value| round(value.diameter_delta_mm)),
            "best_axis_dot": best.as_ref().map(|value| round(value.axis_dot)),
            "best_endpoint_distance_mm": best.as_ref().map(|value| round(value.centerline_distance_mm))
        }));
    }

    let matched_endpoints = endpoint_matches
        .iter()
        .filter(|value| value.get("matched").and_then(Value::as_bool) == Some(true))
        .count();
    let side_points = [
        center.add(width_axis.scale(-(width / 2.0))),
        center.add(width_axis.scale(width / 2.0)),
    ];
    let mut side_plane_matches = Vec::new();
    for side_point in side_points {
        let mut best: Option<PlaneMatch> = None;
        let mut matched = false;
        for plane in &evidence.planes {
            let normal_dot = width_axis.dot(plane.normal).abs();
            let distance = side_point.sub(plane.point).dot(plane.normal).abs();
            let candidate = PlaneMatch {
                normal_dot,
                distance_mm: distance,
            };
            if best
                .as_ref()
                .is_none_or(|best| candidate.score() < best.score())
            {
                best = Some(candidate);
            }
            if normal_dot >= axis_dot_min && distance <= side_plane_tolerance {
                matched = true;
            }
        }
        side_plane_matches.push(json!({
            "matched": matched,
            "best_normal_dot": best.as_ref().map(|value| round(value.normal_dot)),
            "best_plane_distance_mm": best.as_ref().map(|value| round(value.distance_mm))
        }));
    }
    let matched_side_planes = side_plane_matches
        .iter()
        .filter(|value| value.get("matched").and_then(Value::as_bool) == Some(true))
        .count();
    let pass = matched_endpoints == 2 && matched_side_planes == 2;

    json!({
        "rule_id": full_rule_id,
        "status": if pass { "pass" } else { "fail" },
        "reason": if pass { "ok" } else { "missing_declared_feature" },
        "feature_id": feature_id,
        "measured": {
            "artifact_path": resolved_artifact.label_path,
            "candidate_cylinders": evidence.cylinders.len(),
            "candidate_planes": evidence.planes.len(),
            "matched_slot_endpoints": matched_endpoints,
            "matched_slot_side_planes": matched_side_planes,
            "slot_endpoint_matches": endpoint_matches,
            "slot_side_plane_matches": side_plane_matches
        },
        "required": {
            "width_mm": width,
            "length_mm": length,
            "center_mm": center.to_json(),
            "axis": axis.to_json(),
            "span_axis": span_axis.to_json(),
            "width_axis": width_axis.to_json(),
            "width_tolerance_mm": width_tolerance,
            "axis_dot_min": axis_dot_min,
            "endpoint_tolerance_mm": endpoint_tolerance,
            "side_plane_tolerance_mm": side_plane_tolerance
        },
        "message": if pass {
            "Declared straight-slot endpoint and side-plane geometry exists in the STEP artifact."
        } else {
            "Design data declares a straight slot, but matching endpoint cylinders and side planes were not found."
        }
    })
}

fn summarize_features(manifest: &Value, checks: &[Value]) -> Value {
    let declared_features: Vec<&Value> = manifest
        .get("features")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .collect();
    let mut checked_feature_ids: HashSet<String> = HashSet::new();
    for check in checks {
        if let Some(feature_id) = string_field(check, "feature_id") {
            checked_feature_ids.insert(feature_id.to_string());
        }
        for feature_id in check
            .get("feature_ids")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
        {
            checked_feature_ids.insert(feature_id.to_string());
        }
    }
    let mut checked = Vec::new();
    let mut unchecked = Vec::new();
    let mut intent_counts: HashMap<String, usize> = HashMap::new();

    for feature in declared_features {
        for intent in feature_intents(feature) {
            *intent_counts.entry(intent).or_insert(0) += 1;
        }
        let feature_id = string_field(feature, "id");
        if feature_id.is_some_and(|id| checked_feature_ids.contains(id)) {
            checked.push(Value::String(feature_id.unwrap().to_string()));
        } else if let Some(id) = feature_id {
            unchecked.push(Value::String(id.to_string()));
        }
    }

    let candidate_cylinders_considered = checks
        .iter()
        .filter_map(|check| {
            check
                .pointer("/measured/candidate_cylinders")
                .and_then(Value::as_u64)
        })
        .max()
        .unwrap_or(0);
    let mut intent_values = serde_json::Map::new();
    for (intent, count) in intent_counts {
        intent_values.insert(intent, json!(count));
    }

    json!({
        "declared": checked.len() + unchecked.len(),
        "checked": checked.len(),
        "unchecked": unchecked.len(),
        "checked_feature_ids": checked,
        "unchecked_feature_ids": unchecked,
        "intent_counts": intent_values,
        "step_candidate_cylinders_considered": candidate_cylinders_considered
    })
}

#[derive(Debug, Clone, Copy)]
struct Vec3 {
    x: f64,
    y: f64,
    z: f64,
}

impl Vec3 {
    fn from_values(values: Vec<f64>) -> Option<Self> {
        if values.len() != 3 {
            return None;
        }
        Some(Self {
            x: values[0],
            y: values[1],
            z: values[2],
        })
    }

    fn normalized(self) -> Option<Self> {
        let length = self.length();
        if !length.is_finite() || length <= f64::EPSILON {
            return None;
        }
        Some(Self {
            x: self.x / length,
            y: self.y / length,
            z: self.z / length,
        })
    }

    fn dot(self, other: Self) -> f64 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    fn cross(self, other: Self) -> Self {
        Self {
            x: self.y * other.z - self.z * other.y,
            y: self.z * other.x - self.x * other.z,
            z: self.x * other.y - self.y * other.x,
        }
    }

    fn length(self) -> f64 {
        self.dot(self).sqrt()
    }

    fn sub(self, other: Self) -> Self {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
            z: self.z - other.z,
        }
    }

    fn add(self, other: Self) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
            z: self.z + other.z,
        }
    }

    fn scale(self, factor: f64) -> Self {
        Self {
            x: self.x * factor,
            y: self.y * factor,
            z: self.z * factor,
        }
    }

    fn coordinate(self, axis: usize) -> f64 {
        match axis {
            0 => self.x,
            1 => self.y,
            2 => self.z,
            _ => f64::NAN,
        }
    }

    fn distance_to_line(self, line_point: Self, line_axis: Self) -> f64 {
        self.sub(line_point).cross(line_axis).length()
    }

    fn to_json(self) -> Value {
        json!([round(self.x), round(self.y), round(self.z)])
    }
}

#[derive(Debug)]
struct StepCylinder {
    point: Vec3,
    axis: Vec3,
    radius_mm: f64,
}

#[derive(Debug, Default)]
struct StepEvidence {
    cylinders: Vec<StepCylinder>,
    planes: Vec<StepPlane>,
}

#[derive(Debug)]
struct StepPlane {
    point: Vec3,
    normal: Vec3,
}

#[derive(Debug)]
struct CylinderMatch {
    axis_dot: f64,
    diameter_delta_mm: f64,
    centerline_distance_mm: f64,
}

impl CylinderMatch {
    fn score(&self) -> f64 {
        self.diameter_delta_mm + (1.0 - self.axis_dot).abs() + self.centerline_distance_mm
    }
}

#[derive(Debug)]
struct CounterboreCylinderMatch {
    axis_dot: f64,
    diameter_delta_mm: f64,
    centerline_distance_mm: f64,
    axial_distance_mm: f64,
}

impl CounterboreCylinderMatch {
    fn score(&self) -> f64 {
        self.diameter_delta_mm
            + (1.0 - self.axis_dot).abs()
            + self.centerline_distance_mm
            + self.axial_distance_mm
    }
}

#[derive(Debug)]
struct PlaneMatch {
    normal_dot: f64,
    distance_mm: f64,
}

impl PlaneMatch {
    fn score(&self) -> f64 {
        (1.0 - self.normal_dot).abs() + self.distance_mm
    }
}

fn parse_step_evidence(path: &Path) -> Result<StepEvidence, String> {
    if std::env::var("BURR_STEP_CYLINDER_BACKEND")
        .ok()
        .is_some_and(|backend| backend == "ocp")
    {
        return parse_step_evidence_with_ocp(path);
    }
    parse_step_evidence_from_text(path)
}

fn parse_step_evidence_with_ocp(path: &Path) -> Result<StepEvidence, String> {
    let command = std::env::var("BURR_OCP_STEP_CYLINDERS")
        .unwrap_or_else(|_| "burr-ocp-step-cylinders".to_string());
    let mut parts = command.split_whitespace();
    let Some(program) = parts.next() else {
        return Err("BURR_OCP_STEP_CYLINDERS is empty.".to_string());
    };
    let output = Command::new(program)
        .args(parts)
        .arg(path)
        .output()
        .map_err(|error| {
            format!(
                "Failed to run OCP STEP cylinder extractor `{command}`: {error}. Install optional package `burr-ocp` or unset BURR_STEP_CYLINDER_BACKEND."
            )
        })?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(format!(
            "OCP STEP cylinder extractor failed with exit {}: {}",
            output.status.code().unwrap_or(-1),
            if stderr.is_empty() {
                "<no stderr>"
            } else {
                stderr.as_str()
            }
        ));
    }
    let stdout = String::from_utf8(output.stdout).map_err(|error| {
        format!("OCP STEP cylinder extractor returned non-UTF8 output: {error}")
    })?;
    parse_ocp_step_evidence_json(&stdout)
}

fn parse_ocp_step_evidence_json(text: &str) -> Result<StepEvidence, String> {
    let value = read_json_str(text)
        .map_err(|error| format!("Failed to parse OCP STEP cylinder JSON: {error}"))?;
    if string_field(&value, "schema_version") != Some("burr.ocp-step-cylinders.v1") {
        return Err("OCP STEP cylinder JSON has an unsupported schema_version.".to_string());
    }
    let mut evidence = StepEvidence::default();
    for cylinder in value
        .get("cylinders")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let Some(point) = cylinder
            .get("point_mm")
            .and_then(number_array)
            .and_then(Vec3::from_values)
        else {
            continue;
        };
        let Some(axis) = cylinder
            .get("axis")
            .and_then(number_array)
            .and_then(Vec3::from_values)
            .and_then(Vec3::normalized)
        else {
            continue;
        };
        let Some(radius) = number_field(cylinder, "radius_mm").filter(|value| *value > 0.0) else {
            continue;
        };
        evidence.cylinders.push(StepCylinder {
            point,
            axis,
            radius_mm: radius,
        });
    }
    for plane in value
        .get("planes")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let Some(point) = plane
            .get("point_mm")
            .and_then(number_array)
            .and_then(Vec3::from_values)
        else {
            continue;
        };
        let Some(normal) = plane
            .get("normal")
            .and_then(number_array)
            .and_then(Vec3::from_values)
            .and_then(Vec3::normalized)
        else {
            continue;
        };
        evidence.planes.push(StepPlane { point, normal });
    }
    Ok(evidence)
}

#[cfg(test)]
fn parse_ocp_step_cylinders_json(text: &str) -> Result<Vec<StepCylinder>, String> {
    parse_ocp_step_evidence_json(text).map(|evidence| evidence.cylinders)
}

fn parse_step_evidence_from_text(path: &Path) -> Result<StepEvidence, String> {
    let text = fs::read_to_string(path)
        .map_err(|error| format!("Failed to read STEP artifact {}: {error}", path.display()))?;
    let entities = collect_step_entities(&text);
    let mut points = HashMap::new();
    let mut directions = HashMap::new();

    for (id, entity) in &entities {
        if entity.starts_with("CARTESIAN_POINT") {
            if let Some(point) = parse_vector_entity(entity) {
                points.insert(id.clone(), point);
            }
        } else if entity.starts_with("DIRECTION") {
            if let Some(direction) = parse_vector_entity(entity).and_then(Vec3::normalized) {
                directions.insert(id.clone(), direction);
            }
        }
    }

    let mut axes = HashMap::new();
    for (id, entity) in &entities {
        if !entity.starts_with("AXIS2_PLACEMENT_3D") {
            continue;
        }
        let refs = step_refs(entity);
        if refs.len() < 2 {
            continue;
        }
        let Some(point) = points.get(&refs[0]).copied() else {
            continue;
        };
        let Some(axis) = directions.get(&refs[1]).copied() else {
            continue;
        };
        axes.insert(id.clone(), (point, axis));
    }

    let mut evidence = StepEvidence::default();
    for entity in entities.values() {
        if entity.starts_with("CYLINDRICAL_SURFACE") {
            let refs = step_refs(entity);
            let Some(axis_ref) = refs.first() else {
                continue;
            };
            let Some((point, axis)) = axes.get(axis_ref).copied() else {
                continue;
            };
            let Some(radius) = parse_last_step_number(entity).filter(|value| *value > 0.0) else {
                continue;
            };
            evidence.cylinders.push(StepCylinder {
                point,
                axis,
                radius_mm: radius,
            });
        } else if entity.starts_with("PLANE") {
            let refs = step_refs(entity);
            let Some(axis_ref) = refs.first() else {
                continue;
            };
            let Some((point, normal)) = axes.get(axis_ref).copied() else {
                continue;
            };
            evidence.planes.push(StepPlane { point, normal });
        }
    }

    Ok(evidence)
}

fn collect_step_entities(text: &str) -> HashMap<String, String> {
    let mut entities = HashMap::new();
    let mut current = String::new();

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') && current.is_empty() {
            current.push_str(trimmed);
        } else if !current.is_empty() {
            current.push(' ');
            current.push_str(trimmed);
        }

        if current.ends_with(';') {
            if let Some((id, entity)) = parse_step_entity(&current) {
                entities.insert(id, entity);
            }
            current.clear();
        }
    }

    entities
}

fn parse_step_entity(text: &str) -> Option<(String, String)> {
    let (id, entity) = text.split_once('=')?;
    let id = id.trim();
    if !id.starts_with('#') {
        return None;
    }
    Some((
        id.to_string(),
        entity.trim().trim_end_matches(';').trim().to_string(),
    ))
}

fn parse_vector_entity(entity: &str) -> Option<Vec3> {
    let start = entity.find(",(")? + 2;
    let rest = &entity[start..];
    let end = rest.find(')')?;
    parse_step_numbers(&rest[..end]).and_then(Vec3::from_values)
}

fn parse_last_step_number(entity: &str) -> Option<f64> {
    let trimmed = entity.trim().trim_end_matches(')');
    let (_, last) = trimmed.rsplit_once(',')?;
    last.trim()
        .parse::<f64>()
        .ok()
        .filter(|value| value.is_finite())
}

fn parse_step_numbers(text: &str) -> Option<Vec<f64>> {
    text.split(',')
        .map(|part| {
            part.trim()
                .parse::<f64>()
                .ok()
                .filter(|value| value.is_finite())
        })
        .collect()
}

fn step_refs(text: &str) -> Vec<String> {
    let bytes = text.as_bytes();
    let mut refs = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] != b'#' {
            index += 1;
            continue;
        }
        let start = index;
        index += 1;
        while index < bytes.len() && bytes[index].is_ascii_digit() {
            index += 1;
        }
        if index > start + 1 {
            refs.push(text[start..index].to_string());
        }
    }
    refs
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

fn design_data_rulepack_path(
    manifest: &Value,
    manifest_dir: &Path,
) -> Result<Option<PathBuf>, String> {
    let Some(rulepack) = manifest.get("rulepack") else {
        return Ok(None);
    };
    let path = if let Some(path) = rulepack.as_str() {
        path
    } else if let Some(object) = rulepack.as_object() {
        let Some(path) = object.get("path") else {
            return Err("Manifest rulepack object must include path.".to_string());
        };
        path.as_str()
            .ok_or_else(|| "Manifest rulepack path must be a string.".to_string())?
    } else {
        return Err("Manifest rulepack must be a string path or object with path.".to_string());
    };
    if path.is_empty() {
        return Err("Manifest rulepack path must be a non-empty string.".to_string());
    }
    Ok(Some(normalize_path(&manifest_dir.join(path))))
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
    if let Some(id) = string_field(applies_to, "id") {
        if string_field(feature, "id") != Some(id) {
            return false;
        }
    }
    if let Some(kind) = string_field(applies_to, "kind") {
        if string_field(feature, "kind") != Some(kind) {
            return false;
        }
    }
    if let Some(kind_any) = applies_to.get("kind_any").and_then(Value::as_array) {
        if !kind_any.is_empty() {
            let Some(kind) = string_field(feature, "kind") else {
                return false;
            };
            let allowed: HashSet<&str> = kind_any.iter().filter_map(Value::as_str).collect();
            if !allowed.contains(kind) {
                return false;
            }
        }
    }
    if let Some(fastener) = string_field(applies_to, "fastener") {
        if string_field(feature, "fastener") != Some(fastener) {
            return false;
        }
    }
    if let Some(intent) = string_field(applies_to, "intent") {
        if !feature_intents(feature).iter().any(|value| value == intent) {
            return false;
        }
    }
    if let Some(intent_any) = applies_to.get("intent_any").and_then(Value::as_array) {
        if !intent_any.is_empty() {
            let intents = feature_intents(feature);
            let allowed: HashSet<&str> = intent_any.iter().filter_map(Value::as_str).collect();
            if !intents
                .iter()
                .any(|intent| allowed.contains(intent.as_str()))
            {
                return false;
            }
        }
    }
    if let Some(role_any) = applies_to.get("role_any").and_then(Value::as_array) {
        if !role_any.is_empty() {
            let roles = normalize_string_values(feature.get("role"));
            let allowed: HashSet<&str> = role_any.iter().filter_map(Value::as_str).collect();
            if !roles.iter().any(|role| allowed.contains(role.as_str())) {
                return false;
            }
        }
    }
    true
}

fn feature_intents(feature: &Value) -> Vec<String> {
    let intents = normalize_string_values(feature.get("intent"));
    if intents.is_empty() {
        vec!["mechanical_interface".to_string()]
    } else {
        intents
    }
}

fn normalize_string_values(value: Option<&Value>) -> Vec<String> {
    match value {
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

fn find_feature<'a>(manifest: &'a Value, feature_id: &str) -> Option<&'a Value> {
    manifest
        .get("features")?
        .as_array()?
        .iter()
        .find(|feature| string_field(feature, "id") == Some(feature_id))
}

fn find_artifact<'a>(manifest: &'a Value, kind: &str) -> Option<&'a Value> {
    manifest
        .get("artifacts")?
        .as_array()?
        .iter()
        .find(|artifact| string_field(artifact, "kind") == Some(kind))
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

fn project_name_from_dir(project_dir: &Path) -> String {
    let raw_name = project_dir
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("burr-project");
    let mut name = String::new();
    let mut previous_dash = false;
    for character in raw_name.chars() {
        let normalized = character.to_ascii_lowercase();
        if normalized.is_ascii_alphanumeric() {
            name.push(normalized);
            previous_dash = false;
        } else if !previous_dash && !name.is_empty() {
            name.push('-');
            previous_dash = true;
        }
    }
    while name.ends_with('-') {
        name.pop();
    }
    if name.is_empty() {
        "burr-project".to_string()
    } else {
        name
    }
}

fn starter_pyproject(project_name: &str) -> String {
    format!(
        r#"[project]
name = "{project_name}"
version = "0.1.0"
requires-python = ">=3.11"
dependencies = [
  "build123d>=0.11.0",
  "{BURR_BUILD123D_PYPI_DEPENDENCY}",
]
"#
    )
}

fn starter_design(project_name: &str) -> String {
    format!(
        r#"from pathlib import Path

from build123d import Box, BuildPart, Locations, export_step
from burr_build123d import BurrDesignData, DESIGN_DATA_FILE, m3_clearance_hole


BASE_DIR = Path(__file__).resolve().parent
STEP_FILE = "actuator.step"

housing_length = 86.0
housing_width = 48.0
housing_height = 40.0
m3_hole_y = 12.0
m3_hole_z = 12.0
m3_diameter = 3.4

design = BurrDesignData(
    artifact_id="{project_name}",
    artifact_type="actuator_mount",
    process={{"kind": "FDM", "material": "PETG", "nozzle_mm": 0.4}},
)
design.source("design.py")
design.artifact(STEP_FILE)
design.part(
    "housing",
    bbox_min=(-housing_length / 2.0, -housing_width / 2.0, 0),
    bbox_max=(housing_length / 2.0, housing_width / 2.0, housing_height),
)

with BuildPart() as housing:
    with Locations((0, 0, housing_height / 2.0)):
        Box(housing_length, housing_width, housing_height)

    m3_clearance_hole(
        design,
        feature_id="m3_lower_left",
        part="housing",
        center=(housing_length / 2.0 - 3.0, -m3_hole_y, m3_hole_z),
        axis=(1, 0, 0),
        role="loaded_mount",
        diameter_mm=m3_diameter,
        cut_depth_mm=8.0,
    )

export_step(housing.part, BASE_DIR / STEP_FILE)
design.write(BASE_DIR / DESIGN_DATA_FILE)

print(f"wrote {{BASE_DIR / STEP_FILE}}")
print(f"wrote {{BASE_DIR / DESIGN_DATA_FILE}}")
"#
    )
}

fn starter_gitignore() -> String {
    ".venv/\n__pycache__/\n*.pyc\nactuator.step\nburr-design-data.json\nburr-receipt.json\n"
        .to_string()
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

fn value_at_path<'a>(value: &'a Value, path: &str) -> Option<&'a Value> {
    if path.is_empty() {
        return None;
    }
    let mut current = value;
    for segment in path.split('.') {
        if segment.is_empty() {
            return None;
        }
        current = current.get(segment)?;
    }
    Some(current)
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
        assert!(bad.receipt["checks"]
            .as_array()
            .unwrap()
            .iter()
            .any(|check| {
                string_field(check, "rule_id")
                    == Some("actuator_mount:m3_clearance_hole_wall_thickness")
                    && string_field(check, "reason") == Some("ok")
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
            Some("0.13.0")
        );
    }

    #[test]
    fn init_project_writes_starter_files_without_overwrite() {
        let temp = tempfile::tempdir().unwrap();
        let project = temp.path().join("My Starter Part");

        let written = init_project(&project).unwrap();
        assert_eq!(written.len(), 3);
        assert!(project.join("pyproject.toml").exists());
        assert!(project.join("design.py").exists());
        assert!(project.join(".gitignore").exists());

        let pyproject = fs::read_to_string(project.join("pyproject.toml")).unwrap();
        assert!(pyproject.contains("name = \"my-starter-part\""));
        assert!(pyproject.contains(BURR_BUILD123D_PYPI_DEPENDENCY));
        assert!(!pyproject.contains("git+https://"));

        let design = fs::read_to_string(project.join("design.py")).unwrap();
        assert!(design.contains("artifact_id=\"my-starter-part\""));
        assert!(design.contains("m3_clearance_hole"));

        let error = init_project(&project).unwrap_err();
        assert!(error.contains("Refusing to overwrite existing file"));
    }

    #[test]
    fn init_project_refuses_file_target() {
        let temp = tempfile::tempdir().unwrap();
        let target = temp.path().join("not-a-dir");
        fs::write(&target, "").unwrap();

        let error = init_project(&target).unwrap_err();
        assert!(error.contains("exists and is not a directory"));
    }

    #[test]
    fn init_project_normalizes_project_names() {
        let temp = tempfile::tempdir().unwrap();
        let project = temp.path().join("My_Starter Part!!");

        init_project(&project).unwrap();
        let pyproject = fs::read_to_string(project.join("pyproject.toml")).unwrap();
        let design = fs::read_to_string(project.join("design.py")).unwrap();

        assert!(pyproject.contains("name = \"my-starter-part\""));
        assert!(design.contains("artifact_id=\"my-starter-part\""));
    }

    #[test]
    fn manifest_declared_rulepack_is_used_when_cli_rulepack_is_absent() {
        let temp = tempfile::tempdir().unwrap();
        let source_path = temp.path().join("source.py");
        let step_path = temp.path().join("part.step");
        let rulepack_path = temp.path().join("local.rulepack.json");
        let manifest_path = temp.path().join(DESIGN_DATA_FILE_NAME);
        fs::write(&source_path, "print('source')\n").unwrap();
        fs::write(&step_path, "ISO-10303-21;\nEND-ISO-10303-21;\n").unwrap();

        let manifest = json!({
            "schema_version": "burr.design-data.v1",
            "artifact_id": "local-rulepack-part",
            "artifact_version": "0.1.0",
            "artifact_type": "unit_rulepack_part",
            "units": "mm",
            "rulepack": { "path": "local.rulepack.json" },
            "source": {
                "path": "source.py",
                "sha256": sha256_file(&source_path).unwrap()
            },
            "artifacts": [
                {
                    "kind": "step",
                    "path": "part.step",
                    "sha256": sha256_file(&step_path).unwrap()
                }
            ],
            "measurements": {
                "clearance_mm": 0.25
            }
        });
        let rulepack = json!({
            "schema_version": "burr.rulepack.v1",
            "id": "local_rulepack",
            "version": "0.1.0",
            "artifact_type": "unit_rulepack_part",
            "rules": [
                {
                    "id": "clearance_window",
                    "kind": "numeric_range",
                    "path": "measurements.clearance_mm",
                    "min": 0.2,
                    "max": 0.35
                }
            ]
        });
        write_json_file(&manifest_path, &manifest).unwrap();
        write_json_file(&rulepack_path, &rulepack).unwrap();

        let result = lint_design_data_file(
            &manifest_path,
            &LintOptions {
                cwd: temp.path().to_path_buf(),
                write_receipt: false,
                rulepack_path: None,
            },
        )
        .unwrap();

        assert_eq!(string_field(&result.receipt, "status"), Some("pass"));
        assert_eq!(
            string_field(&result.receipt, "rulepack_id"),
            Some("local_rulepack")
        );
    }

    #[test]
    fn malformed_manifest_rulepack_is_an_error_not_default_fallback() {
        let temp = tempfile::tempdir().unwrap();
        let source_path = temp.path().join("source.py");
        let step_path = temp.path().join("part.step");
        let manifest_path = temp.path().join(DESIGN_DATA_FILE_NAME);
        fs::write(&source_path, "print('source')\n").unwrap();
        fs::write(&step_path, "ISO-10303-21;\nEND-ISO-10303-21;\n").unwrap();

        let mut manifest = json!({
            "schema_version": "burr.design-data.v1",
            "artifact_id": "bad-rulepack-ref",
            "artifact_version": "0.1.0",
            "artifact_type": "captured_slider",
            "units": "mm",
            "rulepack": {},
            "source": {
                "path": "source.py",
                "sha256": sha256_file(&source_path).unwrap()
            },
            "artifacts": [
                {
                    "kind": "step",
                    "path": "part.step",
                    "sha256": sha256_file(&step_path).unwrap()
                }
            ]
        });
        write_json_file(&manifest_path, &manifest).unwrap();

        let options = LintOptions {
            cwd: temp.path().to_path_buf(),
            write_receipt: false,
            rulepack_path: None,
        };
        let error = lint_design_data_file(&manifest_path, &options).unwrap_err();
        assert!(error.contains("Manifest rulepack object must include path"));

        manifest["rulepack"] = json!({ "path": "" });
        write_json_file(&manifest_path, &manifest).unwrap();
        let error = lint_design_data_file(&manifest_path, &options).unwrap_err();
        assert!(error.contains("Manifest rulepack path must be a non-empty string"));
    }

    #[test]
    fn cli_rulepack_overrides_manifest_declared_rulepack() {
        let temp = tempfile::tempdir().unwrap();
        let source_path = temp.path().join("source.py");
        let step_path = temp.path().join("part.step");
        let manifest_path = temp.path().join(DESIGN_DATA_FILE_NAME);
        let manifest_rulepack_path = temp.path().join("manifest.rulepack.json");
        let cli_rulepack_path = temp.path().join("cli.rulepack.json");
        fs::write(&source_path, "print('source')\n").unwrap();
        fs::write(&step_path, "ISO-10303-21;\nEND-ISO-10303-21;\n").unwrap();

        write_json_file(
            &manifest_rulepack_path,
            &json!({
                "schema_version": "burr.rulepack.v1",
                "id": "manifest_rulepack",
                "version": "0.1.0",
                "artifact_type": "unit_rulepack_part",
                "rules": []
            }),
        )
        .unwrap();
        write_json_file(
            &cli_rulepack_path,
            &json!({
                "schema_version": "burr.rulepack.v1",
                "id": "cli_rulepack",
                "version": "0.1.0",
                "artifact_type": "unit_rulepack_part",
                "rules": []
            }),
        )
        .unwrap();
        write_json_file(
            &manifest_path,
            &json!({
                "schema_version": "burr.design-data.v1",
                "artifact_id": "override-rulepack-part",
                "artifact_version": "0.1.0",
                "artifact_type": "unit_rulepack_part",
                "units": "mm",
                "rulepack": { "path": "manifest.rulepack.json" },
                "source": {
                    "path": "source.py",
                    "sha256": sha256_file(&source_path).unwrap()
                },
                "artifacts": [
                    {
                        "kind": "step",
                        "path": "part.step",
                        "sha256": sha256_file(&step_path).unwrap()
                    }
                ]
            }),
        )
        .unwrap();

        let result = lint_design_data_file(
            &manifest_path,
            &LintOptions {
                cwd: temp.path().to_path_buf(),
                write_receipt: false,
                rulepack_path: Some(cli_rulepack_path),
            },
        )
        .unwrap();
        assert_eq!(
            string_field(&result.receipt, "rulepack_id"),
            Some("cli_rulepack")
        );
    }

    #[test]
    fn feature_count_and_numeric_range_check_manifest_level_claims() {
        let temp = tempfile::tempdir().unwrap();
        let source_path = temp.path().join("source.py");
        let step_path = temp.path().join("part.step");
        fs::write(&source_path, "print('source')\n").unwrap();
        fs::write(&step_path, "ISO-10303-21;\nEND-ISO-10303-21;\n").unwrap();

        let manifest = json!({
            "schema_version": "burr.design-data.v1",
            "artifact_id": "count-range-part",
            "artifact_version": "0.1.0",
            "artifact_type": "breadth_fixture",
            "units": "mm",
            "source": {
                "path": "source.py",
                "sha256": sha256_file(&source_path).unwrap()
            },
            "artifacts": [
                {
                    "kind": "step",
                    "path": "part.step",
                    "sha256": sha256_file(&step_path).unwrap()
                }
            ],
            "measurements": {
                "clearance_mm": 0.25
            },
            "features": [
                { "id": "relief_a", "kind": "clearance_hole", "intent": "cosmetic", "role": "visual_lightening" },
                { "id": "relief_b", "kind": "clearance_hole", "intent": "cosmetic", "role": "visual_lightening" },
                { "id": "relief_c", "kind": "clearance_hole", "intent": "cosmetic", "role": "visual_lightening" }
            ]
        });
        let rulepack = json!({
            "schema_version": "burr.rulepack.v1",
            "id": "breadth_fixture",
            "version": "0.1.0",
            "artifact_type": "breadth_fixture",
            "rules": [
                {
                    "id": "cosmetic_relief_inventory",
                    "kind": "feature_count",
                    "applies_to": {
                        "kind": "clearance_hole",
                        "intent_any": ["cosmetic"],
                        "role_any": ["visual_lightening"]
                    },
                    "min_count": 3,
                    "max_count": 12
                },
                {
                    "id": "clearance_window",
                    "kind": "numeric_range",
                    "path": "measurements.clearance_mm",
                    "min": 0.2,
                    "max": 0.35
                }
            ]
        });

        let receipt = lint_design_data(&manifest, &rulepack, temp.path(), None);
        assert_eq!(string_field(&receipt, "status"), Some("pass"));
        assert_eq!(
            receipt
                .pointer("/summary/features/checked_feature_ids")
                .and_then(Value::as_array)
                .unwrap()
                .len(),
            3
        );
        assert!(receipt["checks"].as_array().unwrap().iter().any(|check| {
            string_field(check, "rule_id") == Some("breadth_fixture:clearance_window")
                && check.pointer("/measured/value").and_then(Value::as_f64) == Some(0.25)
        }));

        let mut failing_manifest = manifest.clone();
        failing_manifest["measurements"]["clearance_mm"] = json!(0.5);
        failing_manifest["features"] = json!([
            { "id": "relief_a", "kind": "clearance_hole", "intent": "cosmetic", "role": "visual_lightening" },
            { "id": "relief_b", "kind": "clearance_hole", "intent": "cosmetic", "role": "visual_lightening" }
        ]);
        let failing = lint_design_data(&failing_manifest, &rulepack, temp.path(), None);
        assert_eq!(string_field(&failing, "status"), Some("fail"));
        assert!(failing["checks"]
            .as_array()
            .unwrap()
            .iter()
            .any(|check| { string_field(check, "reason") == Some("feature_count_out_of_range") }));
        assert!(failing["checks"]
            .as_array()
            .unwrap()
            .iter()
            .any(|check| { string_field(check, "reason") == Some("numeric_value_out_of_range") }));
    }

    #[test]
    fn feature_pair_spacing_checks_declared_ligaments() {
        let temp = tempfile::tempdir().unwrap();
        let source_path = temp.path().join("source.py");
        let step_path = temp.path().join("part.step");
        fs::write(&source_path, "print('source')\n").unwrap();
        fs::write(&step_path, "ISO-10303-21;\nEND-ISO-10303-21;\n").unwrap();

        let manifest = json!({
            "schema_version": "burr.design-data.v1",
            "artifact_id": "dense-hole-part",
            "artifact_version": "0.1.0",
            "artifact_type": "printed_plate",
            "units": "mm",
            "source": {
                "path": "source.py",
                "sha256": sha256_file(&source_path).unwrap()
            },
            "artifacts": [
                {
                    "kind": "step",
                    "path": "part.step",
                    "sha256": sha256_file(&step_path).unwrap()
                }
            ],
            "features": [
                {
                    "id": "relief_a",
                    "kind": "clearance_hole",
                    "intent": "cosmetic",
                    "role": "visual_lightening",
                    "center_mm": [0.0, 0.0, 0.0],
                    "diameter_mm": 3.0
                },
                {
                    "id": "relief_b",
                    "kind": "clearance_hole",
                    "intent": "cosmetic",
                    "role": "visual_lightening",
                    "center_mm": [0.0, 3.4, 0.0],
                    "diameter_mm": 3.0
                },
                {
                    "id": "relief_c",
                    "kind": "clearance_hole",
                    "intent": "cosmetic",
                    "role": "visual_lightening",
                    "center_mm": [0.0, 12.0, 0.0],
                    "diameter_mm": 3.0
                },
                {
                    "id": "loaded_mount",
                    "kind": "clearance_hole",
                    "intent": "mechanical_interface",
                    "role": "loaded_mount",
                    "center_mm": [0.0, 3.5, 0.0],
                    "diameter_mm": 3.0
                }
            ]
        });
        let rulepack = json!({
            "schema_version": "burr.rulepack.v1",
            "id": "printed_plate",
            "version": "0.1.0",
            "artifact_type": "printed_plate",
            "rules": [
                {
                    "id": "cosmetic_relief_ligament",
                    "kind": "feature_pair_spacing",
                    "applies_to": {
                        "kind": "clearance_hole",
                        "intent_any": ["cosmetic"],
                        "role_any": ["visual_lightening"]
                    },
                    "min_clearance_mm": 1.2
                }
            ]
        });

        let receipt = lint_design_data(&manifest, &rulepack, temp.path(), None);
        assert_eq!(string_field(&receipt, "status"), Some("fail"));
        let check = receipt["checks"]
            .as_array()
            .unwrap()
            .iter()
            .find(|check| {
                string_field(check, "rule_id") == Some("printed_plate:cosmetic_relief_ligament")
            })
            .unwrap();
        assert_eq!(
            string_field(check, "reason"),
            Some("insufficient_feature_pair_spacing")
        );
        assert_eq!(
            check
                .pointer("/measured/closest_pair/clearance_mm")
                .and_then(Value::as_f64),
            Some(0.4)
        );
        assert_eq!(
            check
                .pointer("/required/min_clearance_mm")
                .and_then(Value::as_f64),
            Some(1.2)
        );
        assert_eq!(
            check
                .pointer("/measured/pair_count")
                .and_then(Value::as_u64),
            Some(3)
        );

        let mut passing_manifest = manifest.clone();
        passing_manifest["features"][1]["center_mm"] = json!([0.0, 5.0, 0.0]);
        let passing = lint_design_data(&passing_manifest, &rulepack, temp.path(), None);
        assert_eq!(string_field(&passing, "status"), Some("pass"));
    }

    #[test]
    fn feature_pair_spacing_checks_slot_capsules() {
        let temp = tempfile::tempdir().unwrap();
        let source_path = temp.path().join("source.py");
        let step_path = temp.path().join("part.step");
        fs::write(&source_path, "print('source')\n").unwrap();
        fs::write(&step_path, "ISO-10303-21;\nEND-ISO-10303-21;\n").unwrap();

        let manifest = json!({
            "schema_version": "burr.design-data.v1",
            "artifact_id": "slot-ligament-part",
            "artifact_version": "0.1.0",
            "artifact_type": "printed_plate",
            "units": "mm",
            "source": {
                "path": "source.py",
                "sha256": sha256_file(&source_path).unwrap()
            },
            "artifacts": [
                {
                    "kind": "step",
                    "path": "part.step",
                    "sha256": sha256_file(&step_path).unwrap()
                }
            ],
            "features": [
                {
                    "id": "relief_slot",
                    "kind": "straight_slot",
                    "intent": "cosmetic",
                    "role": "relief_slot",
                    "center_mm": [0.0, 0.0, 0.0],
                    "width_mm": 2.0,
                    "length_mm": 10.0,
                    "span_axis": [0.0, 0.0, 1.0]
                },
                {
                    "id": "relief_hole",
                    "kind": "clearance_hole",
                    "intent": "cosmetic",
                    "role": "visual_lightening",
                    "center_mm": [0.0, 0.0, 5.8],
                    "diameter_mm": 3.0
                }
            ]
        });
        let rulepack = json!({
            "schema_version": "burr.rulepack.v1",
            "id": "printed_plate",
            "version": "0.1.0",
            "artifact_type": "printed_plate",
            "rules": [
                {
                    "id": "relief_ligament",
                    "kind": "feature_pair_spacing",
                    "applies_to": {
                        "kind_any": ["clearance_hole", "straight_slot"],
                        "intent_any": ["cosmetic"],
                        "role_any": ["visual_lightening", "relief_slot"]
                    },
                    "min_clearance_mm": 1.2
                }
            ]
        });

        let receipt = lint_design_data(&manifest, &rulepack, temp.path(), None);
        assert_eq!(string_field(&receipt, "status"), Some("fail"));
        let check = receipt["checks"]
            .as_array()
            .unwrap()
            .iter()
            .find(|check| string_field(check, "rule_id") == Some("printed_plate:relief_ligament"))
            .unwrap();
        assert_eq!(
            string_field(check, "reason"),
            Some("insufficient_feature_pair_spacing")
        );
        assert_eq!(
            check
                .pointer("/measured/closest_pair/shape_distance_mm")
                .and_then(Value::as_f64),
            Some(1.8)
        );
        assert_eq!(
            check
                .pointer("/measured/closest_pair/clearance_mm")
                .and_then(Value::as_f64),
            Some(-0.7)
        );

        let mut passing_manifest = manifest.clone();
        passing_manifest["features"][1]["center_mm"] = json!([0.0, 0.0, 8.0]);
        let passing = lint_design_data(&passing_manifest, &rulepack, temp.path(), None);
        assert_eq!(string_field(&passing, "status"), Some("pass"));
    }

    #[test]
    fn feature_edge_distance_checks_slot_envelopes() {
        let temp = tempfile::tempdir().unwrap();
        let source_path = temp.path().join("source.py");
        let step_path = temp.path().join("part.step");
        fs::write(&source_path, "print('source')\n").unwrap();
        fs::write(&step_path, "ISO-10303-21;\nEND-ISO-10303-21;\n").unwrap();

        let manifest = json!({
            "schema_version": "burr.design-data.v1",
            "artifact_id": "slot-edge-part",
            "artifact_version": "0.1.0",
            "artifact_type": "actuator_mount",
            "units": "mm",
            "source": {
                "path": "source.py",
                "sha256": sha256_file(&source_path).unwrap()
            },
            "artifacts": [
                {
                    "kind": "step",
                    "path": "part.step",
                    "sha256": sha256_file(&step_path).unwrap()
                }
            ],
            "parts": [
                {
                    "id": "housing",
                    "bbox_mm": {
                        "min": [-35.0, -21.0, -10.0],
                        "max": [35.0, 21.0, 10.0]
                    }
                }
            ],
            "features": [
                {
                    "id": "motor_adjust_slot",
                    "part": "housing",
                    "kind": "straight_slot",
                    "intent": "mechanical_interface",
                    "role": "loaded_mount",
                    "center_mm": [0.0, -10.0, 0.0],
                    "axis": [1.0, 0.0, 0.0],
                    "span_axis": [0.0, 1.0, 0.0],
                    "width_mm": 4.0,
                    "length_mm": 18.0
                }
            ]
        });
        let rulepack = json!({
            "schema_version": "burr.rulepack.v1",
            "id": "actuator_mount",
            "version": "0.13.0",
            "artifact_type": "actuator_mount",
            "rules": [
                {
                    "id": "mechanical_slot_edge_distance",
                    "kind": "feature_edge_distance",
                    "applies_to": {
                        "kind": "straight_slot",
                        "intent_any": ["mechanical_interface"],
                        "role_any": ["loaded_mount"]
                    },
                    "min_wall_to_edge_mm": 3.0
                }
            ]
        });

        let receipt = lint_design_data(&manifest, &rulepack, temp.path(), None);
        assert_eq!(string_field(&receipt, "status"), Some("fail"));
        let check = receipt["checks"]
            .as_array()
            .unwrap()
            .iter()
            .find(|check| {
                string_field(check, "rule_id")
                    == Some("actuator_mount:mechanical_slot_edge_distance")
            })
            .unwrap();
        assert_eq!(
            string_field(check, "reason"),
            Some("insufficient_feature_edge_distance")
        );
        assert_eq!(
            check
                .pointer("/measured/wall_to_edge_mm")
                .and_then(Value::as_f64),
            Some(2.0)
        );
        assert_eq!(
            check
                .pointer("/required/min_wall_to_edge_mm")
                .and_then(Value::as_f64),
            Some(3.0)
        );
        assert_eq!(number_field(check, "margin_mm"), Some(-1.0));

        let mut passing_manifest = manifest.clone();
        passing_manifest["features"][0]["center_mm"] = json!([0.0, -7.0, 0.0]);
        let passing = lint_design_data(&passing_manifest, &rulepack, temp.path(), None);
        assert_eq!(string_field(&passing, "status"), Some("pass"));
    }

    #[test]
    fn count_and_range_rules_require_bounds() {
        let temp = tempfile::tempdir().unwrap();
        let source_path = temp.path().join("source.py");
        let step_path = temp.path().join("part.step");
        fs::write(&source_path, "print('source')\n").unwrap();
        fs::write(&step_path, "ISO-10303-21;\nEND-ISO-10303-21;\n").unwrap();

        let manifest = json!({
            "schema_version": "burr.design-data.v1",
            "artifact_id": "bad-rule-bounds",
            "artifact_version": "0.1.0",
            "artifact_type": "breadth_fixture",
            "units": "mm",
            "source": {
                "path": "source.py",
                "sha256": sha256_file(&source_path).unwrap()
            },
            "artifacts": [
                {
                    "kind": "step",
                    "path": "part.step",
                    "sha256": sha256_file(&step_path).unwrap()
                }
            ],
            "measurements": {
                "clearance_mm": 0.25
            },
            "features": [
                { "id": "relief_a", "kind": "clearance_hole", "intent": "cosmetic", "role": "visual_lightening" }
            ]
        });
        let rulepack = json!({
            "schema_version": "burr.rulepack.v1",
            "id": "breadth_fixture",
            "version": "0.1.0",
            "artifact_type": "breadth_fixture",
            "rules": [
                {
                    "id": "unbounded_feature_count",
                    "kind": "feature_count",
                    "applies_to": { "kind": "clearance_hole" }
                },
                {
                    "id": "unbounded_numeric_range",
                    "kind": "numeric_range",
                    "path": "measurements.clearance_mm"
                }
            ]
        });

        let receipt = lint_design_data(&manifest, &rulepack, temp.path(), None);
        assert_eq!(string_field(&receipt, "status"), Some("fail"));
        assert!(receipt["checks"].as_array().unwrap().iter().any(|check| {
            string_field(check, "reason") == Some("invalid_feature_count_rule_bounds")
        }));
        assert!(receipt["checks"].as_array().unwrap().iter().any(|check| {
            string_field(check, "reason") == Some("invalid_numeric_range_rule_bounds")
        }));
    }

    #[test]
    fn fastener_support_wall_thickness_checks_declared_boss_meat() {
        let temp = tempfile::tempdir().unwrap();
        let source_path = temp.path().join("source.py");
        let step_path = temp.path().join("part.step");
        fs::write(&source_path, "print('source')\n").unwrap();
        fs::write(&step_path, "ISO-10303-21;\nEND-ISO-10303-21;\n").unwrap();

        let manifest = json!({
            "schema_version": "burr.design-data.v1",
            "artifact_id": "boss-meat-fixture",
            "artifact_version": "0.1.0",
            "artifact_type": "boss_meat_fixture",
            "units": "mm",
            "source": {
                "path": "source.py",
                "sha256": sha256_file(&source_path).unwrap()
            },
            "artifacts": [
                {
                    "kind": "step",
                    "path": "part.step",
                    "sha256": sha256_file(&step_path).unwrap()
                }
            ],
            "features": [
                {
                    "id": "m3_bad_boss",
                    "kind": "clearance_hole",
                    "intent": "mechanical_interface",
                    "role": "bossed_mount",
                    "fastener": "M3",
                    "diameter_mm": 3.4,
                    "support_diameter_mm": 6.0
                },
                {
                    "id": "m3_good_boss",
                    "kind": "clearance_hole",
                    "intent": "mechanical_interface",
                    "role": "bossed_mount",
                    "fastener": "M3",
                    "diameter_mm": 3.4,
                    "support_diameter_mm": 8.0
                },
                {
                    "id": "m3_bad_insert_boss",
                    "kind": "heat_set_insert_pocket",
                    "intent": "mechanical_interface",
                    "role": "bossed_insert",
                    "insert": "M3x5.7",
                    "pocket_diameter_mm": 4.6,
                    "support_diameter_mm": 8.0
                },
                {
                    "id": "m3_good_insert_boss",
                    "kind": "heat_set_insert_pocket",
                    "intent": "mechanical_interface",
                    "role": "bossed_insert",
                    "insert": "M3x5.7",
                    "pocket_diameter_mm": 4.6,
                    "support_diameter_mm": 9.0
                }
            ]
        });
        let rulepack = json!({
            "schema_version": "burr.rulepack.v1",
            "id": "boss_meat_fixture",
            "version": "0.1.0",
            "artifact_type": "boss_meat_fixture",
            "rules": [
                {
                    "id": "support_wall",
                    "kind": "fastener_support_wall_thickness",
                    "applies_to": {
                        "kind": "clearance_hole",
                        "intent_any": ["mechanical_interface"],
                        "role_any": ["bossed_mount"]
                    },
                    "min_wall_thickness_mm": 2.0
                },
                {
                    "id": "insert_support_wall",
                    "kind": "fastener_support_wall_thickness",
                    "applies_to": {
                        "kind": "heat_set_insert_pocket",
                        "intent_any": ["mechanical_interface"],
                        "role_any": ["bossed_insert"]
                    },
                    "min_wall_thickness_mm": 2.0
                }
            ]
        });

        let receipt = lint_design_data(&manifest, &rulepack, temp.path(), None);
        assert_eq!(string_field(&receipt, "status"), Some("fail"));
        assert!(receipt["checks"].as_array().unwrap().iter().any(|check| {
            string_field(check, "feature_id") == Some("m3_bad_boss")
                && string_field(check, "reason") == Some("insufficient_fastener_support_wall")
                && check
                    .pointer("/measured/support_wall_thickness_mm")
                    .and_then(Value::as_f64)
                    == Some(1.3)
                && check.pointer("/margin_mm").and_then(Value::as_f64) == Some(-0.7)
        }));
        assert!(receipt["checks"].as_array().unwrap().iter().any(|check| {
            string_field(check, "feature_id") == Some("m3_good_boss")
                && string_field(check, "reason") == Some("ok")
                && check
                    .pointer("/measured/support_wall_thickness_mm")
                    .and_then(Value::as_f64)
                    == Some(2.3)
        }));
        assert!(receipt["checks"].as_array().unwrap().iter().any(|check| {
            string_field(check, "feature_id") == Some("m3_bad_insert_boss")
                && string_field(check, "reason") == Some("insufficient_fastener_support_wall")
                && check
                    .pointer("/measured/support_wall_thickness_mm")
                    .and_then(Value::as_f64)
                    == Some(1.7)
        }));
        assert!(receipt["checks"].as_array().unwrap().iter().any(|check| {
            string_field(check, "feature_id") == Some("m3_good_insert_boss")
                && string_field(check, "reason") == Some("ok")
                && check
                    .pointer("/measured/support_wall_thickness_mm")
                    .and_then(Value::as_f64)
                    == Some(2.2)
        }));
        assert!(format_receipt_diagnostics(&receipt)
            .iter()
            .flatten()
            .any(|line| line.contains("Measured support wall: 1.3 mm")));
    }

    #[test]
    fn blind_pocket_back_wall_thickness_checks_remaining_host_material() {
        let temp = tempfile::tempdir().unwrap();
        let source_path = temp.path().join("source.py");
        let step_path = temp.path().join("part.step");
        fs::write(&source_path, "print('source')\n").unwrap();
        fs::write(&step_path, "ISO-10303-21;\nEND-ISO-10303-21;\n").unwrap();

        let manifest = json!({
            "schema_version": "burr.design-data.v1",
            "artifact_id": "insert-pocket-back-wall-fixture",
            "artifact_version": "0.1.0",
            "artifact_type": "insert_pocket_back_wall_fixture",
            "units": "mm",
            "source": {
                "path": "source.py",
                "sha256": sha256_file(&source_path).unwrap()
            },
            "artifacts": [
                {
                    "kind": "step",
                    "path": "part.step",
                    "sha256": sha256_file(&step_path).unwrap()
                }
            ],
            "parts": [
                {
                    "id": "good_housing",
                    "bbox_mm": {
                        "min": [-4.35, -12.0, -8.0],
                        "max": [4.35, 12.0, 8.0]
                    }
                },
                {
                    "id": "bad_housing",
                    "bbox_mm": {
                        "min": [-3.35, -12.0, -8.0],
                        "max": [3.35, 12.0, 8.0]
                    }
                }
            ],
            "features": [
                {
                    "id": "good_insert_pocket",
                    "part": "good_housing",
                    "kind": "heat_set_insert_pocket",
                    "intent": "mechanical_interface",
                    "role": "threaded_mount",
                    "insert": "M3x5.7",
                    "pocket_center_mm": [-1.5, 0.0, 0.0],
                    "bottom_center_mm": [1.35, 0.0, 0.0],
                    "axis": [1.0, 0.0, 0.0]
                },
                {
                    "id": "bad_insert_pocket",
                    "part": "bad_housing",
                    "kind": "heat_set_insert_pocket",
                    "intent": "mechanical_interface",
                    "role": "threaded_mount",
                    "insert": "M3x5.7",
                    "pocket_center_mm": [-0.5, 0.0, 0.0],
                    "bottom_center_mm": [2.35, 0.0, 0.0],
                    "axis": [1.0, 0.0, 0.0]
                }
            ]
        });
        let rulepack = json!({
            "schema_version": "burr.rulepack.v1",
            "id": "insert_pocket_back_wall_fixture",
            "version": "0.1.0",
            "artifact_type": "insert_pocket_back_wall_fixture",
            "rules": [
                {
                    "id": "insert_back_wall",
                    "kind": "blind_pocket_back_wall_thickness",
                    "applies_to": {
                        "kind": "heat_set_insert_pocket",
                        "intent_any": ["mechanical_interface"],
                        "role_any": ["threaded_mount"]
                    },
                    "min_back_wall_thickness_mm": 2.0
                }
            ]
        });

        let receipt = lint_design_data(&manifest, &rulepack, temp.path(), None);
        assert_eq!(string_field(&receipt, "status"), Some("fail"));
        assert!(receipt["checks"].as_array().unwrap().iter().any(|check| {
            string_field(check, "feature_id") == Some("good_insert_pocket")
                && string_field(check, "reason") == Some("ok")
                && check
                    .pointer("/measured/back_wall_thickness_mm")
                    .and_then(Value::as_f64)
                    == Some(3.0)
                && check.pointer("/margin_mm").and_then(Value::as_f64) == Some(1.0)
        }));
        assert!(receipt["checks"].as_array().unwrap().iter().any(|check| {
            string_field(check, "feature_id") == Some("bad_insert_pocket")
                && string_field(check, "reason") == Some("insufficient_blind_pocket_back_wall")
                && check
                    .pointer("/measured/back_wall_thickness_mm")
                    .and_then(Value::as_f64)
                    == Some(1.0)
                && check.pointer("/margin_mm").and_then(Value::as_f64) == Some(-1.0)
        }));
        assert!(format_receipt_diagnostics(&receipt)
            .iter()
            .flatten()
            .any(|line| line.contains("Measured back wall: 1 mm")));
    }

    #[test]
    fn standoff_boss_support_link_requires_aligned_supported_feature() {
        let temp = tempfile::tempdir().unwrap();
        let source_path = temp.path().join("source.py");
        let step_path = temp.path().join("part.step");
        fs::write(&source_path, "print('source')\n").unwrap();
        fs::write(&step_path, "ISO-10303-21;\nEND-ISO-10303-21;\n").unwrap();

        let manifest = json!({
            "schema_version": "burr.design-data.v1",
            "artifact_id": "support-link-fixture",
            "artifact_version": "0.1.0",
            "artifact_type": "support_link_fixture",
            "units": "mm",
            "source": {
                "path": "source.py",
                "sha256": sha256_file(&source_path).unwrap()
            },
            "artifacts": [
                {
                    "kind": "step",
                    "path": "part.step",
                    "sha256": sha256_file(&step_path).unwrap()
                }
            ],
            "features": [
                {
                    "id": "m3_good_boss",
                    "kind": "standoff_boss",
                    "intent": "mechanical_interface",
                    "fastener": "M3",
                    "boss_diameter_mm": 8.0,
                    "boss_height_mm": 5.0,
                    "center_mm": [0.0, 0.0, 6.5],
                    "boss_center_mm": [0.0, 0.0, 6.5],
                    "top_center_mm": [0.0, 0.0, 9.0],
                    "axis": [0.0, 0.0, 1.0],
                    "role": "pcb_standoff",
                    "supports_feature_id": "m3_supported_mount"
                },
                {
                    "id": "m3_bad_boss",
                    "kind": "standoff_boss",
                    "intent": "mechanical_interface",
                    "fastener": "M3",
                    "boss_diameter_mm": 8.0,
                    "boss_height_mm": 5.0,
                    "center_mm": [0.8, 0.0, 6.5],
                    "boss_center_mm": [0.8, 0.0, 6.5],
                    "top_center_mm": [0.8, 0.0, 9.0],
                    "axis": [0.0, 0.0, 1.0],
                    "role": "pcb_standoff",
                    "supports_feature_id": "m3_supported_mount"
                },
                {
                    "id": "m3_supported_mount",
                    "kind": "clearance_hole",
                    "intent": "reference",
                    "fastener": "M3",
                    "diameter_mm": 3.4,
                    "support_diameter_mm": 8.0,
                    "center_mm": [0.0, 0.0, 6.5],
                    "axis": [0.0, 0.0, 1.0],
                    "role": "bossed_mount"
                }
            ]
        });
        let rulepack = json!({
            "schema_version": "burr.rulepack.v1",
            "id": "support_link_fixture",
            "version": "0.1.0",
            "artifact_type": "support_link_fixture",
            "rules": [
                {
                    "id": "standoff_support_link",
                    "kind": "standoff_boss_support_link",
                    "applies_to": {
                        "kind": "standoff_boss",
                        "intent_any": ["mechanical_interface"]
                    },
                    "centerline_tolerance_mm": 0.25,
                    "support_diameter_tolerance_mm": 0.05,
                    "axis_dot_min": 0.99
                }
            ]
        });

        let receipt = lint_design_data(&manifest, &rulepack, temp.path(), None);
        assert_eq!(string_field(&receipt, "status"), Some("fail"));
        assert!(receipt["checks"].as_array().unwrap().iter().any(|check| {
            string_field(check, "feature_id") == Some("m3_good_boss")
                && string_field(check, "reason") == Some("ok")
                && string_field(check, "related_feature_id") == Some("m3_supported_mount")
        }));
        assert!(receipt["checks"].as_array().unwrap().iter().any(|check| {
            string_field(check, "feature_id") == Some("m3_bad_boss")
                && string_field(check, "reason") == Some("standoff_boss_support_mismatch")
                && string_field(check, "related_feature_id") == Some("m3_supported_mount")
                && check
                    .pointer("/measured/centerline_distance_mm")
                    .and_then(Value::as_f64)
                    == Some(0.8)
                && check.pointer("/repair/action").and_then(Value::as_str)
                    == Some("align_standoff_boss_to_supported_feature")
        }));
        assert!(format_receipt_diagnostics(&receipt)
            .iter()
            .flatten()
            .any(|line| line.contains("Measured centerline offset: 0.8 mm")));
    }

    #[test]
    fn feature_presence_accepts_reversed_step_axis() {
        let temp = tempfile::tempdir().unwrap();
        let step_path = temp.path().join("part.step");
        write_step_cylinder(&step_path, (0.0, 0.0, 8.0), (-1.0, 0.0, 0.0), 1.7);
        let source_path = temp.path().join("source.py");
        fs::write(&source_path, "print('source')\n").unwrap();

        let manifest = test_manifest(
            sha256_file(&source_path).unwrap(),
            sha256_file(&step_path).unwrap(),
        );
        let receipt = lint_design_data(&manifest, &default_rulepack().unwrap(), temp.path(), None);

        assert_eq!(string_field(&receipt, "status"), Some("pass"));
        assert!(receipt["checks"].as_array().unwrap().iter().any(|check| {
            string_field(check, "rule_id") == Some("actuator_mount:m3_clearance_hole_step_presence")
                && string_field(check, "reason") == Some("ok")
                && check.pointer("/measured/axis_dot").and_then(Value::as_f64) == Some(1.0)
        }));
    }

    #[test]
    fn feature_presence_rejects_present_but_wrong_cylinder() {
        let temp = tempfile::tempdir().unwrap();
        let step_path = temp.path().join("part.step");
        write_step_cylinder(&step_path, (0.0, 2.0, 8.0), (1.0, 0.0, 0.0), 1.9);
        let source_path = temp.path().join("source.py");
        fs::write(&source_path, "print('source')\n").unwrap();

        let manifest = test_manifest(
            sha256_file(&source_path).unwrap(),
            sha256_file(&step_path).unwrap(),
        );
        let receipt = lint_design_data(&manifest, &default_rulepack().unwrap(), temp.path(), None);

        assert_eq!(string_field(&receipt, "status"), Some("fail"));
        assert!(receipt["checks"].as_array().unwrap().iter().any(|check| {
            string_field(check, "rule_id") == Some("actuator_mount:m3_clearance_hole_step_presence")
                && string_field(check, "reason") == Some("missing_declared_feature")
                && check
                    .pointer("/measured/candidate_cylinders")
                    .and_then(Value::as_u64)
                    == Some(1)
                && check
                    .pointer("/measured/best_centerline_distance_mm")
                    .and_then(Value::as_f64)
                    == Some(2.0)
        }));
    }

    #[test]
    fn straight_slot_presence_requires_endpoint_cylinders_and_side_planes() {
        let temp = tempfile::tempdir().unwrap();
        let step_path = temp.path().join("part.step");
        write_step_surfaces(
            &step_path,
            &[
                ((0.0, -6.0, 8.0), (1.0, 0.0, 0.0), 2.0),
                ((0.0, 6.0, 8.0), (1.0, 0.0, 0.0), 2.0),
            ],
            &[
                ((0.0, 0.0, 6.0), (0.0, 0.0, 1.0)),
                ((0.0, 0.0, 10.0), (0.0, 0.0, 1.0)),
            ],
        );
        let source_path = temp.path().join("source.py");
        fs::write(&source_path, "print('source')\n").unwrap();

        let manifest = slot_manifest(
            sha256_file(&source_path).unwrap(),
            sha256_file(&step_path).unwrap(),
            "mechanical_interface",
        );
        let receipt = lint_design_data(&manifest, &default_rulepack().unwrap(), temp.path(), None);

        assert_eq!(string_field(&receipt, "status"), Some("pass"));
        assert!(receipt["checks"].as_array().unwrap().iter().any(|check| {
            string_field(check, "rule_id") == Some("actuator_mount:straight_slot_step_presence")
                && string_field(check, "reason") == Some("ok")
                && check
                    .pointer("/measured/matched_slot_endpoints")
                    .and_then(Value::as_u64)
                    == Some(2)
                && check
                    .pointer("/measured/matched_slot_side_planes")
                    .and_then(Value::as_u64)
                    == Some(2)
        }));
    }

    #[test]
    fn straight_slot_presence_rejects_two_disconnected_holes() {
        let temp = tempfile::tempdir().unwrap();
        let step_path = temp.path().join("part.step");
        write_step_cylinders(
            &step_path,
            &[
                ((0.0, -6.0, 8.0), (1.0, 0.0, 0.0), 2.0),
                ((0.0, 6.0, 8.0), (1.0, 0.0, 0.0), 2.0),
            ],
        );
        let source_path = temp.path().join("source.py");
        fs::write(&source_path, "print('source')\n").unwrap();

        let manifest = slot_manifest(
            sha256_file(&source_path).unwrap(),
            sha256_file(&step_path).unwrap(),
            "mechanical_interface",
        );
        let receipt = lint_design_data(&manifest, &default_rulepack().unwrap(), temp.path(), None);

        assert_eq!(string_field(&receipt, "status"), Some("fail"));
        assert!(receipt["checks"].as_array().unwrap().iter().any(|check| {
            string_field(check, "rule_id") == Some("actuator_mount:straight_slot_step_presence")
                && string_field(check, "reason") == Some("missing_declared_feature")
                && check
                    .pointer("/measured/matched_slot_endpoints")
                    .and_then(Value::as_u64)
                    == Some(2)
                && check
                    .pointer("/measured/matched_slot_side_planes")
                    .and_then(Value::as_u64)
                    == Some(0)
        }));
    }

    #[test]
    fn counterbore_presence_requires_bore_counterbore_and_shoulder_plane() {
        let temp = tempfile::tempdir().unwrap();
        let step_path = temp.path().join("part.step");
        write_step_surfaces(
            &step_path,
            &[
                ((0.0, 0.0, 8.0), (1.0, 0.0, 0.0), 1.7),
                ((-8.0, 0.0, 8.0), (1.0, 0.0, 0.0), 3.4),
            ],
            &[((-6.0, 0.0, 8.0), (1.0, 0.0, 0.0))],
        );
        let source_path = temp.path().join("source.py");
        fs::write(&source_path, "print('source')\n").unwrap();

        let manifest = counterbore_manifest(
            sha256_file(&source_path).unwrap(),
            sha256_file(&step_path).unwrap(),
            "mechanical_interface",
            6.8,
        );
        let receipt = lint_design_data(&manifest, &default_rulepack().unwrap(), temp.path(), None);

        assert_eq!(string_field(&receipt, "status"), Some("pass"));
        assert!(receipt["checks"].as_array().unwrap().iter().any(|check| {
            string_field(check, "rule_id") == Some("actuator_mount:counterbore_step_presence")
                && string_field(check, "reason") == Some("ok")
                && check
                    .pointer("/measured/matched_bore_cylinder")
                    .and_then(Value::as_bool)
                    == Some(true)
                && check
                    .pointer("/measured/matched_counterbore_cylinder")
                    .and_then(Value::as_bool)
                    == Some(true)
                && check
                    .pointer("/measured/matched_counterbore_shoulder_plane")
                    .and_then(Value::as_bool)
                    == Some(true)
        }));
    }

    #[test]
    fn counterbore_presence_rejects_two_cylinders_without_shoulder() {
        let temp = tempfile::tempdir().unwrap();
        let step_path = temp.path().join("part.step");
        write_step_cylinders(
            &step_path,
            &[
                ((0.0, 0.0, 8.0), (1.0, 0.0, 0.0), 1.7),
                ((-8.0, 0.0, 8.0), (1.0, 0.0, 0.0), 3.4),
            ],
        );
        let source_path = temp.path().join("source.py");
        fs::write(&source_path, "print('source')\n").unwrap();

        let manifest = counterbore_manifest(
            sha256_file(&source_path).unwrap(),
            sha256_file(&step_path).unwrap(),
            "mechanical_interface",
            6.8,
        );
        let receipt = lint_design_data(&manifest, &default_rulepack().unwrap(), temp.path(), None);

        assert_eq!(string_field(&receipt, "status"), Some("fail"));
        assert!(receipt["checks"].as_array().unwrap().iter().any(|check| {
            string_field(check, "rule_id") == Some("actuator_mount:counterbore_step_presence")
                && string_field(check, "reason") == Some("missing_declared_feature")
                && check
                    .pointer("/measured/matched_bore_cylinder")
                    .and_then(Value::as_bool)
                    == Some(true)
                && check
                    .pointer("/measured/matched_counterbore_cylinder")
                    .and_then(Value::as_bool)
                    == Some(true)
                && check
                    .pointer("/measured/matched_counterbore_shoulder_plane")
                    .and_then(Value::as_bool)
                    == Some(false)
        }));
    }

    #[test]
    fn counterbore_presence_rejects_invalid_dimensions() {
        let temp = tempfile::tempdir().unwrap();
        let step_path = temp.path().join("part.step");
        write_step_cylinder(&step_path, (0.0, 0.0, 8.0), (1.0, 0.0, 0.0), 1.7);
        let source_path = temp.path().join("source.py");
        fs::write(&source_path, "print('source')\n").unwrap();

        let manifest = counterbore_manifest(
            sha256_file(&source_path).unwrap(),
            sha256_file(&step_path).unwrap(),
            "mechanical_interface",
            3.4,
        );
        let receipt = lint_design_data(&manifest, &default_rulepack().unwrap(), temp.path(), None);

        assert_eq!(string_field(&receipt, "status"), Some("fail"));
        assert!(receipt["checks"].as_array().unwrap().iter().any(|check| {
            string_field(check, "rule_id") == Some("actuator_mount:counterbore_step_presence")
                && string_field(check, "reason") == Some("invalid_counterbore_dimensions")
        }));
    }

    #[test]
    fn heat_set_insert_pocket_presence_requires_cylinder_and_bottom_plane() {
        let temp = tempfile::tempdir().unwrap();
        let step_path = temp.path().join("part.step");
        write_step_surfaces(
            &step_path,
            &[((-7.15, 0.0, 8.0), (1.0, 0.0, 0.0), 2.3)],
            &[((-4.3, 0.0, 8.0), (1.0, 0.0, 0.0))],
        );
        let source_path = temp.path().join("source.py");
        fs::write(&source_path, "print('source')\n").unwrap();

        let manifest = heat_set_insert_pocket_manifest(
            sha256_file(&source_path).unwrap(),
            sha256_file(&step_path).unwrap(),
            "mechanical_interface",
        );
        let receipt = lint_design_data(&manifest, &default_rulepack().unwrap(), temp.path(), None);

        assert_eq!(string_field(&receipt, "status"), Some("pass"));
        assert!(receipt["checks"].as_array().unwrap().iter().any(|check| {
            string_field(check, "rule_id")
                == Some("actuator_mount:heat_set_insert_pocket_step_presence")
                && string_field(check, "reason") == Some("ok")
                && check
                    .pointer("/measured/matched_pocket_cylinder")
                    .and_then(Value::as_bool)
                    == Some(true)
                && check
                    .pointer("/measured/matched_pocket_bottom_plane")
                    .and_then(Value::as_bool)
                    == Some(true)
        }));
    }

    #[test]
    fn heat_set_insert_pocket_presence_rejects_through_hole_without_bottom_plane() {
        let temp = tempfile::tempdir().unwrap();
        let step_path = temp.path().join("part.step");
        write_step_cylinder(&step_path, (-7.15, 0.0, 8.0), (1.0, 0.0, 0.0), 2.3);
        let source_path = temp.path().join("source.py");
        fs::write(&source_path, "print('source')\n").unwrap();

        let manifest = heat_set_insert_pocket_manifest(
            sha256_file(&source_path).unwrap(),
            sha256_file(&step_path).unwrap(),
            "mechanical_interface",
        );
        let receipt = lint_design_data(&manifest, &default_rulepack().unwrap(), temp.path(), None);

        assert_eq!(string_field(&receipt, "status"), Some("fail"));
        assert!(receipt["checks"].as_array().unwrap().iter().any(|check| {
            string_field(check, "rule_id")
                == Some("actuator_mount:heat_set_insert_pocket_step_presence")
                && string_field(check, "reason") == Some("missing_declared_feature")
                && check
                    .pointer("/measured/matched_pocket_cylinder")
                    .and_then(Value::as_bool)
                    == Some(true)
                && check
                    .pointer("/measured/matched_pocket_bottom_plane")
                    .and_then(Value::as_bool)
                    == Some(false)
        }));
    }

    #[test]
    fn bearing_seat_presence_requires_cylinder_and_shoulder_plane() {
        let temp = tempfile::tempdir().unwrap();
        let step_path = temp.path().join("part.step");
        write_step_surfaces(
            &step_path,
            &[((-8.5, 0.0, 8.0), (1.0, 0.0, 0.0), 11.0)],
            &[((-5.0, 0.0, 8.0), (1.0, 0.0, 0.0))],
        );
        let source_path = temp.path().join("source.py");
        fs::write(&source_path, "print('source')\n").unwrap();

        let manifest = bearing_seat_manifest(
            sha256_file(&source_path).unwrap(),
            sha256_file(&step_path).unwrap(),
            "mechanical_interface",
        );
        let receipt = lint_design_data(&manifest, &default_rulepack().unwrap(), temp.path(), None);

        assert_eq!(string_field(&receipt, "status"), Some("pass"));
        assert!(receipt["checks"].as_array().unwrap().iter().any(|check| {
            string_field(check, "rule_id") == Some("actuator_mount:bearing_seat_step_presence")
                && string_field(check, "reason") == Some("ok")
                && check
                    .pointer("/measured/matched_seat_cylinder")
                    .and_then(Value::as_bool)
                    == Some(true)
                && check
                    .pointer("/measured/matched_seat_shoulder_plane")
                    .and_then(Value::as_bool)
                    == Some(true)
        }));
    }

    #[test]
    fn bearing_seat_presence_rejects_cylinder_without_shoulder() {
        let temp = tempfile::tempdir().unwrap();
        let step_path = temp.path().join("part.step");
        write_step_cylinder(&step_path, (-8.5, 0.0, 8.0), (1.0, 0.0, 0.0), 11.0);
        let source_path = temp.path().join("source.py");
        fs::write(&source_path, "print('source')\n").unwrap();

        let manifest = bearing_seat_manifest(
            sha256_file(&source_path).unwrap(),
            sha256_file(&step_path).unwrap(),
            "mechanical_interface",
        );
        let receipt = lint_design_data(&manifest, &default_rulepack().unwrap(), temp.path(), None);

        assert_eq!(string_field(&receipt, "status"), Some("fail"));
        assert!(receipt["checks"].as_array().unwrap().iter().any(|check| {
            string_field(check, "rule_id") == Some("actuator_mount:bearing_seat_step_presence")
                && string_field(check, "reason") == Some("missing_declared_feature")
                && check
                    .pointer("/measured/matched_seat_cylinder")
                    .and_then(Value::as_bool)
                    == Some(true)
                && check
                    .pointer("/measured/matched_seat_shoulder_plane")
                    .and_then(Value::as_bool)
                    == Some(false)
        }));
    }

    #[test]
    fn standoff_boss_presence_requires_outer_cylinder_and_top_plane() {
        let temp = tempfile::tempdir().unwrap();
        let step_path = temp.path().join("part.step");
        write_step_surfaces(
            &step_path,
            &[((0.0, 0.0, 6.5), (0.0, 0.0, 1.0), 4.0)],
            &[((0.0, 0.0, 9.0), (0.0, 0.0, 1.0))],
        );
        let source_path = temp.path().join("source.py");
        fs::write(&source_path, "print('source')\n").unwrap();

        let manifest = standoff_boss_manifest(
            sha256_file(&source_path).unwrap(),
            sha256_file(&step_path).unwrap(),
            "mechanical_interface",
        );
        let receipt = lint_design_data(&manifest, &default_rulepack().unwrap(), temp.path(), None);

        assert_eq!(string_field(&receipt, "status"), Some("pass"));
        assert!(receipt["checks"].as_array().unwrap().iter().any(|check| {
            string_field(check, "rule_id") == Some("actuator_mount:m3_standoff_boss_step_presence")
                && string_field(check, "reason") == Some("ok")
                && check
                    .pointer("/measured/matched_boss_cylinder")
                    .and_then(Value::as_bool)
                    == Some(true)
                && check
                    .pointer("/measured/matched_boss_top_plane")
                    .and_then(Value::as_bool)
                    == Some(true)
        }));
    }

    #[test]
    fn standoff_boss_presence_rejects_inner_hole_without_outer_boss() {
        let temp = tempfile::tempdir().unwrap();
        let step_path = temp.path().join("part.step");
        write_step_surfaces(
            &step_path,
            &[((0.0, 0.0, 6.5), (0.0, 0.0, 1.0), 1.7)],
            &[((0.0, 0.0, 9.0), (0.0, 0.0, 1.0))],
        );
        let source_path = temp.path().join("source.py");
        fs::write(&source_path, "print('source')\n").unwrap();

        let manifest = standoff_boss_manifest(
            sha256_file(&source_path).unwrap(),
            sha256_file(&step_path).unwrap(),
            "mechanical_interface",
        );
        let receipt = lint_design_data(&manifest, &default_rulepack().unwrap(), temp.path(), None);

        assert_eq!(string_field(&receipt, "status"), Some("fail"));
        assert!(receipt["checks"].as_array().unwrap().iter().any(|check| {
            string_field(check, "rule_id") == Some("actuator_mount:m3_standoff_boss_step_presence")
                && string_field(check, "reason") == Some("missing_declared_feature")
                && check
                    .pointer("/measured/matched_boss_cylinder")
                    .and_then(Value::as_bool)
                    == Some(false)
                && check
                    .pointer("/measured/matched_boss_top_plane")
                    .and_then(Value::as_bool)
                    == Some(true)
        }));
    }

    #[test]
    fn standoff_boss_presence_rejects_boss_cylinder_without_top_plane() {
        let temp = tempfile::tempdir().unwrap();
        let step_path = temp.path().join("part.step");
        write_step_cylinder(&step_path, (0.0, 0.0, 6.5), (0.0, 0.0, 1.0), 4.0);
        let source_path = temp.path().join("source.py");
        fs::write(&source_path, "print('source')\n").unwrap();

        let manifest = standoff_boss_manifest(
            sha256_file(&source_path).unwrap(),
            sha256_file(&step_path).unwrap(),
            "mechanical_interface",
        );
        let receipt = lint_design_data(&manifest, &default_rulepack().unwrap(), temp.path(), None);

        assert_eq!(string_field(&receipt, "status"), Some("fail"));
        assert!(receipt["checks"].as_array().unwrap().iter().any(|check| {
            string_field(check, "rule_id") == Some("actuator_mount:m3_standoff_boss_step_presence")
                && string_field(check, "reason") == Some("missing_declared_feature")
                && check
                    .pointer("/measured/matched_boss_cylinder")
                    .and_then(Value::as_bool)
                    == Some(true)
                && check
                    .pointer("/measured/matched_boss_top_plane")
                    .and_then(Value::as_bool)
                    == Some(false)
        }));
    }

    #[test]
    fn cosmetic_straight_slot_intent_is_not_linted_by_actuator_rules() {
        let temp = tempfile::tempdir().unwrap();
        let step_path = temp.path().join("part.step");
        write_step_cylinders(
            &step_path,
            &[
                ((0.0, -6.0, 8.0), (1.0, 0.0, 0.0), 2.0),
                ((0.0, 6.0, 8.0), (1.0, 0.0, 0.0), 2.0),
            ],
        );
        let source_path = temp.path().join("source.py");
        fs::write(&source_path, "print('source')\n").unwrap();

        let manifest = slot_manifest(
            sha256_file(&source_path).unwrap(),
            sha256_file(&step_path).unwrap(),
            "cosmetic",
        );
        let receipt = lint_design_data(&manifest, &default_rulepack().unwrap(), temp.path(), None);

        assert_eq!(string_field(&receipt, "status"), Some("pass"));
        assert!(!receipt["checks"].as_array().unwrap().iter().any(|check| {
            string_field(check, "rule_id")
                .is_some_and(|rule_id| rule_id.starts_with("actuator_mount:"))
        }));
    }

    #[test]
    fn cosmetic_counterbore_intent_is_not_linted_by_actuator_rules() {
        let temp = tempfile::tempdir().unwrap();
        let step_path = temp.path().join("part.step");
        write_step_cylinders(
            &step_path,
            &[
                ((0.0, 0.0, 8.0), (1.0, 0.0, 0.0), 1.7),
                ((-8.0, 0.0, 8.0), (1.0, 0.0, 0.0), 3.4),
            ],
        );
        let source_path = temp.path().join("source.py");
        fs::write(&source_path, "print('source')\n").unwrap();

        let manifest = counterbore_manifest(
            sha256_file(&source_path).unwrap(),
            sha256_file(&step_path).unwrap(),
            "cosmetic",
            6.8,
        );
        let receipt = lint_design_data(&manifest, &default_rulepack().unwrap(), temp.path(), None);

        assert_eq!(string_field(&receipt, "status"), Some("pass"));
        assert!(!receipt["checks"].as_array().unwrap().iter().any(|check| {
            string_field(check, "rule_id")
                .is_some_and(|rule_id| rule_id.starts_with("actuator_mount:"))
        }));
    }

    #[test]
    fn cosmetic_insert_pocket_intent_is_not_linted_by_actuator_rules() {
        let temp = tempfile::tempdir().unwrap();
        let step_path = temp.path().join("part.step");
        write_step_surfaces(
            &step_path,
            &[((-7.15, 0.0, 8.0), (1.0, 0.0, 0.0), 2.3)],
            &[((-4.3, 0.0, 8.0), (1.0, 0.0, 0.0))],
        );
        let source_path = temp.path().join("source.py");
        fs::write(&source_path, "print('source')\n").unwrap();

        let manifest = heat_set_insert_pocket_manifest(
            sha256_file(&source_path).unwrap(),
            sha256_file(&step_path).unwrap(),
            "cosmetic",
        );
        let receipt = lint_design_data(&manifest, &default_rulepack().unwrap(), temp.path(), None);

        assert_eq!(string_field(&receipt, "status"), Some("pass"));
        assert!(!receipt["checks"].as_array().unwrap().iter().any(|check| {
            string_field(check, "rule_id")
                .is_some_and(|rule_id| rule_id.starts_with("actuator_mount:"))
        }));
    }

    #[test]
    fn cosmetic_bearing_seat_intent_is_not_linted_by_actuator_rules() {
        let temp = tempfile::tempdir().unwrap();
        let step_path = temp.path().join("part.step");
        write_step_surfaces(
            &step_path,
            &[((-8.5, 0.0, 8.0), (1.0, 0.0, 0.0), 11.0)],
            &[((-5.0, 0.0, 8.0), (1.0, 0.0, 0.0))],
        );
        let source_path = temp.path().join("source.py");
        fs::write(&source_path, "print('source')\n").unwrap();

        let manifest = bearing_seat_manifest(
            sha256_file(&source_path).unwrap(),
            sha256_file(&step_path).unwrap(),
            "cosmetic",
        );
        let receipt = lint_design_data(&manifest, &default_rulepack().unwrap(), temp.path(), None);

        assert_eq!(string_field(&receipt, "status"), Some("pass"));
        assert!(!receipt["checks"].as_array().unwrap().iter().any(|check| {
            string_field(check, "rule_id")
                .is_some_and(|rule_id| rule_id.starts_with("actuator_mount:"))
        }));
    }

    #[test]
    fn cosmetic_standoff_boss_intent_is_not_linted_by_actuator_rules() {
        let temp = tempfile::tempdir().unwrap();
        let step_path = temp.path().join("part.step");
        write_step_surfaces(
            &step_path,
            &[((0.0, 0.0, 6.5), (0.0, 0.0, 1.0), 4.0)],
            &[((0.0, 0.0, 9.0), (0.0, 0.0, 1.0))],
        );
        let source_path = temp.path().join("source.py");
        fs::write(&source_path, "print('source')\n").unwrap();

        let manifest = standoff_boss_manifest(
            sha256_file(&source_path).unwrap(),
            sha256_file(&step_path).unwrap(),
            "cosmetic",
        );
        let receipt = lint_design_data(&manifest, &default_rulepack().unwrap(), temp.path(), None);

        assert_eq!(string_field(&receipt, "status"), Some("pass"));
        assert!(!receipt["checks"].as_array().unwrap().iter().any(|check| {
            string_field(check, "rule_id")
                .is_some_and(|rule_id| rule_id.starts_with("actuator_mount:"))
        }));
    }

    #[test]
    fn ocp_step_cylinder_json_maps_to_step_cylinders() {
        let cylinders = parse_ocp_step_cylinders_json(
            r#"{
              "schema_version": "burr.ocp-step-cylinders.v1",
              "units": "mm",
              "cylinders": [
                {
                  "point_mm": [-4.0, 0.0, 8.0],
                  "axis": [1.0, 0.0, 0.0],
                  "radius_mm": 1.7
                }
              ],
              "warnings": []
            }"#,
        )
        .unwrap();

        assert_eq!(cylinders.len(), 1);
        assert_eq!(round(cylinders[0].radius_mm), 1.7);
        assert_eq!(
            round(cylinders[0].axis.dot(Vec3 {
                x: 1.0,
                y: 0.0,
                z: 0.0
            })),
            1.0
        );
    }

    #[test]
    fn non_mechanical_hole_intent_is_not_linted_by_actuator_rules() {
        let temp = tempfile::tempdir().unwrap();
        let step_path = temp.path().join("part.step");
        fs::write(
            &step_path,
            "ISO-10303-21;\nHEADER;\nENDSEC;\nDATA;\nENDSEC;\nEND-ISO-10303-21;\n",
        )
        .unwrap();
        let source_path = temp.path().join("source.py");
        fs::write(&source_path, "print('source')\n").unwrap();

        let mut manifest = test_manifest(
            sha256_file(&source_path).unwrap(),
            sha256_file(&step_path).unwrap(),
        );
        manifest["features"][0]["intent"] = json!("weight_reduction");
        let receipt = lint_design_data(&manifest, &default_rulepack().unwrap(), temp.path(), None);

        assert_eq!(string_field(&receipt, "status"), Some("pass"));
        assert!(!receipt["checks"].as_array().unwrap().iter().any(|check| {
            string_field(check, "rule_id")
                .is_some_and(|rule_id| rule_id.starts_with("actuator_mount:"))
        }));
        assert!(receipt["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|warning| { string_field(warning, "reason") == Some("no_applicable_features") }));
    }

    #[test]
    fn undeclared_step_cylinders_are_not_lint_targets() {
        let temp = tempfile::tempdir().unwrap();
        let step_path = temp.path().join("part.step");
        write_step_cylinders(
            &step_path,
            &[
                ((0.0, 0.0, 8.0), (1.0, 0.0, 0.0), 1.7),
                ((4.0, 3.0, 8.0), (0.0, 1.0, 0.0), 2.0),
                ((-4.0, -3.0, 8.0), (0.0, 0.0, 1.0), 0.8),
            ],
        );
        let source_path = temp.path().join("source.py");
        fs::write(&source_path, "print('source')\n").unwrap();

        let mut manifest = test_manifest(
            sha256_file(&source_path).unwrap(),
            sha256_file(&step_path).unwrap(),
        );
        manifest["features"] = json!([]);
        let receipt = lint_design_data(&manifest, &default_rulepack().unwrap(), temp.path(), None);

        assert_eq!(string_field(&receipt, "status"), Some("pass"));
        assert!(!receipt["checks"].as_array().unwrap().iter().any(|check| {
            string_field(check, "rule_id")
                .is_some_and(|rule_id| rule_id.starts_with("actuator_mount:"))
        }));
    }

    #[test]
    fn feature_presence_ignores_extra_random_cylinders() {
        let temp = tempfile::tempdir().unwrap();
        let step_path = temp.path().join("part.step");
        write_step_cylinders(
            &step_path,
            &[
                ((0.0, 0.0, 8.0), (1.0, 0.0, 0.0), 1.7),
                ((4.0, 3.0, 8.0), (0.0, 1.0, 0.0), 2.0),
                ((-4.0, -3.0, 8.0), (0.0, 0.0, 1.0), 0.8),
            ],
        );
        let source_path = temp.path().join("source.py");
        fs::write(&source_path, "print('source')\n").unwrap();

        let manifest = test_manifest(
            sha256_file(&source_path).unwrap(),
            sha256_file(&step_path).unwrap(),
        );
        let receipt = lint_design_data(&manifest, &default_rulepack().unwrap(), temp.path(), None);

        assert_eq!(string_field(&receipt, "status"), Some("pass"));
        assert!(receipt["checks"].as_array().unwrap().iter().any(|check| {
            string_field(check, "rule_id") == Some("actuator_mount:m3_clearance_hole_step_presence")
                && string_field(check, "reason") == Some("ok")
                && check
                    .pointer("/measured/candidate_cylinders")
                    .and_then(Value::as_u64)
                    == Some(3)
        }));
    }

    #[test]
    fn intent_array_can_target_mechanical_interface() {
        let temp = tempfile::tempdir().unwrap();
        let step_path = temp.path().join("part.step");
        write_step_cylinder(&step_path, (0.0, 0.0, 8.0), (1.0, 0.0, 0.0), 1.7);
        let source_path = temp.path().join("source.py");
        fs::write(&source_path, "print('source')\n").unwrap();

        let mut manifest = test_manifest(
            sha256_file(&source_path).unwrap(),
            sha256_file(&step_path).unwrap(),
        );
        manifest["features"][0]["intent"] = json!(["weight_reduction", "mechanical_interface"]);
        let receipt = lint_design_data(&manifest, &default_rulepack().unwrap(), temp.path(), None);

        assert_eq!(string_field(&receipt, "status"), Some("pass"));
        assert!(receipt["checks"].as_array().unwrap().iter().any(|check| {
            string_field(check, "rule_id") == Some("actuator_mount:m3_clearance_hole_step_presence")
                && string_field(check, "reason") == Some("ok")
        }));
    }

    fn write_step_cylinder(
        path: &Path,
        point: (f64, f64, f64),
        axis: (f64, f64, f64),
        radius: f64,
    ) {
        write_step_cylinders(path, &[(point, axis, radius)]);
    }

    fn write_step_cylinders(path: &Path, cylinders: &[((f64, f64, f64), (f64, f64, f64), f64)]) {
        write_step_surfaces(path, cylinders, &[]);
    }

    fn write_step_surfaces(
        path: &Path,
        cylinders: &[((f64, f64, f64), (f64, f64, f64), f64)],
        planes: &[((f64, f64, f64), (f64, f64, f64))],
    ) {
        let mut data = String::from("ISO-10303-21;\nHEADER;\nENDSEC;\nDATA;\n");
        let mut base = 1;
        for (point, axis, radius) in cylinders {
            data.push_str(&format!(
                "#{base} = CARTESIAN_POINT('',({},{},{}));\n#{} = DIRECTION('',({},{},{}));\n#{} = DIRECTION('',(0.,0.,1.));\n#{} = AXIS2_PLACEMENT_3D('',#{base},#{},#{});\n#{} = CYLINDRICAL_SURFACE('',#{},{});\n",
                point.0,
                point.1,
                point.2,
                base + 1,
                axis.0,
                axis.1,
                axis.2,
                base + 2,
                base + 3,
                base + 1,
                base + 2,
                base + 4,
                base + 3,
                radius,
            ));
            base += 5;
        }
        for (point, normal) in planes {
            data.push_str(&format!(
                "#{base} = CARTESIAN_POINT('',({},{},{}));\n#{} = DIRECTION('',({},{},{}));\n#{} = DIRECTION('',(1.,0.,0.));\n#{} = AXIS2_PLACEMENT_3D('',#{base},#{},#{});\n#{} = PLANE('',#{});\n",
                point.0,
                point.1,
                point.2,
                base + 1,
                normal.0,
                normal.1,
                normal.2,
                base + 2,
                base + 3,
                base + 1,
                base + 2,
                base + 4,
                base + 3,
            ));
            base += 5;
        }
        data.push_str("ENDSEC;\nEND-ISO-10303-21;\n");
        fs::write(path, data).unwrap();
    }

    fn test_manifest(source_sha: String, artifact_sha: String) -> Value {
        json!({
            "schema_version": "burr.design-data.v1",
            "artifact_id": "unit-presence",
            "artifact_version": "0.1.0",
            "artifact_type": "actuator_mount",
            "units": "mm",
            "source": {
                "path": "source.py",
                "sha256": source_sha,
                "size_bytes": 16
            },
            "artifacts": [
                {
                    "kind": "step",
                    "path": "part.step",
                    "sha256": artifact_sha
                }
            ],
            "parts": [
                {
                    "id": "housing",
                    "bbox_mm": {
                        "min": [-20.0, -14.0, 0.0],
                        "max": [20.0, 14.0, 16.0]
                    }
                }
            ],
            "features": [
                {
                    "id": "m3_claimed",
                    "part": "housing",
                    "kind": "clearance_hole",
                    "intent": "mechanical_interface",
                    "fastener": "M3",
                    "diameter_mm": 3.4,
                    "center_mm": [0.0, 0.0, 8.0],
                    "axis": [1.0, 0.0, 0.0],
                    "role": "alignment"
                }
            ]
        })
    }

    fn slot_manifest(source_sha: String, artifact_sha: String, intent: &str) -> Value {
        json!({
            "schema_version": "burr.design-data.v1",
            "artifact_id": "unit-slot-presence",
            "artifact_version": "0.1.0",
            "artifact_type": "actuator_mount",
            "units": "mm",
            "source": {
                "path": "source.py",
                "sha256": source_sha,
                "size_bytes": 16
            },
            "artifacts": [
                {
                    "kind": "step",
                    "path": "part.step",
                    "sha256": artifact_sha
                }
            ],
            "parts": [
                {
                    "id": "housing",
                    "bbox_mm": {
                        "min": [-20.0, -14.0, 0.0],
                        "max": [20.0, 14.0, 16.0]
                    }
                }
            ],
            "features": [
                {
                    "id": "slot_claimed",
                    "part": "housing",
                    "kind": "straight_slot",
                    "intent": intent,
                    "width_mm": 4.0,
                    "length_mm": 16.0,
                    "center_mm": [0.0, 0.0, 8.0],
                    "axis": [1.0, 0.0, 0.0],
                    "span_axis": [0.0, 1.0, 0.0],
                    "role": "loaded_mount"
                }
            ]
        })
    }

    fn counterbore_manifest(
        source_sha: String,
        artifact_sha: String,
        intent: &str,
        counterbore_diameter: f64,
    ) -> Value {
        json!({
            "schema_version": "burr.design-data.v1",
            "artifact_id": "unit-counterbore-presence",
            "artifact_version": "0.1.0",
            "artifact_type": "actuator_mount",
            "units": "mm",
            "source": {
                "path": "source.py",
                "sha256": source_sha,
                "size_bytes": 16
            },
            "artifacts": [
                {
                    "kind": "step",
                    "path": "part.step",
                    "sha256": artifact_sha
                }
            ],
            "parts": [
                {
                    "id": "housing",
                    "bbox_mm": {
                        "min": [-10.0, -10.0, 0.0],
                        "max": [10.0, 10.0, 16.0]
                    }
                }
            ],
            "features": [
                {
                    "id": "counterbore_claimed",
                    "part": "housing",
                    "kind": "counterbore",
                    "intent": intent,
                    "bore_diameter_mm": 3.4,
                    "counterbore_diameter_mm": counterbore_diameter,
                    "counterbore_depth_mm": 4.0,
                    "center_mm": [0.0, 0.0, 8.0],
                    "axis": [1.0, 0.0, 0.0],
                    "counterbore_center_mm": [-8.0, 0.0, 8.0],
                    "shoulder_center_mm": [-6.0, 0.0, 8.0],
                    "role": "loaded_mount"
                }
            ]
        })
    }

    fn heat_set_insert_pocket_manifest(
        source_sha: String,
        artifact_sha: String,
        intent: &str,
    ) -> Value {
        json!({
            "schema_version": "burr.design-data.v1",
            "artifact_id": "unit-heat-set-insert-pocket-presence",
            "artifact_version": "0.1.0",
            "artifact_type": "actuator_mount",
            "units": "mm",
            "source": {
                "path": "source.py",
                "sha256": source_sha,
                "size_bytes": 16
            },
            "artifacts": [
                {
                    "kind": "step",
                    "path": "part.step",
                    "sha256": artifact_sha
                }
            ],
            "parts": [
                {
                    "id": "housing",
                    "bbox_mm": {
                        "min": [-10.0, -10.0, 0.0],
                        "max": [10.0, 10.0, 16.0]
                    }
                }
            ],
            "features": [
                {
                    "id": "m3_insert_pocket",
                    "part": "housing",
                    "kind": "heat_set_insert_pocket",
                    "intent": intent,
                    "insert": "M3x5.7",
                    "pocket_diameter_mm": 4.6,
                    "pocket_depth_mm": 5.7,
                    "center_mm": [0.0, 0.0, 8.0],
                    "axis": [1.0, 0.0, 0.0],
                    "pocket_center_mm": [-7.15, 0.0, 8.0],
                    "bottom_center_mm": [-4.3, 0.0, 8.0],
                    "role": "threaded_mount"
                }
            ]
        })
    }

    fn bearing_seat_manifest(source_sha: String, artifact_sha: String, intent: &str) -> Value {
        json!({
            "schema_version": "burr.design-data.v1",
            "artifact_id": "unit-bearing-seat-presence",
            "artifact_version": "0.1.0",
            "artifact_type": "actuator_mount",
            "units": "mm",
            "source": {
                "path": "source.py",
                "sha256": source_sha,
                "size_bytes": 16
            },
            "artifacts": [
                {
                    "kind": "step",
                    "path": "part.step",
                    "sha256": artifact_sha
                }
            ],
            "parts": [
                {
                    "id": "housing",
                    "bbox_mm": {
                        "min": [-12.0, -14.0, 0.0],
                        "max": [12.0, 14.0, 16.0]
                    }
                }
            ],
            "features": [
                {
                    "id": "bearing_608_seat",
                    "part": "housing",
                    "kind": "bearing_seat",
                    "intent": intent,
                    "bearing": "608",
                    "seat_diameter_mm": 22.0,
                    "seat_depth_mm": 7.0,
                    "center_mm": [0.0, 0.0, 8.0],
                    "axis": [1.0, 0.0, 0.0],
                    "seat_center_mm": [-8.5, 0.0, 8.0],
                    "shoulder_center_mm": [-5.0, 0.0, 8.0],
                    "role": "bearing_support"
                }
            ]
        })
    }

    fn standoff_boss_manifest(source_sha: String, artifact_sha: String, intent: &str) -> Value {
        json!({
            "schema_version": "burr.design-data.v1",
            "artifact_id": "unit-standoff-boss-presence",
            "artifact_version": "0.1.0",
            "artifact_type": "actuator_mount",
            "units": "mm",
            "source": {
                "path": "source.py",
                "sha256": source_sha,
                "size_bytes": 16
            },
            "artifacts": [
                {
                    "kind": "step",
                    "path": "part.step",
                    "sha256": artifact_sha
                }
            ],
            "parts": [
                {
                    "id": "housing",
                    "bbox_mm": {
                        "min": [-12.0, -12.0, 0.0],
                        "max": [12.0, 12.0, 9.0]
                    }
                }
            ],
            "features": [
                {
                    "id": "m3_standoff_boss",
                    "part": "housing",
                    "kind": "standoff_boss",
                    "intent": intent,
                    "fastener": "M3",
                    "boss_diameter_mm": 8.0,
                    "boss_height_mm": 5.0,
                    "center_mm": [0.0, 0.0, 6.5],
                    "axis": [0.0, 0.0, 1.0],
                    "role": "pcb_standoff",
                    "supports_feature_id": "m3_bossed_mount"
                },
                {
                    "id": "m3_bossed_mount",
                    "part": "housing",
                    "kind": "clearance_hole",
                    "intent": "reference",
                    "fastener": "M3",
                    "diameter_mm": 3.4,
                    "support_diameter_mm": 8.0,
                    "center_mm": [0.0, 0.0, 6.5],
                    "axis": [0.0, 0.0, 1.0],
                    "role": "bossed_mount"
                }
            ]
        })
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
