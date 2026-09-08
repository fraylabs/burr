use serde::Deserialize;
use serde_json::{json, Value};
use std::{
    collections::HashSet,
    fs,
    path::{Component, Path, PathBuf},
};

pub const PROJECT_CONFIG_SCHEMA_VERSION: &str = "burr.project.v2";

const CONFIG_DIRECTORY: &str = ".burr";
const CONFIG_FILE: &str = "config.toml";

#[derive(Clone, Debug)]
pub struct Project {
    root: PathBuf,
    model_roots: Vec<PathBuf>,
    motions: Vec<Motion>,
    config_path: Option<PathBuf>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Motion {
    pub id: String,
    pub label: String,
    pub model: String,
    pub from_label: String,
    pub to_label: String,
    pub duration_ms: u32,
    pub joints: Vec<MotionJoint>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum MotionJoint {
    Revolute {
        components: Vec<String>,
        origin_mm: [f32; 3],
        axis: [f32; 3],
        angle_degrees: f32,
    },
    Prismatic {
        components: Vec<String>,
        axis: [f32; 3],
        distance_mm: f32,
    },
}

impl Motion {
    pub fn cache_signature(&self) -> String {
        let mut signature = format!(
            "{}|{}|{}|{}|{}|{}",
            self.id,
            self.model,
            self.from_label,
            self.to_label,
            self.duration_ms,
            self.joints.len()
        );
        for joint in &self.joints {
            match joint {
                MotionJoint::Revolute {
                    components,
                    origin_mm,
                    axis,
                    angle_degrees,
                } => {
                    signature.push_str("|revolute|");
                    append_components(&mut signature, components);
                    append_f32s(&mut signature, origin_mm);
                    append_f32s(&mut signature, axis);
                    append_f32s(&mut signature, &[*angle_degrees]);
                }
                MotionJoint::Prismatic {
                    components,
                    axis,
                    distance_mm,
                } => {
                    signature.push_str("|prismatic|");
                    append_components(&mut signature, components);
                    append_f32s(&mut signature, axis);
                    append_f32s(&mut signature, &[*distance_mm]);
                }
            }
        }
        signature
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProjectConfig {
    schema_version: String,
    project: ProjectSection,
    #[serde(default)]
    motions: Vec<MotionSection>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProjectSection {
    models: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct MotionSection {
    id: String,
    label: String,
    model: String,
    from_label: String,
    to_label: String,
    duration_ms: u32,
    joints: Vec<MotionJointSection>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case", deny_unknown_fields)]
enum MotionJointSection {
    Revolute {
        components: Vec<String>,
        origin_mm: [f32; 3],
        axis: [f32; 3],
        angle_degrees: f32,
    },
    Prismatic {
        components: Vec<String>,
        axis: [f32; 3],
        distance_mm: f32,
    },
}

impl Project {
    pub fn discover(start: &Path) -> Result<Self, String> {
        let start = start
            .canonicalize()
            .map_err(|error| format!("Failed to open Burr project {}: {error}", start.display()))?;
        if !start.is_dir() {
            return Err(format!(
                "Burr project path is not a directory: {}",
                start.display()
            ));
        }

        let Some(discovered_config_path) = find_config(&start) else {
            return Ok(Self {
                root: start.clone(),
                model_roots: vec![start],
                motions: Vec::new(),
                config_path: None,
            });
        };
        let discovered_config_directory = discovered_config_path
            .parent()
            .ok_or_else(|| "Burr project configuration has no parent directory.".to_string())?;
        let root = discovered_config_directory
            .parent()
            .ok_or_else(|| "Burr project configuration has no project root.".to_string())?
            .canonicalize()
            .map_err(|error| format!("Failed to resolve Burr project root: {error}"))?;
        let config_directory = discovered_config_directory
            .canonicalize()
            .map_err(|error| format!("Failed to resolve Burr configuration directory: {error}"))?;
        if !config_directory.starts_with(&root) {
            return Err(
                "Burr configuration directory must remain inside its project root.".to_string(),
            );
        }
        let config_path = discovered_config_path
            .canonicalize()
            .map_err(|error| format!("Failed to resolve Burr project configuration: {error}"))?;
        if config_path.parent() != Some(config_directory.as_path()) {
            return Err(
                "Burr project configuration must remain inside its .burr directory.".to_string(),
            );
        }
        let config = read_project_config(&config_path)?;
        let model_roots = resolve_model_roots(&root, &config.project.models)?;
        let motions = resolve_motions(&root, &model_roots, config.motions)?;

        Ok(Self {
            root,
            model_roots,
            motions,
            config_path: Some(config_path),
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn model_roots(&self) -> &[PathBuf] {
        &self.model_roots
    }

    pub fn motion(&self, id: &str) -> Option<&Motion> {
        self.motions.iter().find(|motion| motion.id == id)
    }

    pub fn is_configured(&self) -> bool {
        self.config_path.is_some()
    }

    pub fn contains_model(&self, path: &Path) -> bool {
        self.model_roots.iter().any(|root| path.starts_with(root))
    }

    pub fn public_state(&self) -> Value {
        let root_name = self
            .root
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("project");
        let model_paths = self
            .model_roots
            .iter()
            .filter_map(|path| relative_portable_path(&self.root, path))
            .collect::<Vec<_>>();
        json!({
            "schema_version": "burr.project-state.v2",
            "root": root_name,
            "configured": self.is_configured(),
            "config_path": self.config_path.as_ref().and_then(|path| relative_portable_path(&self.root, path)),
            "model_paths": model_paths,
            "motions": self.motions.iter().map(|motion| json!({
                "id": motion.id,
                "label": motion.label,
                "model": motion.model,
                "from_label": motion.from_label,
                "to_label": motion.to_label,
                "duration_ms": motion.duration_ms,
            })).collect::<Vec<_>>(),
        })
    }
}

fn find_config(start: &Path) -> Option<PathBuf> {
    start
        .ancestors()
        .map(|directory| directory.join(CONFIG_DIRECTORY).join(CONFIG_FILE))
        .find(|candidate| candidate.is_file())
}

fn read_project_config(path: &Path) -> Result<ProjectConfig, String> {
    let text = fs::read_to_string(path)
        .map_err(|error| format!("Failed to read {}: {error}", path.display()))?;
    #[derive(Deserialize)]
    struct SchemaHeader {
        schema_version: String,
    }
    let header = toml::from_str::<SchemaHeader>(&text)
        .map_err(|error| format!("Invalid {}: {error}", path.display()))?;
    if header.schema_version != PROJECT_CONFIG_SCHEMA_VERSION {
        return Err(format!(
            "Unsupported Burr project schema '{}'; expected '{PROJECT_CONFIG_SCHEMA_VERSION}'.",
            header.schema_version
        ));
    }
    let config = toml::from_str::<ProjectConfig>(&text)
        .map_err(|error| format!("Invalid {}: {error}", path.display()))?;
    debug_assert_eq!(config.schema_version, PROJECT_CONFIG_SCHEMA_VERSION);
    Ok(config)
}

fn resolve_model_roots(root: &Path, configured: &[String]) -> Result<Vec<PathBuf>, String> {
    if configured.is_empty() {
        return Err(
            "Burr project configuration must declare at least one project.models path.".to_string(),
        );
    }

    let mut roots = Vec::with_capacity(configured.len());
    for relative in configured {
        let path = resolve_project_path(root, relative, "Model path")?;
        if !path.is_dir() {
            return Err(format!(
                "Configured model path is not a directory: {relative}"
            ));
        }
        if roots
            .iter()
            .any(|existing: &PathBuf| path.starts_with(existing) || existing.starts_with(&path))
        {
            return Err(format!(
                "Configured model paths overlap after resolution: {relative}"
            ));
        }
        roots.push(path);
    }
    Ok(roots)
}

fn resolve_motions(
    root: &Path,
    model_roots: &[PathBuf],
    configured: Vec<MotionSection>,
) -> Result<Vec<Motion>, String> {
    let mut motions = Vec::with_capacity(configured.len());
    for motion in configured {
        if motion.id.is_empty()
            || !motion
                .id
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        {
            return Err(format!(
                "Motion id must use lowercase letters, numbers, and hyphens: {}",
                motion.id
            ));
        }
        if motions
            .iter()
            .any(|existing: &Motion| existing.id == motion.id)
        {
            return Err(format!("Motion ids must be unique: {}", motion.id));
        }
        if motion.label.trim().is_empty()
            || motion.from_label.trim().is_empty()
            || motion.to_label.trim().is_empty()
        {
            return Err(format!("Motion labels must not be empty: {}", motion.id));
        }
        if !(100..=10_000).contains(&motion.duration_ms) {
            return Err(format!(
                "Motion duration_ms must be from 100 to 10000: {}",
                motion.id
            ));
        }
        if motion.joints.is_empty() {
            return Err(format!(
                "Motion must declare at least one rigid joint: {}",
                motion.id
            ));
        }
        let model = resolve_motion_model(root, model_roots, &motion.model)?;
        let joints = resolve_motion_joints(&motion.id, motion.joints)?;
        motions.push(Motion {
            id: motion.id,
            label: motion.label,
            model,
            from_label: motion.from_label,
            to_label: motion.to_label,
            duration_ms: motion.duration_ms,
            joints,
        });
    }
    Ok(motions)
}

fn resolve_motion_model(
    root: &Path,
    model_roots: &[PathBuf],
    configured: &str,
) -> Result<String, String> {
    let path = resolve_project_path(root, configured, "Motion model")?;
    if !path.is_file() || !is_step_path(&path) {
        return Err(format!(
            "Motion model must be an existing STEP file: {configured}"
        ));
    }
    if !model_roots
        .iter()
        .any(|model_root| path.starts_with(model_root))
    {
        return Err(format!(
            "Motion model must remain inside the configured model scope: {configured}"
        ));
    }
    relative_portable_path(root, &path)
        .ok_or_else(|| format!("Motion model could not be made project-relative: {configured}"))
}

fn resolve_motion_joints(
    motion_id: &str,
    configured: Vec<MotionJointSection>,
) -> Result<Vec<MotionJoint>, String> {
    let mut assigned_components = HashSet::new();
    configured
        .into_iter()
        .enumerate()
        .map(|(index, joint)| {
            let label = format!("Motion '{motion_id}' joint {}", index + 1);
            match joint {
                MotionJointSection::Revolute {
                    components,
                    origin_mm,
                    axis,
                    angle_degrees,
                } => {
                    let components = resolve_joint_components(
                        &label,
                        components,
                        &mut assigned_components,
                    )?;
                    require_finite_vector(&label, "origin_mm", origin_mm)?;
                    let axis = normalized_axis(&label, axis)?;
                    if !angle_degrees.is_finite()
                        || angle_degrees.abs() <= f32::EPSILON
                        || angle_degrees.abs() > 360.0
                    {
                        return Err(format!(
                            "{label} angle_degrees must be finite, non-zero, and at most 360 degrees."
                        ));
                    }
                    Ok(MotionJoint::Revolute {
                        components,
                        origin_mm,
                        axis,
                        angle_degrees,
                    })
                }
                MotionJointSection::Prismatic {
                    components,
                    axis,
                    distance_mm,
                } => {
                    let components = resolve_joint_components(
                        &label,
                        components,
                        &mut assigned_components,
                    )?;
                    let axis = normalized_axis(&label, axis)?;
                    if !distance_mm.is_finite() || distance_mm.abs() <= f32::EPSILON {
                        return Err(format!(
                            "{label} distance_mm must be finite and non-zero."
                        ));
                    }
                    Ok(MotionJoint::Prismatic {
                        components,
                        axis,
                        distance_mm,
                    })
                }
            }
        })
        .collect()
}

fn resolve_joint_components(
    label: &str,
    components: Vec<String>,
    assigned_components: &mut HashSet<String>,
) -> Result<Vec<String>, String> {
    if components.is_empty() {
        return Err(format!("{label} must name at least one component."));
    }
    for component in &components {
        if component.trim().is_empty() {
            return Err(format!("{label} component names must not be empty."));
        }
        if !assigned_components.insert(component.clone()) {
            return Err(format!(
                "Motion component may be assigned to only one joint: '{component}'."
            ));
        }
    }
    Ok(components)
}

fn require_finite_vector(label: &str, field: &str, vector: [f32; 3]) -> Result<(), String> {
    if vector.into_iter().all(f32::is_finite) {
        Ok(())
    } else {
        Err(format!("{label} {field} must contain only finite values."))
    }
}

fn normalized_axis(label: &str, axis: [f32; 3]) -> Result<[f32; 3], String> {
    require_finite_vector(label, "axis", axis)?;
    let largest = axis.into_iter().map(f32::abs).fold(0.0, f32::max);
    if largest <= f32::EPSILON {
        return Err(format!("{label} axis must be non-zero."));
    }
    let scaled = [axis[0] / largest, axis[1] / largest, axis[2] / largest];
    let length = (scaled[0] * scaled[0] + scaled[1] * scaled[1] + scaled[2] * scaled[2]).sqrt();
    Ok([scaled[0] / length, scaled[1] / length, scaled[2] / length])
}

fn append_components(signature: &mut String, components: &[String]) {
    for component in components {
        signature.push_str(&component.len().to_string());
        signature.push(':');
        signature.push_str(component);
        signature.push('|');
    }
}

fn append_f32s(signature: &mut String, values: &[f32]) {
    for value in values {
        signature.push_str(&format!("{:08x}|", value.to_bits()));
    }
}

fn is_step_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            extension.eq_ignore_ascii_case("step") || extension.eq_ignore_ascii_case("stp")
        })
}

fn resolve_project_path(base: &Path, relative: &str, label: &str) -> Result<PathBuf, String> {
    let relative_path = Path::new(relative);
    if relative.trim().is_empty()
        || relative_path.is_absolute()
        || relative_path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(format!(
            "{label} must be a relative path without '..': {relative}"
        ));
    }
    let path = base.join(relative_path);
    let canonical = path
        .canonicalize()
        .map_err(|error| format!("{label} does not exist: {relative} ({error})"))?;
    if !canonical.starts_with(base) {
        return Err(format!("{label} must remain inside {}.", base.display()));
    }
    Ok(canonical)
}

fn relative_portable_path(root: &Path, path: &Path) -> Option<String> {
    let relative = path.strip_prefix(root).ok()?;
    if relative.as_os_str().is_empty() {
        return Some(".".to_string());
    }
    relative
        .components()
        .map(|component| match component {
            Component::Normal(part) => part.to_str().map(ToOwned::to_owned),
            _ => None,
        })
        .collect::<Option<Vec<_>>>()
        .map(|components| components.join("/"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn write_config(root: &Path, contents: &str) {
        fs::create_dir_all(root.join(".burr")).unwrap();
        fs::write(root.join(".burr/config.toml"), contents).unwrap();
    }

    #[test]
    fn missing_config_uses_requested_directory() {
        let temp = tempdir().unwrap();
        let project = Project::discover(temp.path()).unwrap();
        assert!(!project.is_configured());
        assert_eq!(
            project.model_roots(),
            &[temp.path().canonicalize().unwrap()]
        );
    }

    #[test]
    fn discovers_parent_config_and_resolves_model_scope() {
        let temp = tempdir().unwrap();
        fs::create_dir_all(temp.path().join("models/subdirectory")).unwrap();
        write_config(
            temp.path(),
            "schema_version = \"burr.project.v2\"\n\n[project]\nmodels = [\"models\"]\n",
        );

        let project = Project::discover(&temp.path().join("models/subdirectory")).unwrap();
        assert_eq!(project.root(), temp.path().canonicalize().unwrap());

        let state = project.public_state();
        assert_eq!(state["configured"], true);
        assert_eq!(state["config_path"], ".burr/config.toml");
        assert_eq!(state["model_paths"][0], "models");
    }

    #[test]
    fn resolves_single_model_motion_inside_model_scope() {
        let temp = tempdir().unwrap();
        fs::create_dir_all(temp.path().join("models")).unwrap();
        fs::write(temp.path().join("models/deployed.step"), "STEP").unwrap();
        write_config(
            temp.path(),
            r#"schema_version = "burr.project.v2"

[project]
models = ["models"]

[[motions]]
id = "fold"
label = "Fold"
model = "models/deployed.step"
from_label = "Deployed"
to_label = "Folded"
duration_ms = 1200

[[motions.joints]]
type = "revolute"
components = ["left-arm", "left-pin"]
origin_mm = [10.0, 20.0, 30.0]
axis = [0.0, 0.0, 2.0]
angle_degrees = 95.0

[[motions.joints]]
type = "prismatic"
components = ["lock-bar"]
axis = [0.0, 1.0, 0.0]
distance_mm = 7.5
"#,
        );

        let project = Project::discover(temp.path()).unwrap();
        assert_eq!(project.motions.len(), 1);
        assert_eq!(
            project.motion("fold"),
            Some(&Motion {
                id: "fold".to_string(),
                label: "Fold".to_string(),
                model: "models/deployed.step".to_string(),
                from_label: "Deployed".to_string(),
                to_label: "Folded".to_string(),
                duration_ms: 1200,
                joints: vec![
                    MotionJoint::Revolute {
                        components: vec!["left-arm".to_string(), "left-pin".to_string()],
                        origin_mm: [10.0, 20.0, 30.0],
                        axis: [0.0, 0.0, 1.0],
                        angle_degrees: 95.0,
                    },
                    MotionJoint::Prismatic {
                        components: vec!["lock-bar".to_string()],
                        axis: [0.0, 1.0, 0.0],
                        distance_mm: 7.5,
                    },
                ],
            })
        );
        assert_eq!(project.public_state()["motions"][0]["id"], "fold");
        assert_eq!(
            project.public_state()["schema_version"],
            "burr.project-state.v2"
        );
    }

    #[test]
    fn motion_model_outside_model_scope_is_rejected() {
        let temp = tempdir().unwrap();
        fs::create_dir_all(temp.path().join("models")).unwrap();
        fs::create_dir_all(temp.path().join("exports")).unwrap();
        fs::write(temp.path().join("exports/deployed.step"), "STEP").unwrap();
        write_config(
            temp.path(),
            r#"schema_version = "burr.project.v2"

[project]
models = ["models"]

[[motions]]
id = "fold"
label = "Fold"
model = "exports/deployed.step"
from_label = "Deployed"
to_label = "Folded"
duration_ms = 1200

[[motions.joints]]
type = "revolute"
components = ["arm"]
origin_mm = [0.0, 0.0, 0.0]
axis = [0.0, 0.0, 1.0]
angle_degrees = 90.0
"#,
        );

        assert!(Project::discover(temp.path())
            .unwrap_err()
            .contains("configured model scope"));
    }

    #[test]
    fn malformed_config_is_rejected() {
        let temp = tempdir().unwrap();
        fs::create_dir_all(temp.path().join("models")).unwrap();
        write_config(
            temp.path(),
            "schema_version = \"burr.project.v2\"\n[project\nmodels = [\"models\"]\n",
        );
        assert!(Project::discover(temp.path())
            .unwrap_err()
            .contains("Invalid"));
    }

    #[test]
    fn unknown_configuration_fields_are_rejected() {
        let temp = tempdir().unwrap();
        fs::create_dir_all(temp.path().join("models")).unwrap();
        write_config(
            temp.path(),
            "schema_version = \"burr.project.v2\"\n[project]\nmodels = [\"models\"]\nunexpected = true\n",
        );
        let error = Project::discover(temp.path()).unwrap_err();
        assert!(error.contains("unknown field `unexpected`"));
    }

    #[test]
    fn model_paths_cannot_escape_the_project() {
        let workspace = tempdir().unwrap();
        let root = workspace.path().join("project");
        fs::create_dir_all(root.join("models")).unwrap();
        write_config(
            &root,
            "schema_version = \"burr.project.v2\"\n[project]\nmodels = [\"../outside\"]\n",
        );
        assert!(Project::discover(&root)
            .unwrap_err()
            .contains("relative path without '..'"));
    }

    #[test]
    fn old_two_pose_schema_is_rejected_before_motion_decoding() {
        let temp = tempdir().unwrap();
        fs::create_dir_all(temp.path().join("models")).unwrap();
        write_config(
            temp.path(),
            "schema_version = \"burr.project.v1\"\n[project]\nmodels = [\"models\"]\n",
        );

        let error = Project::discover(temp.path()).unwrap_err();
        assert!(error.contains("Unsupported Burr project schema 'burr.project.v1'"));
        assert!(error.contains("burr.project.v2"));
    }

    #[test]
    fn motion_rejects_duplicate_component_assignments() {
        let temp = tempdir().unwrap();
        fs::create_dir_all(temp.path().join("models")).unwrap();
        fs::write(temp.path().join("models/assembly.step"), "STEP").unwrap();
        write_config(
            temp.path(),
            r#"schema_version = "burr.project.v2"
[project]
models = ["models"]
[[motions]]
id = "fold"
label = "Fold"
model = "models/assembly.step"
from_label = "Open"
to_label = "Closed"
duration_ms = 500
[[motions.joints]]
type = "revolute"
components = ["arm"]
origin_mm = [0.0, 0.0, 0.0]
axis = [0.0, 0.0, 1.0]
angle_degrees = 90.0
[[motions.joints]]
type = "prismatic"
components = ["arm"]
axis = [1.0, 0.0, 0.0]
distance_mm = 2.0
"#,
        );

        assert!(Project::discover(temp.path())
            .unwrap_err()
            .contains("assigned to only one joint"));
    }

    #[test]
    fn motion_rejects_zero_axis() {
        let temp = tempdir().unwrap();
        fs::create_dir_all(temp.path().join("models")).unwrap();
        fs::write(temp.path().join("models/assembly.step"), "STEP").unwrap();
        write_config(
            temp.path(),
            r#"schema_version = "burr.project.v2"
[project]
models = ["models"]
[[motions]]
id = "fold"
label = "Fold"
model = "models/assembly.step"
from_label = "Open"
to_label = "Closed"
duration_ms = 500
[[motions.joints]]
type = "revolute"
components = ["arm"]
origin_mm = [0.0, 0.0, 0.0]
axis = [0.0, 0.0, 0.0]
angle_degrees = 90.0
"#,
        );

        assert!(Project::discover(temp.path())
            .unwrap_err()
            .contains("axis must be non-zero"));
    }

    #[test]
    fn motion_cache_signature_changes_with_joint_parameters() {
        let mut motion = Motion {
            id: "fold".to_string(),
            label: "Fold".to_string(),
            model: "models/assembly.step".to_string(),
            from_label: "Open".to_string(),
            to_label: "Closed".to_string(),
            duration_ms: 500,
            joints: vec![MotionJoint::Revolute {
                components: vec!["arm".to_string()],
                origin_mm: [0.0, 0.0, 0.0],
                axis: [0.0, 0.0, 1.0],
                angle_degrees: 90.0,
            }],
        };
        let before = motion.cache_signature();
        let MotionJoint::Revolute { angle_degrees, .. } = &mut motion.joints[0] else {
            panic!("fixture joint was not revolute");
        };
        *angle_degrees = 45.0;

        assert_ne!(before, motion.cache_signature());
    }
}
