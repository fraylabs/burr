use serde::Deserialize;
use serde_json::{json, Value};
use std::{
    fs,
    path::{Component, Path, PathBuf},
};

pub const PROJECT_CONFIG_SCHEMA_VERSION: &str = "burr.project.v1";

const CONFIG_DIRECTORY: &str = ".burr";
const CONFIG_FILE: &str = "config.toml";

#[derive(Clone, Debug)]
pub struct Project {
    root: PathBuf,
    model_roots: Vec<PathBuf>,
    config_path: Option<PathBuf>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProjectConfig {
    schema_version: String,
    project: ProjectSection,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProjectSection {
    models: Vec<String>,
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

        Ok(Self {
            root,
            model_roots,
            config_path: Some(config_path),
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn model_roots(&self) -> &[PathBuf] {
        &self.model_roots
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
            "schema_version": "burr.project-state.v1",
            "root": root_name,
            "configured": self.is_configured(),
            "config_path": self.config_path.as_ref().and_then(|path| relative_portable_path(&self.root, path)),
            "model_paths": model_paths,
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
            "schema_version = \"burr.project.v1\"\n\n[project]\nmodels = [\"models\"]\n",
        );

        let project = Project::discover(&temp.path().join("models/subdirectory")).unwrap();
        assert_eq!(project.root(), temp.path().canonicalize().unwrap());

        let state = project.public_state();
        assert_eq!(state["configured"], true);
        assert_eq!(state["config_path"], ".burr/config.toml");
        assert_eq!(state["model_paths"][0], "models");
        assert!(state.get("packs").is_none());
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
    fn check_configuration_is_not_part_of_the_model_scope_contract() {
        let temp = tempdir().unwrap();
        fs::create_dir_all(temp.path().join("models")).unwrap();
        write_config(
            temp.path(),
            "schema_version = \"burr.project.v1\"\n[project]\nmodels = [\"models\"]\n[[packs]]\nid = \"builtin:mechanical-fit\"\n",
        );
        let error = Project::discover(temp.path()).unwrap_err();
        assert!(error.contains("unknown field `packs`"));
    }

    #[test]
    fn model_paths_cannot_escape_the_project() {
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
    }
}
