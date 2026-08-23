use crate::project::{PackSource, Project, ResolvedPack};
use burr::{
    build_receipt_repair_packet, find_design_data_paths, lint_targets, LintOptions, LintResult,
    DESIGN_DATA_FILE_NAME,
};
use serde::Serialize;
use serde_json::{json, Value};
use std::path::Path;

pub const CHECK_RESULTS_SCHEMA_VERSION: &str = "burr.check-results.v1";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Outcome {
    Pass,
    Incomplete,
    Fail,
}

impl Outcome {
    pub fn label(self) -> &'static str {
        match self {
            Self::Pass => "PASS",
            Self::Incomplete => "INCOMPLETE",
            Self::Fail => "FAIL",
        }
    }

    fn from_receipt(receipt: &Value) -> Self {
        match receipt
            .get("outcome")
            .or_else(|| receipt.get("status"))
            .and_then(Value::as_str)
        {
            Some("pass") => Self::Pass,
            Some("incomplete") => Self::Incomplete,
            _ => Self::Fail,
        }
    }

    fn combine(self, other: Self) -> Self {
        match (self, other) {
            (Self::Fail, _) | (_, Self::Fail) => Self::Fail,
            (Self::Incomplete, _) | (_, Self::Incomplete) => Self::Incomplete,
            _ => Self::Pass,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    Mesh,
    Brep,
    Assembly,
    DeclaredIntent,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
    Notice,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct GeometryReference {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub part_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub feature_id: Option<String>,
    pub face_ids: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Finding {
    pub id: String,
    pub severity: Severity,
    pub code: String,
    pub title: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rule_id: Option<String>,
    pub evidence: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remediation: Option<String>,
    pub geometry: Vec<GeometryReference>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct TargetResult {
    pub source_path: String,
    pub outcome: Outcome,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifact_id: Option<String>,
    pub checks: u64,
    pub failures: u64,
    pub incomplete_checks: u64,
    pub warnings: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct PackSummary {
    pub targets: usize,
    pub targets_passed: usize,
    pub targets_incomplete: usize,
    pub targets_failed: usize,
    pub findings: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct PackResult {
    pub id: String,
    pub version: String,
    pub source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    pub required_capabilities: Vec<Capability>,
    pub available_capabilities: Vec<Capability>,
    pub outcome: Outcome,
    pub targets: Vec<TargetResult>,
    pub findings: Vec<Finding>,
    pub summary: PackSummary,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct CheckReport {
    pub schema_version: &'static str,
    pub capability_catalog: Vec<Capability>,
    pub outcome: Option<Outcome>,
    pub packs: Vec<PackResult>,
}

impl CheckReport {
    pub fn run(project: &Project) -> Self {
        let packs = project
            .packs()
            .iter()
            .map(|pack| run_pack(project, pack))
            .collect::<Vec<_>>();
        let outcome = packs
            .iter()
            .map(|pack| pack.outcome)
            .reduce(Outcome::combine);
        Self {
            schema_version: CHECK_RESULTS_SCHEMA_VERSION,
            capability_catalog: vec![
                Capability::Mesh,
                Capability::Brep,
                Capability::Assembly,
                Capability::DeclaredIntent,
            ],
            outcome,
            packs,
        }
    }

    pub fn public_state(&self) -> Value {
        serde_json::to_value(self).expect("check report is serializable")
    }
}

fn run_pack(project: &Project, pack: &ResolvedPack) -> PackResult {
    match (&pack.source, pack.id.as_str()) {
        (PackSource::Builtin, "builtin:mechanical-fit") => run_mechanical_fit(project, pack),
        (PackSource::Local(path), _) => unsupported_local_pack(project, pack, path),
        (PackSource::Builtin, _) => unavailable_builtin_pack(project, pack),
    }
}

fn run_mechanical_fit(project: &Project, pack: &ResolvedPack) -> PackResult {
    let inputs = project
        .model_roots()
        .iter()
        .map(|path| path.to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    let discovered = match find_design_data_paths(&inputs, project.root()) {
        Ok(paths) => paths,
        Err(error) => {
            return pack_runtime_error(
                project,
                pack,
                "design_data_discovery_failed",
                "Mechanical-fit input discovery failed",
                error,
                json!({ "model_paths": public_model_paths(project) }),
            )
        }
    };
    if discovered.is_empty() {
        return pack_runtime_error(
            project,
            pack,
            "missing_design_data",
            "Mechanical-fit input is missing",
            format!("No {DESIGN_DATA_FILE_NAME} file was found under the configured model paths."),
            json!({
                "required_capability": Capability::DeclaredIntent,
                "required_file": DESIGN_DATA_FILE_NAME,
                "model_paths": public_model_paths(project),
            }),
        );
    }

    let options = LintOptions {
        rulepack_path: None,
        write_receipt: false,
        cwd: project.root().to_path_buf(),
    };
    let results = match lint_targets(&inputs, &options) {
        Ok(results) => results,
        Err(error) => {
            return pack_runtime_error(
                project,
                pack,
                "mechanical_fit_evaluation_failed",
                "Mechanical-fit evaluation could not complete",
                error,
                json!({
                    "design_data_paths": discovered
                        .iter()
                        .filter_map(|path| project.relative_path(path))
                        .collect::<Vec<_>>()
                }),
            )
        }
    };

    let mut outcome = Outcome::Pass;
    let mut targets = Vec::with_capacity(results.len());
    let mut findings = Vec::new();
    for result in &results {
        let target_outcome = Outcome::from_receipt(&result.receipt);
        outcome = outcome.combine(target_outcome);
        targets.push(target_result(project, result, target_outcome));
        findings.extend(findings_from_receipt(project, result));
    }

    finish_pack_result(
        project,
        pack,
        vec![Capability::DeclaredIntent],
        vec![Capability::DeclaredIntent],
        outcome,
        targets,
        findings,
    )
}

fn target_result(project: &Project, result: &LintResult, outcome: Outcome) -> TargetResult {
    TargetResult {
        source_path: public_source_path(project, &result.design_data_path),
        outcome,
        artifact_id: result
            .receipt
            .get("artifact_id")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        checks: result
            .receipt
            .pointer("/summary/checks")
            .and_then(Value::as_u64)
            .unwrap_or(0),
        failures: result
            .receipt
            .pointer("/summary/failures")
            .and_then(Value::as_u64)
            .unwrap_or(0),
        incomplete_checks: result
            .receipt
            .pointer("/summary/incomplete_checks")
            .and_then(Value::as_u64)
            .unwrap_or(0),
        warnings: result
            .receipt
            .pointer("/summary/warnings")
            .and_then(Value::as_u64)
            .unwrap_or(0),
    }
}

fn findings_from_receipt(project: &Project, result: &LintResult) -> Vec<Finding> {
    let source_path = public_source_path(project, &result.design_data_path);
    let repair = build_receipt_repair_packet(&result.receipt);
    let mut findings = Vec::new();

    for (index, failure) in repair
        .get("failures")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .enumerate()
    {
        let rule_id = string_value(failure, "rule_id");
        let reason = string_value(failure, "reason").unwrap_or_else(|| "check_failed".to_string());
        let feature_id = string_value(failure, "feature_id");
        let check = matching_check(
            &result.receipt,
            rule_id.as_deref(),
            feature_id.as_deref(),
            &reason,
        )
        .cloned()
        .unwrap_or_else(|| failure.clone());
        let model_path = check
            .pointer("/measured/artifact_path")
            .and_then(Value::as_str)
            .and_then(|path| {
                public_model_reference_path(project, &result.design_data_path, Path::new(path))
            });
        findings.push(Finding {
            id: finding_id("failure", &source_path, rule_id.as_deref(), index),
            severity: Severity::Error,
            code: reason,
            title: string_value(failure, "headline")
                .unwrap_or_else(|| "Mechanical check failed".to_string()),
            message: string_value(failure, "problem")
                .or_else(|| string_value(&check, "message"))
                .unwrap_or_else(|| "A mechanical check failed.".to_string()),
            source_path: Some(source_path.clone()),
            rule_id,
            evidence: check,
            remediation: string_value(failure, "fix"),
            geometry: geometry_reference(model_path, None, feature_id),
        });
    }

    for (index, reason) in repair
        .get("incomplete_reasons")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .enumerate()
    {
        let rule_id = string_value(reason, "rule_id");
        let code = string_value(reason, "reason").unwrap_or_else(|| "incomplete".to_string());
        let feature_id = string_value(reason, "feature_id");
        findings.push(Finding {
            id: finding_id("incomplete", &source_path, rule_id.as_deref(), index),
            severity: Severity::Warning,
            code,
            title: "Mechanical check is incomplete".to_string(),
            message: string_value(reason, "message").unwrap_or_else(|| {
                "Burr could not establish a complete mechanical result.".to_string()
            }),
            source_path: Some(source_path.clone()),
            rule_id,
            evidence: reason.clone(),
            remediation: None,
            geometry: geometry_reference(None, None, feature_id),
        });
    }

    for (index, warning) in result
        .receipt
        .get("warnings")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|warning| warning.get("affects_outcome").and_then(Value::as_bool) != Some(true))
        .enumerate()
    {
        let rule_id = string_value(warning, "rule_id");
        let code = string_value(warning, "reason").unwrap_or_else(|| "warning".to_string());
        findings.push(Finding {
            id: finding_id("notice", &source_path, rule_id.as_deref(), index),
            severity: Severity::Notice,
            code,
            title: "Mechanical check notice".to_string(),
            message: string_value(warning, "message")
                .unwrap_or_else(|| "The mechanical check reported a notice.".to_string()),
            source_path: Some(source_path.clone()),
            rule_id,
            evidence: warning.clone(),
            remediation: None,
            geometry: Vec::new(),
        });
    }

    findings
}

fn matching_check<'a>(
    receipt: &'a Value,
    rule_id: Option<&str>,
    feature_id: Option<&str>,
    reason: &str,
) -> Option<&'a Value> {
    receipt
        .get("checks")
        .and_then(Value::as_array)?
        .iter()
        .find(|check| {
            check.get("status").and_then(Value::as_str) == Some("fail")
                && check.get("rule_id").and_then(Value::as_str) == rule_id
                && check.get("feature_id").and_then(Value::as_str) == feature_id
                && check.get("reason").and_then(Value::as_str) == Some(reason)
        })
}

fn unsupported_local_pack(project: &Project, pack: &ResolvedPack, path: &Path) -> PackResult {
    pack_runtime_error(
        project,
        pack,
        "local_pack_runtime_unavailable",
        "Local pack is configured but not executable yet",
        "Burr resolved this local pack, but the shared local check definition contract is not available in this release."
            .to_string(),
        json!({ "path": project.relative_path(path) }),
    )
}

fn unavailable_builtin_pack(project: &Project, pack: &ResolvedPack) -> PackResult {
    pack_runtime_error(
        project,
        pack,
        "builtin_pack_runtime_unavailable",
        "Built-in pack runtime is unavailable",
        format!("Burr does not have a runtime for {}.", pack.id),
        Value::Null,
    )
}

fn pack_runtime_error(
    project: &Project,
    pack: &ResolvedPack,
    code: &str,
    title: &str,
    message: String,
    evidence: Value,
) -> PackResult {
    let finding = Finding {
        id: format!("{}:{code}", pack.id),
        severity: Severity::Warning,
        code: code.to_string(),
        title: title.to_string(),
        message,
        source_path: None,
        rule_id: None,
        evidence,
        remediation: None,
        geometry: Vec::new(),
    };
    finish_pack_result(
        project,
        pack,
        if pack.id == "builtin:mechanical-fit" {
            vec![Capability::DeclaredIntent]
        } else {
            Vec::new()
        },
        Vec::new(),
        Outcome::Incomplete,
        Vec::new(),
        vec![finding],
    )
}

fn finish_pack_result(
    project: &Project,
    pack: &ResolvedPack,
    required_capabilities: Vec<Capability>,
    available_capabilities: Vec<Capability>,
    outcome: Outcome,
    targets: Vec<TargetResult>,
    findings: Vec<Finding>,
) -> PackResult {
    let summary = PackSummary {
        targets: targets.len(),
        targets_passed: targets
            .iter()
            .filter(|target| target.outcome == Outcome::Pass)
            .count(),
        targets_incomplete: targets
            .iter()
            .filter(|target| target.outcome == Outcome::Incomplete)
            .count(),
        targets_failed: targets
            .iter()
            .filter(|target| target.outcome == Outcome::Fail)
            .count(),
        findings: findings.len(),
    };
    let (source, path) = match &pack.source {
        PackSource::Builtin => ("builtin".to_string(), None),
        PackSource::Local(path) => ("local".to_string(), project.relative_path(path)),
    };
    PackResult {
        id: pack.id.clone(),
        version: pack.version.clone(),
        source,
        path,
        required_capabilities,
        available_capabilities,
        outcome,
        targets,
        findings,
        summary,
    }
}

fn public_model_paths(project: &Project) -> Vec<String> {
    project
        .model_roots()
        .iter()
        .filter_map(|path| project.relative_path(path))
        .collect()
}

fn public_source_path(project: &Project, path: &Path) -> String {
    project
        .relative_path(path)
        .unwrap_or_else(|| "<outside-project>".to_string())
}

fn public_model_reference_path(
    project: &Project,
    design_data_path: &Path,
    artifact_path: &Path,
) -> Option<String> {
    let candidate = if artifact_path.is_absolute() {
        artifact_path.to_path_buf()
    } else {
        design_data_path.parent()?.join(artifact_path)
    };
    let canonical = candidate.canonicalize().ok()?;
    project
        .contains_model(&canonical)
        .then(|| project.relative_path(&canonical))
        .flatten()
}

fn geometry_reference(
    model_path: Option<String>,
    part_id: Option<String>,
    feature_id: Option<String>,
) -> Vec<GeometryReference> {
    if model_path.is_none() && part_id.is_none() && feature_id.is_none() {
        return Vec::new();
    }
    vec![GeometryReference {
        model_path,
        part_id,
        feature_id,
        face_ids: Vec::new(),
    }]
}

fn string_value(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

fn finding_id(kind: &str, source_path: &str, rule_id: Option<&str>, index: usize) -> String {
    format!(
        "{kind}:{}:{}:{index}",
        source_path.replace(['/', '\\'], ":"),
        rule_id.unwrap_or("scope")
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    const GOOD_FIXTURE: &str = "examples/linear-actuator-good";
    const BAD_FIXTURE: &str = "examples/linear-actuator-bad";

    fn copy_design_fixture(source: &str, destination: &Path) {
        fs::create_dir_all(destination).unwrap();
        for file in ["source.py", "actuator.step", DESIGN_DATA_FILE_NAME] {
            fs::copy(
                Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join(source)
                    .join(file),
                destination.join(file),
            )
            .unwrap();
        }
    }

    fn configured_project(
        fixture: Option<&str>,
        pack_enabled: bool,
    ) -> (tempfile::TempDir, Project) {
        let temp = tempdir().unwrap();
        let models = temp.path().join("models");
        fs::create_dir_all(&models).unwrap();
        if let Some(fixture) = fixture {
            copy_design_fixture(fixture, &models.join("design"));
        }
        fs::create_dir_all(temp.path().join(".burr")).unwrap();
        let pack = if pack_enabled {
            "\n[[packs]]\nid = \"builtin:mechanical-fit\"\n"
        } else {
            ""
        };
        fs::write(
            temp.path().join(".burr/config.toml"),
            format!(
                "schema_version = \"burr.project.v1\"\n[project]\nmodels = [\"models\"]\n{pack}"
            ),
        )
        .unwrap();
        let project = Project::discover(temp.path()).unwrap();
        (temp, project)
    }

    #[test]
    fn mechanical_fit_preserves_good_and_bad_outcomes_without_writing_receipts() {
        for (fixture, expected, expected_reason) in [
            (GOOD_FIXTURE, Outcome::Pass, None),
            (
                BAD_FIXTURE,
                Outcome::Fail,
                Some("insufficient_edge_distance"),
            ),
        ] {
            let (temp, project) = configured_project(Some(fixture), true);
            let report = CheckReport::run(&project);
            assert_eq!(report.outcome, Some(expected));
            assert_eq!(report.packs.len(), 1);
            assert_eq!(report.packs[0].outcome, expected);
            assert_eq!(report.packs[0].targets[0].outcome, expected);
            if let Some(reason) = expected_reason {
                let finding = report.packs[0]
                    .findings
                    .iter()
                    .find(|finding| finding.code == reason)
                    .unwrap();
                assert_eq!(finding.severity, Severity::Error);
                assert_eq!(
                    finding
                        .evidence
                        .pointer("/measured/center_to_edge_mm")
                        .and_then(Value::as_f64),
                    Some(8.0)
                );
                assert_eq!(
                    public_model_reference_path(
                        &project,
                        &temp.path().join("models/design/burr-design-data.json"),
                        Path::new("actuator.step")
                    )
                    .as_deref(),
                    Some("models/design/actuator.step")
                );
                assert_eq!(
                    finding.geometry[0].feature_id.as_deref(),
                    Some("m3_lower_left")
                );
            }
            assert!(!temp.path().join("models/design/burr-receipt.json").exists());
        }
    }

    #[test]
    fn missing_declared_intent_is_incomplete_not_pass() {
        let (_temp, project) = configured_project(None, true);
        let report = CheckReport::run(&project);
        assert_eq!(report.outcome, Some(Outcome::Incomplete));
        assert_eq!(report.packs[0].outcome, Outcome::Incomplete);
        assert!(report.packs[0].available_capabilities.is_empty());
        assert_eq!(report.packs[0].findings[0].code, "missing_design_data");
    }

    #[test]
    fn disabled_pack_does_not_run_or_claim_pass() {
        let (_temp, project) = configured_project(Some(GOOD_FIXTURE), false);
        let report = CheckReport::run(&project);
        assert_eq!(report.outcome, None);
        assert!(report.packs.is_empty());
        assert_eq!(report.public_state()["outcome"], Value::Null);
        assert_eq!(report.capability_catalog.len(), 4);
    }
}
