use crate::{
    interference::{self, CheckReport},
    project::Project,
};
use look::{
    config::{LightingConfig, UpAxis},
    scene::{compile_scene, prepare_source_textures, CompiledScene, SourceVertexAttributes},
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
const SKIP_DIRECTORIES: [&str; 10] = [
    ".git",
    ".jj",
    ".next",
    "__cadgen__",
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

struct CachedModel {
    version: String,
    scene: CompiledScene,
    report: CheckReport,
    viewers: HashMap<(ViewerTheme, Option<FocusPair>), String>,
}

type ModelCache = HashMap<PathBuf, CachedModel>;

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
struct FocusPair {
    first: usize,
    second: usize,
}

impl FocusPair {
    fn from_query(value: Option<&str>) -> Result<Option<Self>, String> {
        let Some(value) = value else {
            return Ok(None);
        };
        let Some((first, second)) = value.split_once(',') else {
            return Err("Viewer focus must contain two component indexes.".to_string());
        };
        let first = first
            .parse::<usize>()
            .map_err(|_| "Viewer focus contains an invalid component index.".to_string())?;
        let second = second
            .parse::<usize>()
            .map_err(|_| "Viewer focus contains an invalid component index.".to_string())?;
        if first == second {
            return Err("Viewer focus must contain two different components.".to_string());
        }
        Ok(Some(if first < second {
            Self { first, second }
        } else {
            Self {
                first: second,
                second: first,
            }
        }))
    }

    fn label(self) -> String {
        format!("{},{}", self.first, self.second)
    }
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

pub fn run(start: PathBuf) -> Result<(), String> {
    let project = Project::discover(&start)?;

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
    let port = address.port();
    let url = format!("http://127.0.0.1:{port}/");

    println!("BURR PROJECT {}", project.root().display());
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
        if let Err(error) = handle_request(request, &project, &mut cache, port) {
            eprintln!("Viewer request failed: {error}");
        }
    }
    Ok(())
}

fn handle_request(
    request: Request,
    project: &Project,
    cache: &mut ModelCache,
    expected_host_port: u16,
) -> Result<(), String> {
    if !request_host_is_loopback(&request, expected_host_port) {
        return respond(
            request,
            403,
            "text/plain; charset=utf-8",
            "Forbidden".to_string(),
        );
    }
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
        "/api/project" => respond(
            request,
            200,
            "application/json; charset=utf-8",
            project.public_state().to_string(),
        ),
        "/api/tree" => match scan_models(project) {
            Ok(files) => {
                let root_name = project
                    .root()
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
        "/api/checks" => {
            let Some(relative_path) = query_value(&url, "path") else {
                return respond_json_error(request, 400, "No model path was provided.");
            };
            match check_model(project, &relative_path, cache) {
                Ok(report) => respond(
                    request,
                    200,
                    "application/json; charset=utf-8",
                    serde_json::to_string(&report)
                        .map_err(|error| format!("Failed to encode check result: {error}"))?,
                ),
                Err(error) => respond_json_error(request, 422, &error),
            }
        }
        "/viewer" => {
            let Some(relative_path) = query_value(&url, "path") else {
                return respond_html_error(request, 400, "No model path was provided.");
            };
            let theme = ViewerTheme::from_query(query_value(&url, "theme").as_deref());
            let focus = match FocusPair::from_query(query_value(&url, "focus").as_deref()) {
                Ok(focus) => focus,
                Err(error) => return respond_html_error(request, 400, &error),
            };
            match render_model(project, &relative_path, theme, focus, cache) {
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
    project: &Project,
    relative_path: &str,
    theme: ViewerTheme,
    focus: Option<FocusPair>,
    cache: &mut ModelCache,
) -> Result<String, String> {
    let cached = load_model(project, relative_path, cache)?;
    if let Some(focus) = focus {
        if focus.second >= cached.scene.instances.len() {
            return Err("Viewer focus references a missing component.".to_string());
        }
    }
    let viewer_key = (theme, focus);
    if let Some(html) = cached.viewers.get(&viewer_key) {
        return Ok(html.clone());
    }

    let scene = match focus {
        Some(focus) => highlighted_scene(&cached.scene, focus)?,
        None => cached.scene.clone(),
    };
    let lighting = theme.lighting();
    let html = generate_html_viewer(
        &scene,
        relative_path,
        &scene.statistics,
        &lighting,
        theme.canvas_background(),
    )
    .map_err(|error| format!("Look could not build the viewer for {relative_path}: {error:#}"))?;
    let html = inject_viewer_render_modes(html)?;
    let html = inject_viewer_theme(html, theme, focus)?;
    cached.viewers.insert(viewer_key, html.clone());
    Ok(html)
}

fn check_model(
    project: &Project,
    relative_path: &str,
    cache: &mut ModelCache,
) -> Result<CheckReport, String> {
    load_model(project, relative_path, cache).map(|cached| cached.report.clone())
}

fn load_model<'a>(
    project: &Project,
    relative_path: &str,
    cache: &'a mut ModelCache,
) -> Result<&'a mut CachedModel, String> {
    let path = resolve_model_path(project, relative_path)?;
    let version = file_version(&path)?;
    let current = cache
        .get(&path)
        .is_some_and(|cached| cached.version == version);
    if current {
        return cache
            .get_mut(&path)
            .ok_or_else(|| "Viewer model cache became unavailable.".to_string());
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
    let report = if model_format(&path) == Some("STEP") {
        interference::analyze_scene(relative_path, &version, &scene)
    } else {
        CheckReport::unsupported(
            relative_path,
            &version,
            "Assembly interference currently supports STEP files only.",
        )
    };
    cache.insert(
        path.clone(),
        CachedModel {
            version,
            scene,
            report,
            viewers: HashMap::new(),
        },
    );
    cache
        .get_mut(&path)
        .ok_or_else(|| "Viewer model cache became unavailable.".to_string())
}

fn highlighted_scene(scene: &CompiledScene, focus: FocusPair) -> Result<CompiledScene, String> {
    const MUTED: [f32; 4] = [0.28, 0.31, 0.33, 1.0];
    const FIRST: [f32; 4] = [1.0, 0.34, 0.08, 1.0];
    const SECOND: [f32; 4] = [0.12, 0.76, 0.94, 1.0];

    let mut highlighted = scene.clone();
    highlighted.geometries = scene
        .instances
        .iter()
        .enumerate()
        .map(|(index, instance)| {
            let mut geometry = scene
                .geometries
                .get(instance.geometry)
                .cloned()
                .ok_or_else(|| {
                    format!(
                        "Component occurrence {index} references missing geometry {}.",
                        instance.geometry
                    )
                })?;
            let color = if index == focus.first {
                FIRST
            } else if index == focus.second {
                SECOND
            } else {
                MUTED
            };
            geometry.source_attributes = Some(
                geometry
                    .vertices
                    .iter()
                    .map(|_| SourceVertexAttributes {
                        tex_coord_0: [0.0; 2],
                        tex_coord_1: [0.0; 2],
                        color,
                    })
                    .collect(),
            );
            Ok(geometry)
        })
        .collect::<Result<Vec<_>, String>>()?;
    for (index, instance) in highlighted.instances.iter_mut().enumerate() {
        instance.geometry = index;
    }
    Ok(highlighted)
}

fn request_host_is_loopback(request: &Request, expected_port: u16) -> bool {
    request
        .headers()
        .iter()
        .find(|header| header.field.equiv("Host"))
        .is_some_and(|header| host_is_loopback(header.value.as_str(), expected_port))
}

fn host_is_loopback(host: &str, expected_port: u16) -> bool {
    host == format!("127.0.0.1:{expected_port}")
        || host.eq_ignore_ascii_case(&format!("localhost:{expected_port}"))
        || host == format!("[::1]:{expected_port}")
}

fn inject_viewer_theme(
    html: String,
    theme: ViewerTheme,
    focus: Option<FocusPair>,
) -> Result<String, String> {
    let marker = "</head>";
    let Some(index) = html.find(marker) else {
        return Err("Look viewer HTML did not contain a head element.".to_string());
    };
    let theme_style = format!(
        "<style id=\"burr-viewer-theme\" data-burr-theme=\"{}\">{}</style>",
        theme.name(),
        theme.viewer_css()
    );
    let focus_marker = focus.map_or_else(String::new, |focus| {
        format!(
            "<meta name=\"burr-highlighted-components\" content=\"{}\">",
            focus.label()
        )
    });
    let mut themed = String::with_capacity(html.len() + theme_style.len() + focus_marker.len());
    themed.push_str(&html[..index]);
    themed.push_str(&theme_style);
    themed.push_str(&focus_marker);
    themed.push_str(&html[index..]);
    Ok(themed)
}

fn inject_viewer_render_modes(mut html: String) -> Result<String, String> {
    let replacements = [
        (
            "uniform vec3 uCameraPos;\n            out vec4 fragColor;",
            "uniform vec3 uCameraPos;\n            uniform float uOpacity;\n            out vec4 fragColor;",
            "fragment opacity uniform",
        ),
        (
            "fragColor = vec4(col, 1.0);",
            "fragColor = vec4(col, uOpacity);",
            "fragment opacity output",
        ),
        (
            "const uCamPosLoc = gl.getUniformLocation(program, 'uCameraPos');",
            "const uCamPosLoc = gl.getUniformLocation(program, 'uCameraPos');\n        const uOpacityLoc = gl.getUniformLocation(program, 'uOpacity');",
            "opacity uniform location",
        ),
        (
            "gl.enable(gl.DEPTH_TEST);\n            gl.clearColor",
            "gl.enable(gl.DEPTH_TEST);\n            gl.depthMask(true);\n            gl.clearColor",
            "depth-buffer reset",
        ),
        (
            "gl.uniform3fv(uCamPosLoc, camPos);\n\n            gl.bindVertexArray(vao);",
            "gl.uniform3fv(uCamPosLoc, camPos);\n\n            const xRay = burrRenderMode === 'x-ray';\n            gl.uniform1f(uOpacityLoc, xRay ? 0.28 : 1.0);\n            if (xRay) {\n                gl.enable(gl.BLEND);\n                gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);\n                gl.depthMask(false);\n            } else {\n                gl.disable(gl.BLEND);\n                gl.depthMask(true);\n            }\n\n            gl.bindVertexArray(vao);",
            "render-mode state",
        ),
        (
            "gl.drawElements(gl.TRIANGLES, indices.length, gl.UNSIGNED_INT, 0);",
            "gl.drawElements(gl.TRIANGLES, indices.length, gl.UNSIGNED_INT, 0);\n            gl.depthMask(true);",
            "depth-buffer restore",
        ),
    ];
    for (needle, replacement, label) in replacements {
        if !html.contains(needle) {
            return Err(format!(
                "Look viewer HTML did not contain the expected {label} hook."
            ));
        }
        html = html.replacen(needle, replacement, 1);
    }

    let marker = "</head>";
    let Some(index) = html.find(marker) else {
        return Err("Look viewer HTML did not contain a head element.".to_string());
    };
    let controls = r#"<meta name="burr-render-modes" content="x-ray,solid"><script id="burr-render-mode">let burrRenderMode = "x-ray";document.documentElement.dataset.burrRenderMode = burrRenderMode;window.addEventListener("message",(event)=>{if(event.origin!==window.location.origin||event.data?.type!=="burr:set-render-mode")return;const mode=event.data.mode;if(mode!=="x-ray"&&mode!=="solid")return;burrRenderMode=mode;document.documentElement.dataset.burrRenderMode=mode;});</script>"#;
    html.insert_str(index, controls);
    Ok(html)
}

fn scan_models(project: &Project) -> Result<Vec<ModelFile>, String> {
    let mut files = Vec::new();
    for model_root in project.model_roots() {
        collect_models(project.root(), model_root, &mut files)?;
    }
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

fn resolve_model_path(project: &Project, relative_path: &str) -> Result<PathBuf, String> {
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
    let candidate = project.root().join(relative);
    let canonical = candidate
        .canonicalize()
        .map_err(|error| format!("Model does not exist: {relative_path} ({error})"))?;
    if !canonical.starts_with(project.root()) || !project.contains_model(&canonical) {
        return Err("Model path must remain inside a configured model path.".to_string());
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
        fs::create_dir_all(temp.path().join("models/__cadgen__/components")).unwrap();
        fs::create_dir_all(temp.path().join("node_modules/ignored")).unwrap();
        fs::write(temp.path().join("models/zeta.stl"), "solid empty\nendsolid").unwrap();
        fs::write(temp.path().join("models/enclosure/alpha.STEP"), "STEP").unwrap();
        fs::write(temp.path().join("models/readme.txt"), "ignore me").unwrap();
        fs::write(
            temp.path().join("models/__cadgen__/components/render.glb"),
            "generated render cache",
        )
        .unwrap();
        fs::write(
            temp.path().join("node_modules/ignored/hidden.glb"),
            "ignore me",
        )
        .unwrap();

        let project = Project::discover(temp.path()).unwrap();
        let models = scan_models(&project).unwrap();
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
        let project = Project::discover(temp.path()).unwrap();
        let root = project.root();

        assert_eq!(
            resolve_model_path(&project, "part.step").unwrap(),
            root.join("part.step")
        );
        assert!(resolve_model_path(&project, "../outside.step").is_err());
        assert!(resolve_model_path(&project, "notes.txt").is_err());
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
        let themed = inject_viewer_theme(
            html,
            ViewerTheme::Light,
            Some(FocusPair {
                first: 2,
                second: 5,
            }),
        )
        .unwrap();
        assert!(themed.contains("data-burr-theme=\"light\""));
        assert!(themed.contains("background-color: #c9ced0"));
        assert!(themed.contains("name=\"burr-highlighted-components\" content=\"2,5\""));
        assert!(themed.contains("</style><meta"));
    }

    #[test]
    fn viewer_render_modes_default_to_x_ray() {
        let html = r#"<!doctype html><html><head></head><body><script>
uniform vec3 uCameraPos;
            out vec4 fragColor;
fragColor = vec4(col, 1.0);
const uCamPosLoc = gl.getUniformLocation(program, 'uCameraPos');
gl.enable(gl.DEPTH_TEST);
            gl.clearColor
gl.uniform3fv(uCamPosLoc, camPos);

            gl.bindVertexArray(vao);
gl.drawElements(gl.TRIANGLES, indices.length, gl.UNSIGNED_INT, 0);
</script></body></html>"#
            .to_string();
        let rendered = inject_viewer_render_modes(html).unwrap();
        assert!(rendered.contains("name=\"burr-render-modes\" content=\"x-ray,solid\""));
        assert!(rendered.contains("let burrRenderMode = \"x-ray\""));
        assert!(rendered.contains("fragColor = vec4(col, uOpacity);"));
        assert!(rendered.contains("xRay ? 0.28 : 1.0"));
        assert!(rendered.contains("burr:set-render-mode"));
    }

    #[test]
    fn viewer_focus_is_sorted_and_rejects_invalid_pairs() {
        assert_eq!(
            FocusPair::from_query(Some("5,2")).unwrap(),
            Some(FocusPair {
                first: 2,
                second: 5,
            })
        );
        assert!(FocusPair::from_query(Some("2,2")).is_err());
        assert!(FocusPair::from_query(Some("two,5")).is_err());
    }

    #[test]
    fn host_guard_accepts_only_loopback_names_on_the_bound_port() {
        assert!(host_is_loopback("127.0.0.1:43120", 43120));
        assert!(host_is_loopback("localhost:43120", 43120));
        assert!(host_is_loopback("LOCALHOST:43120", 43120));
        assert!(host_is_loopback("[::1]:43120", 43120));
        assert!(!host_is_loopback("attacker.example:43120", 43120));
        assert!(!host_is_loopback("127.0.0.1:43121", 43120));
        assert!(!host_is_loopback("localhost", 43120));
    }

    #[test]
    fn highlighted_scene_rejects_a_missing_geometry_index() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/viewer/models/enclosure/counterbore.step");
        let mut timings = Timings::default();
        let mut scene = compile_scene(&path, UpAxis::Z, &mut timings).unwrap();
        scene.instances[0].geometry = scene.geometries.len();

        let error = highlighted_scene(
            &scene,
            FocusPair {
                first: 0,
                second: 0,
            },
        )
        .err()
        .unwrap();
        assert!(error.contains("references missing geometry"));
    }
}
