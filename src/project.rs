use serde::Deserialize;
use serde_json::{json, Value};
use std::{
    collections::HashSet,
    fs,
    path::{Component, Path, PathBuf},
};

pub const PROJECT_CONFIG_SCHEMA_VERSION: &str = "burr.project.v1";
pub const LOCAL_PACK_SCHEMA_VERSION: &str = "burr.pack.v1";

const CONFIG_DIRECTORY: &str = ".burr";
const CONFIG_FILE: &str = "config.toml";

#[derive(Clone, Debug)]
pub struct Project {
    root: PathBuf,
    model_roots: Vec<PathBuf>,
    config_path: Option<PathBuf>,
    packs: Vec<ResolvedPack>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedPack {
    pub id: String,
    pub version: String,
    pub source: PackSource,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PackSource {
    Builtin,
    Local(PathBuf),
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProjectConfig {
    schema_version: String,
    project: ProjectSection,
    #[serde(default)]
    packs: Vec<PackSelection>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProjectSection {
    models: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PackSelection {
    id: Option<String>,
    path: Option<String>,
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
                config_path: None,
                packs: Vec::new(),
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
        let packs = resolve_packs(&root, &config_directory, &config.packs)?;

        Ok(Self {
            root,
            model_roots,
            config_path: Some(config_path),
            packs,
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn model_roots(&self) -> &[PathBuf] {
        &self.model_roots
    }

    pub fn packs(&self) -> &[ResolvedPack] {
        &self.packs
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
        let packs = self
            .packs
            .iter()
            .map(|pack| match &pack.source {
                PackSource::Builtin => json!({
                    "id": pack.id,
                    "version": pack.version,
                    "source": "builtin",
                }),
                PackSource::Local(path) => json!({
                    "id": pack.id,
                    "version": pack.version,
                    "source": "local",
                    "path": relative_portable_path(&self.root, path),
                }),
            })
            .collect::<Vec<_>>();
        json!({
            "schema_version": "burr.project-state.v1",
            "root": root_name,
            "configured": self.is_configured(),
            "config_path": self.config_path.as_ref().and_then(|path| relative_portable_path(&self.root, path)),
            "model_paths": model_paths,
            "packs": packs,
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
    let config = toml::from_str::<ProjectConfig>(&text)
        .map_err(|error| format!("Invalid {}: {error}", path.display()))?;
    if config.schema_version != PROJECT_CONFIG_SCHEMA_VERSION {
        return Err(format!(
            "Unsupported Burr project schema '{}'; expected '{PROJECT_CONFIG_SCHEMA_VERSION}'.",
            config.schema_version
        ));
    }
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

fn resolve_packs(
    root: &Path,
    config_directory: &Path,
    configured: &[PackSelection],
) -> Result<Vec<ResolvedPack>, String> {
    let mut packs = Vec::with_capacity(configured.len());
    let mut ids = HashSet::new();
    for selection in configured {
        let pack = match (&selection.id, &selection.path) {
            (Some(id), None) => resolve_builtin_pack(id)?,
            (None, Some(relative)) => resolve_local_pack(root, config_directory, relative)?,
            (Some(_), Some(_)) => {
                return Err(
                    "Each [[packs]] entry must declare exactly one of id or path, not both."
                        .to_string(),
                )
            }
            (None, None) => {
                return Err(
                    "Each [[packs]] entry must declare exactly one of id or path.".to_string(),
                )
            }
        };
        if !ids.insert(pack.id.clone()) {
            return Err(format!("Duplicate Burr pack id: {}", pack.id));
        }
        packs.push(pack);
    }
    Ok(packs)
}

fn resolve_builtin_pack(id: &str) -> Result<ResolvedPack, String> {
    match id {
        "builtin:mechanical-fit" => Ok(ResolvedPack {
            id: id.to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            source: PackSource::Builtin,
        }),
        _ => Err(format!("Unknown built-in Burr pack: {id}")),
    }
}

fn resolve_local_pack(
    root: &Path,
    config_directory: &Path,
    relative: &str,
) -> Result<ResolvedPack, String> {
    let path = resolve_project_path(config_directory, relative, "Local pack path")?;
    if !path.starts_with(config_directory) || !path.starts_with(root) {
        return Err(
            "Local pack path must remain inside the project's .burr directory.".to_string(),
        );
    }
    if !path.is_file() {
        return Err(format!("Local Burr pack is not a file: {relative}"));
    }
    let text = fs::read_to_string(&path)
        .map_err(|error| format!("Failed to read local Burr pack {relative}: {error}"))?;
    let document = toml::from_str::<toml::Value>(&text)
        .map_err(|error| format!("Invalid local Burr pack {relative}: {error}"))?;
    let schema_version = required_string(&document, "schema_version", relative)?;
    if schema_version != LOCAL_PACK_SCHEMA_VERSION {
        return Err(format!(
            "Unsupported local pack schema '{schema_version}' in {relative}; expected '{LOCAL_PACK_SCHEMA_VERSION}'."
        ));
    }
    let id = required_string(&document, "id", relative)?;
    if !valid_pack_id(id) {
        return Err(format!(
            "Local Burr pack id '{id}' in {relative} must use lowercase letters, numbers, '.', '_', ':', or '-'."
        ));
    }
    let version = required_string(&document, "version", relative)?;
    Ok(ResolvedPack {
        id: id.to_string(),
        version: version.to_string(),
        source: PackSource::Local(path),
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

fn required_string<'a>(
    document: &'a toml::Value,
    key: &str,
    path: &str,
) -> Result<&'a str, String> {
    document
        .get(key)
        .and_then(toml::Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("Local Burr pack {path} requires a non-empty '{key}' string."))
}

fn valid_pack_id(id: &str) -> bool {
    !id.is_empty()
        && id.bytes().all(|byte| {
            byte.is_ascii_lowercase()
                || byte.is_ascii_digit()
                || matches!(byte, b'.' | b'_' | b':' | b'-')
        })
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
    fn missing_config_uses_requested_directory_without_packs() {
        let temp = tempdir().unwrap();
        let project = Project::discover(temp.path()).unwrap();
        assert!(!project.is_configured());
        assert!(project.packs().is_empty());
        assert_eq!(
            project.model_roots(),
            &[temp.path().canonicalize().unwrap()]
        );
    }

    #[test]
    fn discovers_parent_config_and_resolves_builtin_and_local_packs() {
        let temp = tempdir().unwrap();
        fs::create_dir_all(temp.path().join("models/subdirectory")).unwrap();
        fs::create_dir_all(temp.path().join(".burr/packs")).unwrap();
        fs::write(
            temp.path().join(".burr/packs/product-fit.toml"),
            "schema_version = \"burr.pack.v1\"\nid = \"project:product-fit\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        write_config(
            temp.path(),
            "schema_version = \"burr.project.v1\"\n\n[project]\nmodels = [\"models\"]\n\n[[packs]]\nid = \"builtin:mechanical-fit\"\n\n[[packs]]\npath = \"packs/product-fit.toml\"\n",
        );

        let project = Project::discover(&temp.path().join("models/subdirectory")).unwrap();
        assert_eq!(project.root(), temp.path().canonicalize().unwrap());
        assert_eq!(project.packs().len(), 2);
        assert_eq!(project.packs()[0].id, "builtin:mechanical-fit");
        assert_eq!(project.packs()[1].id, "project:product-fit");
        assert!(matches!(project.packs()[1].source, PackSource::Local(_)));

        let state = project.public_state();
        assert_eq!(state["configured"], true);
        assert_eq!(state["config_path"], ".burr/config.toml");
        assert_eq!(state["model_paths"][0], "models");
        assert_eq!(state["packs"][1]["path"], ".burr/packs/product-fit.toml");
    }

    #[test]
    fn malformed_config_is_rejected() {
        let temp = tempdir().unwrap();
        fs::create_dir_all(temp.path().join("models")).unwrap();
        write_config(
            temp.path(),
            "schema_version = \"burr.project.v1\"\n[project\nmodels = [\"models\"]\n",
        );
        assert!(Project::discover(temp.path())
            .unwrap_err()
            .contains("Invalid"));
    }

    #[test]
    fn unknown_builtin_and_duplicate_pack_ids_are_rejected() {
        let temp = tempdir().unwrap();
        fs::create_dir_all(temp.path().join("models")).unwrap();
        write_config(
            temp.path(),
            "schema_version = \"burr.project.v1\"\n[project]\nmodels = [\"models\"]\n[[packs]]\nid = \"builtin:not-real\"\n",
        );
        assert_eq!(
            Project::discover(temp.path()).unwrap_err(),
            "Unknown built-in Burr pack: builtin:not-real"
        );

        write_config(
            temp.path(),
            "schema_version = \"burr.project.v1\"\n[project]\nmodels = [\"models\"]\n[[packs]]\nid = \"builtin:mechanical-fit\"\n[[packs]]\nid = \"builtin:mechanical-fit\"\n",
        );
        assert_eq!(
            Project::discover(temp.path()).unwrap_err(),
            "Duplicate Burr pack id: builtin:mechanical-fit"
        );
    }

    #[test]
    fn model_and_local_pack_paths_cannot_escape_the_project() {
        let workspace = tempdir().unwrap();
        let root = workspace.path().join("project");
        fs::create_dir_all(root.join("models")).unwrap();
        write_config(
            &root,
            "schema_version = \"burr.project.v1\"\n[project]\nmodels = [\"../outside\"]\n",
        );
        assert!(Project::discover(&root)
            .unwrap_err()
            .contains("relative path without '..'"));

        write_config(
            &root,
            "schema_version = \"burr.project.v1\"\n[project]\nmodels = [\"models\"]\n[[packs]]\npath = \"../outside.toml\"\n",
        );
        assert!(Project::discover(&root)
            .unwrap_err()
            .contains("relative path without '..'"));
    }
}
