use look::{
    config::{LightingConfig, UpAxis},
    scene::{compile_scene, prepare_source_textures},
    timing::Timings,
    ui::generate_html_viewer,
};
use percent_encoding::percent_decode_str;
use serde_json::json;
use std::{
    collections::HashMap,
    fs,
    path::{Component, Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};
use tiny_http::{Header, Method, Request, Response, Server, StatusCode};

const SHELL_HTML: &str = include_str!("viewer_shell.html");
const LOGO_PNG: &[u8] = include_bytes!("../assets/burr-logo.png");
const VIEWER_FRAMING_MARGIN: f32 = 1.3;
const SKIP_DIRECTORIES: [&str; 9] = [
    ".git",
    ".jj",
    ".next",
    "__pycache__",
    "build",
    "dist",
    "node_modules",
    "target",
    "venv",
];

#[derive(Clone, Debug, PartialEq, Eq)]
struct ModelFile {
    relative_path: String,
    name: String,
    format: &'static str,
    version: String,
}

#[derive(Clone)]
struct CachedViewer {
    version: String,
    html: String,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
enum ViewerTheme {
    Dark,
    Light,
}

impl ViewerTheme {
    fn from_query(value: Option<&str>) -> Self {
        match value {
            Some("light") => Self::Light,
            _ => Self::Dark,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Dark => "dark",
            Self::Light => "light",
        }
    }

    fn canvas_background(self) -> &'static str {
        match self {
            Self::Dark => "#0c0d10",
            Self::Light => "#c9ced0",
        }
    }

    fn lighting(self) -> LightingConfig {
        let mut lighting = LightingConfig::default();
        if self == Self::Light {
            lighting.ambient = 0.22;
            lighting.intensity = 0.95;
        }
        lighting
    }

    fn viewer_css(self) -> &'static str {
        match self {
            Self::Dark => DARK_VIEWER_THEME_CSS,
            Self::Light => LIGHT_VIEWER_THEME_CSS,
        }
    }
}

const DARK_VIEWER_THEME_CSS: &str = r#"
body { background-color: #0c0d10; color: #f3f4f5; }
.header-bar, .toolbar, .legend {
  background: rgba(17, 18, 21, 0.88);
  border-color: #2a2d33;
  box-shadow: 0 12px 36px rgba(0, 0, 0, 0.42);
}
.model-title { color: #f3f4f5; }
.format-badge { background: #405e6d; color: #f7fbfd; }
.stat-item, .btn { color: #929aa1; }
.stat-value, .legend kbd { color: #c8cdd1; }
.btn:hover, .legend kbd { background: #24272c; }
.legend { color: #747c83; }
"#;

const LIGHT_VIEWER_THEME_CSS: &str = r#"
body { background-color: #c9ced0; color: #1b1d1f; }
.header-bar, .toolbar, .legend {
  background: rgba(250, 249, 246, 0.9);
  border-color: #c8c7c1;
  box-shadow: 0 12px 32px rgba(31, 35, 38, 0.16);
}
.model-title { color: #1b1d1f; }
.format-badge { background: #4f6e7d; color: #ffffff; }
.stat-item, .btn { color: #687077; }
.stat-value, .legend kbd { color: #34383c; }
.btn:hover, .legend kbd { background: #e7e5df; color: #17191b; }
.legend { color: #6b7277; }
"#;

pub fn run(root: PathBuf) -> Result<(), String> {
    let root = root
        .canonicalize()
        .map_err(|error| format!("Failed to open viewer root {}: {error}", root.display()))?;
    if !root.is_dir() {
        return Err(format!(
            "Viewer root is not a directory: {}",
            root.display()
        ));
    }

    let requested_port = std::env::var("BURR_VIEWER_PORT")
        .ok()
        .map(|value| {
            value
                .parse::<u16>()
                .map_err(|_| "BURR_VIEWER_PORT must be a number from 0 to 65535.".to_string())
        })
        .transpose()?
        .unwrap_or(0);
    let server = Server::http(("127.0.0.1", requested_port))
        .map_err(|error| format!("Failed to start Burr viewer: {error}"))?;
    let address = server
        .server_addr()
        .to_ip()
        .ok_or_else(|| "Burr viewer did not receive an IP address.".to_string())?;
    let url = format!("http://127.0.0.1:{}/", address.port());

    println!("BURR VIEWER {}", root.display());
    println!("OPEN {url}");
    println!("Watching STEP, STL, and GLB files. Press Ctrl-C to stop.");

    if std::env::var_os("BURR_VIEWER_NO_OPEN").is_none() {
        if let Err(error) = open_browser(&url) {
            eprintln!("burr: could not open the browser automatically: {error}");
            eprintln!("burr: open {url} manually");
        }
    }

    let mut cache = HashMap::new();
    for request in server.incoming_requests() {
        if let Err(error) = handle_request(request, &root, &mut cache) {
            eprintln!("Viewer request failed: {error}");
        }
    }
    Ok(())
}

fn handle_request(
    request: Request,
    root: &Path,
    cache: &mut HashMap<(PathBuf, ViewerTheme), CachedViewer>,
) -> Result<(), String> {
    if request.method() != &Method::Get {
        return respond(
            request,
            405,
            "text/plain; charset=utf-8",
            "Method not allowed".to_string(),
        );
    }

    let url = request.url().to_string();
    let route = url.split('?').next().unwrap_or("/");
    match route {
        "/" => respond(
            request,
            200,
            "text/html; charset=utf-8",
            SHELL_HTML.to_string(),
        ),
        "/api/health" => respond(
            request,
            200,
            "application/json; charset=utf-8",
            json!({ "status": "ok" }).to_string(),
        ),
        "/api/tree" => match scan_models(root) {
            Ok(files) => {
                let root_name = root
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("models");
                let files = files
                    .iter()
                    .map(|file| {
                        json!({
                            "path": file.relative_path,
                            "name": file.name,
                            "format": file.format,
                            "version": file.version,
                        })
                    })
                    .collect::<Vec<_>>();
                respond(
                    request,
                    200,
                    "application/json; charset=utf-8",
                    json!({ "root": root_name, "files": files }).to_string(),
                )
            }
            Err(error) => respond_json_error(request, 500, &error),
        },
        "/assets/burr-logo.png" | "/favicon.ico" => {
            respond_bytes(request, 200, "image/png", LOGO_PNG.to_vec())
        }
        "/viewer" => {
            let Some(relative_path) = query_value(&url, "path") else {
                return respond_html_error(request, 400, "No model path was provided.");
            };
            let theme = ViewerTheme::from_query(query_value(&url, "theme").as_deref());
            match render_model(root, &relative_path, theme, cache) {
                Ok(html) => respond(request, 200, "text/html; charset=utf-8", html),
                Err(error) => respond_html_error(request, 422, &error),
            }
        }
        _ => respond(
            request,
            404,
            "text/plain; charset=utf-8",
            "Not found".to_string(),
        ),
    }
}

fn render_model(
    root: &Path,
    relative_path: &str,
    theme: ViewerTheme,
    cache: &mut HashMap<(PathBuf, ViewerTheme), CachedViewer>,
) -> Result<String, String> {
    let path = resolve_model_path(root, relative_path)?;
    let version = file_version(&path)?;
    let cache_key = (path.clone(), theme);
    if let Some(cached) = cache.get(&cache_key) {
        if cached.version == version {
            return Ok(cached.html.clone());
        }
    }

    let mut timings = Timings::default();
    let up_axis = if model_format(&path) == Some("GLB") {
        UpAxis::Y
    } else {
        UpAxis::Z
    };
    let mut scene = compile_scene(&path, up_axis, &mut timings)
        .map_err(|error| format!("Look could not render {relative_path}: {error:#}"))?;
    prepare_source_textures(&mut scene, &mut timings)
        .map_err(|error| format!("Look could not prepare {relative_path}: {error:#}"))?;
    // Look's self-contained viewer fits directly to the scene sphere. Leave a
    // little breathing room for the shorter iframe viewport created by Burr's
    // navigation shell and for models whose diagonal approaches that sphere.
    scene.fit_radius *= VIEWER_FRAMING_MARGIN;
    let lighting = theme.lighting();
    let html = generate_html_viewer(
        &scene,
        relative_path,
        &scene.statistics,
        &lighting,
        theme.canvas_background(),
    )
    .map_err(|error| format!("Look could not build the viewer for {relative_path}: {error:#}"))?;
    let html = inject_viewer_theme(html, theme)?;

    cache.insert(
        cache_key,
        CachedViewer {
            version,
            html: html.clone(),
        },
    );
    Ok(html)
}

fn inject_viewer_theme(html: String, theme: ViewerTheme) -> Result<String, String> {
    let marker = "</head>";
    let Some(index) = html.find(marker) else {
        return Err("Look viewer HTML did not contain a head element.".to_string());
    };
    let theme_style = format!(
        "<style id=\"burr-viewer-theme\" data-burr-theme=\"{}\">{}</style>",
        theme.name(),
        theme.viewer_css()
    );
    let mut themed = String::with_capacity(html.len() + theme_style.len());
    themed.push_str(&html[..index]);
    themed.push_str(&theme_style);
    themed.push_str(&html[index..]);
    Ok(themed)
}

fn scan_models(root: &Path) -> Result<Vec<ModelFile>, String> {
    let mut files = Vec::new();
    collect_models(root, root, &mut files)?;
    files.sort_by(|left, right| {
        left.relative_path
            .to_ascii_lowercase()
            .cmp(&right.relative_path.to_ascii_lowercase())
            .then_with(|| left.relative_path.cmp(&right.relative_path))
    });
    Ok(files)
}

fn collect_models(root: &Path, directory: &Path, files: &mut Vec<ModelFile>) -> Result<(), String> {
    let entries = fs::read_dir(directory)
        .map_err(|error| format!("Failed to scan {}: {error}", directory.display()))?;
    let mut entries = entries
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("Failed to scan {}: {error}", directory.display()))?;
    entries.sort_by_key(|entry| entry.file_name().to_string_lossy().to_ascii_lowercase());

    for entry in entries {
        let file_type = entry
            .file_type()
            .map_err(|error| format!("Failed to inspect {}: {error}", entry.path().display()))?;
        if file_type.is_symlink() {
            continue;
        }
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with('.') {
            continue;
        }
        if file_type.is_dir() {
            if SKIP_DIRECTORIES.contains(&name.as_ref()) {
                continue;
            }
            collect_models(root, &entry.path(), files)?;
            continue;
        }
        if !file_type.is_file() {
            continue;
        }
        let path = entry.path();
        let Some(format) = model_format(&path) else {
            continue;
        };
        let relative = path
            .strip_prefix(root)
            .map_err(|_| format!("Model escaped the viewer root: {}", path.display()))?;
        let Some(relative_path) = portable_path(relative) else {
            continue;
        };
        let Some(name) = path
            .file_name()
            .and_then(|name| name.to_str())
            .map(ToOwned::to_owned)
        else {
            continue;
        };
        files.push(ModelFile {
            relative_path,
            name,
            format,
            version: file_version(&path)?,
        });
    }
    Ok(())
}

fn resolve_model_path(root: &Path, relative_path: &str) -> Result<PathBuf, String> {
    let relative = Path::new(relative_path);
    if relative.as_os_str().is_empty()
        || relative.is_absolute()
        || relative.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err("Model path must remain inside the viewer root.".to_string());
    }
    let candidate = root.join(relative);
    let canonical = candidate
        .canonicalize()
        .map_err(|error| format!("Model does not exist: {relative_path} ({error})"))?;
    if !canonical.starts_with(root) {
        return Err("Model path must remain inside the viewer root.".to_string());
    }
    if !canonical.is_file() || model_format(&canonical).is_none() {
        return Err(format!("Unsupported model file: {relative_path}"));
    }
    Ok(canonical)
}

fn model_format(path: &Path) -> Option<&'static str> {
    match path
        .extension()
        .and_then(|extension| extension.to_str())?
        .to_ascii_lowercase()
        .as_str()
    {
        "step" | "stp" => Some("STEP"),
        "stl" => Some("STL"),
        "glb" => Some("GLB"),
        _ => None,
    }
}

fn portable_path(path: &Path) -> Option<String> {
    path.components()
        .map(|component| match component {
            Component::Normal(part) => part.to_str().map(ToOwned::to_owned),
            _ => None,
        })
        .collect::<Option<Vec<_>>>()
        .map(|components| components.join("/"))
}

fn file_version(path: &Path) -> Result<String, String> {
    let metadata = fs::metadata(path)
        .map_err(|error| format!("Failed to inspect {}: {error}", path.display()))?;
    let modified = metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH);
    let duration = modified.duration_since(UNIX_EPOCH).unwrap_or_default();
    Ok(format!(
        "{}-{}-{}",
        metadata.len(),
        duration.as_secs(),
        duration.subsec_nanos()
    ))
}

fn query_value(url: &str, wanted: &str) -> Option<String> {
    let query = url.split_once('?')?.1;
    query.split('&').find_map(|pair| {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        (key == wanted).then(|| {
            percent_decode_str(value)
                .decode_utf8()
                .ok()
                .map(|value| value.into_owned())
        })?
    })
}

fn respond(request: Request, status: u16, content_type: &str, body: String) -> Result<(), String> {
    respond_bytes(request, status, content_type, body.into_bytes())
}

fn respond_bytes(
    request: Request,
    status: u16,
    content_type: &str,
    body: Vec<u8>,
) -> Result<(), String> {
    let content_type = Header::from_bytes("Content-Type", content_type)
        .map_err(|_| "Failed to create Content-Type header.".to_string())?;
    let cache_control = Header::from_bytes("Cache-Control", "no-store")
        .map_err(|_| "Failed to create Cache-Control header.".to_string())?;
    request
        .respond(
            Response::from_data(body)
                .with_status_code(StatusCode(status))
                .with_header(content_type)
                .with_header(cache_control),
        )
        .map_err(|error| format!("Failed to send viewer response: {error}"))
}

fn respond_json_error(request: Request, status: u16, message: &str) -> Result<(), String> {
    respond(
        request,
        status,
        "application/json; charset=utf-8",
        json!({ "error": message }).to_string(),
    )
}

fn respond_html_error(request: Request, status: u16, message: &str) -> Result<(), String> {
    let message = escape_html(message);
    respond(
        request,
        status,
        "text/html; charset=utf-8",
        format!(
            "<!doctype html><html><head><meta charset=\"utf-8\"><meta name=\"color-scheme\" content=\"dark\"><style>body{{min-height:100vh;margin:0;display:grid;place-items:center;background:#111418;color:#eef1f3;font:14px system-ui}}main{{max-width:560px;padding:24px}}p{{color:#9aa4aa;line-height:1.6}}</style></head><body><main><h1>Could not open model</h1><p>{message}</p></main></body></html>"
        ),
    )
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn open_browser(url: &str) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    let mut command = {
        let mut command = Command::new("open");
        command.arg(url);
        command
    };
    #[cfg(target_os = "windows")]
    let mut command = {
        let mut command = Command::new("cmd");
        command.args(["/C", "start", "", url]);
        command
    };
    #[cfg(all(unix, not(target_os = "macos")))]
    let mut command = {
        let mut command = Command::new("xdg-open");
        command.arg(url);
        command
    };
    command
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("Failed to open the browser at {url}: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn scan_models_filters_and_sorts_supported_files() {
        let temp = tempdir().unwrap();
        fs::create_dir_all(temp.path().join("models/enclosure")).unwrap();
        fs::create_dir_all(temp.path().join("node_modules/ignored")).unwrap();
        fs::write(temp.path().join("models/zeta.stl"), "solid empty\nendsolid").unwrap();
        fs::write(temp.path().join("models/enclosure/alpha.STEP"), "STEP").unwrap();
        fs::write(temp.path().join("models/readme.txt"), "ignore me").unwrap();
        fs::write(
            temp.path().join("node_modules/ignored/hidden.glb"),
            "ignore me",
        )
        .unwrap();

        let models = scan_models(temp.path()).unwrap();
        assert_eq!(models.len(), 2);
        assert_eq!(models[0].relative_path, "models/enclosure/alpha.STEP");
        assert_eq!(models[0].format, "STEP");
        assert_eq!(models[1].relative_path, "models/zeta.stl");
        assert_eq!(models[1].format, "STL");
    }

    #[test]
    fn resolve_model_path_rejects_escape_and_unsupported_files() {
        let temp = tempdir().unwrap();
        fs::write(temp.path().join("part.step"), "STEP").unwrap();
        fs::write(temp.path().join("notes.txt"), "notes").unwrap();
        let root = temp.path().canonicalize().unwrap();

        assert_eq!(
            resolve_model_path(&root, "part.step").unwrap(),
            root.join("part.step")
        );
        assert!(resolve_model_path(&root, "../outside.step").is_err());
        assert!(resolve_model_path(&root, "notes.txt").is_err());
    }

    #[test]
    fn file_version_changes_when_model_changes() {
        let temp = tempdir().unwrap();
        let path = temp.path().join("part.stl");
        fs::write(&path, "solid empty\nendsolid").unwrap();
        let before = file_version(&path).unwrap();
        let mut file = fs::OpenOptions::new().append(true).open(&path).unwrap();
        writeln!(file, "changed").unwrap();
        let after = file_version(&path).unwrap();
        assert_ne!(before, after);
    }

    #[test]
    fn query_values_are_percent_decoded() {
        assert_eq!(
            query_value("/viewer?path=models%2Fmotor%20mount.step&v=1", "path").as_deref(),
            Some("models/motor mount.step")
        );
    }

    #[test]
    fn viewer_theme_defaults_dark_and_accepts_light() {
        assert_eq!(ViewerTheme::from_query(None), ViewerTheme::Dark);
        assert_eq!(ViewerTheme::from_query(Some("unknown")), ViewerTheme::Dark);
        assert_eq!(ViewerTheme::from_query(Some("light")), ViewerTheme::Light);
    }

    #[test]
    fn viewer_theme_is_injected_into_look_html() {
        let html = "<!doctype html><html><head></head><body></body></html>".to_string();
        let themed = inject_viewer_theme(html, ViewerTheme::Light).unwrap();
        assert!(themed.contains("data-burr-theme=\"light\""));
        assert!(themed.contains("background-color: #c9ced0"));
        assert!(themed.contains("</style></head>"));
    }
}
