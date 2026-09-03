use crate::{
    cache::{source_fingerprint, ViewerCache},
    interference::{self, CheckReport},
    load_status::{valid_load_id, LoadReporter, LoadTracker},
    motion::{prepare_motion, PreparedMotion, MAX_MOTION_COMPONENTS},
    project::{Motion, Project},
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
    sync::{mpsc, Arc, Mutex, MutexGuard},
    thread,
    time::{SystemTime, UNIX_EPOCH},
};
use tiny_http::{Header, Method, Request, Response, Server, StatusCode};

const SHELL_HTML: &str = include_str!("viewer_shell.html");
const LOGO_PNG: &[u8] = include_bytes!("../assets/burr-logo.png");
const VIEWER_FRAMING_MARGIN: f32 = 1.3;
const SERVER_WORKERS: usize = 4;
const MAX_MEMORY_VIEWERS: usize = 32;
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
    report: Option<CheckReport>,
}

#[derive(Default)]
struct ModelCache {
    models: HashMap<PathBuf, CachedModel>,
    viewers: HashMap<String, String>,
}

struct RenderedViewer {
    html: String,
    cache: &'static str,
}

struct RenderSelection<'a> {
    relative_path: &'a str,
    theme: ViewerTheme,
    focus: Option<FocusPair>,
    motion_id: Option<&'a str>,
}

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
    let project = Arc::new(Project::discover(&start)?);

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

    let cache = Arc::new(Mutex::new(ModelCache::default()));
    let load_tracker = Arc::new(LoadTracker::default());
    let viewer_cache = Arc::new(ViewerCache::from_environment());
    let (sender, receiver) = mpsc::channel::<Request>();
    let receiver = Arc::new(Mutex::new(receiver));
    for _ in 0..SERVER_WORKERS {
        let receiver = Arc::clone(&receiver);
        let project = Arc::clone(&project);
        let cache = Arc::clone(&cache);
        let load_tracker = Arc::clone(&load_tracker);
        let viewer_cache = Arc::clone(&viewer_cache);
        thread::spawn(move || loop {
            let request = {
                let Ok(receiver) = receiver.lock() else {
                    break;
                };
                match receiver.recv() {
                    Ok(request) => request,
                    Err(_) => break,
                }
            };
            if let Err(error) = handle_request(
                request,
                &project,
                &cache,
                &load_tracker,
                &viewer_cache,
                port,
            ) {
                eprintln!("Viewer request failed: {error}");
            }
        });
    }
    for request in server.incoming_requests() {
        sender
            .send(request)
            .map_err(|_| "Burr viewer workers stopped unexpectedly.".to_string())?;
    }
    Ok(())
}

fn handle_request(
    request: Request,
    project: &Project,
    cache: &Mutex<ModelCache>,
    load_tracker: &LoadTracker,
    viewer_cache: &ViewerCache,
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
                let motions = project.public_state()["motions"].clone();
                respond(
                    request,
                    200,
                    "application/json; charset=utf-8",
                    json!({ "root": root_name, "files": files, "motions": motions }).to_string(),
                )
            }
            Err(error) => respond_json_error(request, 500, &error),
        },
        "/api/load-status" => {
            let Some(load_id) = query_value(&url, "id") else {
                return respond_json_error(request, 400, "No load id was provided.");
            };
            if !valid_load_id(&load_id) {
                return respond_json_error(request, 400, "The load id was invalid.");
            }
            respond(
                request,
                200,
                "application/json; charset=utf-8",
                serde_json::to_string(&load_tracker.status(&load_id))
                    .map_err(|error| format!("Failed to encode load status: {error}"))?,
            )
        }
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
            let motion_id = query_value(&url, "motion");
            let load_id = query_value(&url, "load");
            if load_id.as_deref().is_some_and(|id| !valid_load_id(id)) {
                return respond_html_error(request, 400, "The load id was invalid.");
            }
            let reporter = load_tracker.reporter(load_id.as_deref());
            reporter.start(&relative_path);
            let rendered = {
                let mut cache = lock_model_cache(cache)?;
                render_model(
                    project,
                    RenderSelection {
                        relative_path: &relative_path,
                        theme,
                        focus,
                        motion_id: motion_id.as_deref(),
                    },
                    &mut cache,
                    viewer_cache,
                    &reporter,
                )
            };
            match rendered {
                Ok(rendered) => {
                    reporter.ready(rendered.cache);
                    respond(request, 200, "text/html; charset=utf-8", rendered.html)
                }
                Err(error) => {
                    reporter.failed(&error);
                    respond_html_error(request, 422, &error)
                }
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
    selection: RenderSelection<'_>,
    cache: &mut ModelCache,
    viewer_cache: &ViewerCache,
    reporter: &LoadReporter<'_>,
) -> Result<RenderedViewer, String> {
    if let Some(motion_id) = selection.motion_id {
        if selection.focus.is_some() {
            return Err(
                "Motion playback is unavailable while components are highlighted.".to_string(),
            );
        }
        let motion = project
            .motion(motion_id)
            .cloned()
            .ok_or_else(|| format!("Unknown Burr motion: {motion_id}"))?;
        return render_motion_model(
            project,
            selection.relative_path,
            selection.theme,
            &motion,
            cache,
            viewer_cache,
            reporter,
        );
    }

    reporter.stage(
        "Reading source",
        &format!(
            "Checking {} against the reusable viewer cache.",
            selection.relative_path
        ),
    );
    let path = resolve_model_path(project, selection.relative_path)?;
    let fingerprint = source_fingerprint(&path)?;
    let viewer_key = viewer_cache_key(
        selection.relative_path,
        &path,
        &fingerprint,
        selection.theme,
        selection.focus,
        None,
    );
    if let Some(html) = cache.viewers.get(&viewer_key) {
        return Ok(RenderedViewer {
            html: html.clone(),
            cache: "memory",
        });
    }
    match viewer_cache.load(&viewer_key) {
        Ok(Some(html)) => {
            remember_viewer(cache, viewer_key, html.clone());
            return Ok(RenderedViewer {
                html,
                cache: "disk",
            });
        }
        Ok(None) => {}
        Err(error) => eprintln!("burr: {error}; rebuilding the viewer"),
    }

    let html = {
        let cached = load_model(project, selection.relative_path, cache, Some(reporter))?;
        if let Some(focus) = selection.focus {
            if focus.second >= cached.scene.instances.len() {
                return Err("Viewer focus references a missing component.".to_string());
            }
        }

        let scene = match selection.focus {
            Some(focus) => highlighted_scene(&cached.scene, focus)?,
            None => cached.scene.clone(),
        };
        reporter.stage(
            "Building viewer",
            "Encoding the compiled geometry for the local browser.",
        );
        let lighting = selection.theme.lighting();
        let html = generate_html_viewer(
            &scene,
            selection.relative_path,
            &scene.statistics,
            &lighting,
            selection.theme.canvas_background(),
        )
        .map_err(|error| {
            format!(
                "Look could not build the viewer for {}: {error:#}",
                selection.relative_path
            )
        })?;
        let html = inject_viewer_render_modes(html)?;
        inject_viewer_theme(html, selection.theme, selection.focus)?
    };
    persist_viewer(viewer_cache, &viewer_key, &html);
    remember_viewer(cache, viewer_key, html.clone());
    Ok(RenderedViewer {
        html,
        cache: "generated",
    })
}

fn render_motion_model(
    project: &Project,
    relative_path: &str,
    theme: ViewerTheme,
    motion: &Motion,
    cache: &mut ModelCache,
    viewer_cache: &ViewerCache,
    reporter: &LoadReporter<'_>,
) -> Result<RenderedViewer, String> {
    let initial_progress = if relative_path == motion.from {
        0.0
    } else if relative_path == motion.to {
        1.0
    } else {
        return Err(format!(
            "Motion '{}' does not include model {relative_path}.",
            motion.id
        ));
    };

    reporter.stage(
        "Reading source",
        "Checking both motion poses against the reusable viewer cache.",
    );
    let from_path = resolve_model_path(project, &motion.from)?;
    let to_path = resolve_model_path(project, &motion.to)?;
    let from_fingerprint = source_fingerprint(&from_path)?;
    let to_fingerprint = source_fingerprint(&to_path)?;
    let motion_signature = format!(
        "{}|{}|{}|{}|{}",
        motion.id, motion.from, motion.to, motion.duration_ms, to_fingerprint
    );
    let viewer_key = viewer_cache_key(
        relative_path,
        &from_path,
        &from_fingerprint,
        theme,
        None,
        Some(&motion_signature),
    );
    if let Some(html) = cache.viewers.get(&viewer_key) {
        return Ok(RenderedViewer {
            html: html.clone(),
            cache: "memory",
        });
    }
    match viewer_cache.load(&viewer_key) {
        Ok(Some(html)) => {
            remember_viewer(cache, viewer_key, html.clone());
            return Ok(RenderedViewer {
                html,
                cache: "disk",
            });
        }
        Ok(None) => {}
        Err(error) => eprintln!("burr: {error}; rebuilding the viewer"),
    }

    let from_scene = load_model(project, &motion.from, cache, Some(reporter))?
        .scene
        .clone();
    let to_scene = load_model(project, &motion.to, cache, Some(reporter))?
        .scene
        .clone();
    reporter.stage(
        "Preparing motion",
        "Matching rigid components and generating the playback frames.",
    );
    let prepared = prepare_motion(&from_scene, &to_scene, motion.duration_ms, initial_progress)?;
    reporter.stage(
        "Building viewer",
        "Encoding the compiled motion for the local browser.",
    );
    let lighting = theme.lighting();
    let html = generate_html_viewer(
        &prepared.scene,
        relative_path,
        &prepared.scene.statistics,
        &lighting,
        theme.canvas_background(),
    )
    .map_err(|error| format!("Look could not build motion '{}': {error:#}", motion.id))?;
    let html = inject_viewer_render_modes(html)?;
    let html = inject_viewer_motion(html, &prepared)?;
    let html = inject_viewer_theme(html, theme, None)?;
    persist_viewer(viewer_cache, &viewer_key, &html);
    remember_viewer(cache, viewer_key, html.clone());
    Ok(RenderedViewer {
        html,
        cache: "generated",
    })
}

fn check_model(
    project: &Project,
    relative_path: &str,
    cache: &Mutex<ModelCache>,
) -> Result<CheckReport, String> {
    let path = resolve_model_path(project, relative_path)?;
    let format = model_format(&path);
    let (version, scene) = {
        let mut cache = lock_model_cache(cache)?;
        let cached = load_model(project, relative_path, &mut cache, None)?;
        if let Some(report) = cached.report.as_ref() {
            return Ok(report.clone());
        }
        (cached.version.clone(), cached.scene.clone())
    };

    let report = if format == Some("STEP") {
        interference::analyze_scene(relative_path, &version, &scene)
    } else {
        CheckReport::unsupported(
            relative_path,
            &version,
            "Assembly interference currently supports STEP files only.",
        )
    };

    let mut cache = lock_model_cache(cache)?;
    if let Some(cached) = cache.models.get_mut(&path) {
        if cached.version == version {
            cached.report = Some(report.clone());
        }
    }
    Ok(report)
}

fn load_model<'a>(
    project: &Project,
    relative_path: &str,
    cache: &'a mut ModelCache,
    reporter: Option<&LoadReporter<'_>>,
) -> Result<&'a mut CachedModel, String> {
    let path = resolve_model_path(project, relative_path)?;
    let version = file_version(&path)?;
    let current = cache
        .models
        .get(&path)
        .is_some_and(|cached| cached.version == version);
    if current {
        if let Some(reporter) = reporter {
            reporter.stage(
                "Preparing viewer",
                &format!("Reusing compiled geometry for {relative_path}."),
            );
        }
        return cache
            .models
            .get_mut(&path)
            .ok_or_else(|| "Viewer model cache became unavailable.".to_string());
    }
    let mut timings = Timings::default();
    let up_axis = if model_format(&path) == Some("GLB") {
        UpAxis::Y
    } else {
        UpAxis::Z
    };
    if let Some(reporter) = reporter {
        reporter.stage(
            "Tessellating geometry",
            &format!("Look is compiling {relative_path} locally."),
        );
    }
    let mut scene = compile_scene(&path, up_axis, &mut timings)
        .map_err(|error| format!("Look could not render {relative_path}: {error:#}"))?;
    if let Some(reporter) = reporter {
        reporter.stage(
            "Preparing materials",
            &format!("Decoding the textures and materials for {relative_path}."),
        );
    }
    prepare_source_textures(&mut scene, &mut timings)
        .map_err(|error| format!("Look could not prepare {relative_path}: {error:#}"))?;
    // Look's self-contained viewer fits directly to the scene sphere. Leave a
    // little breathing room for the shorter iframe viewport created by Burr's
    // navigation shell and for models whose diagonal approaches that sphere.
    scene.fit_radius *= VIEWER_FRAMING_MARGIN;
    cache.models.insert(
        path.clone(),
        CachedModel {
            version,
            scene,
            report: None,
        },
    );
    cache
        .models
        .get_mut(&path)
        .ok_or_else(|| "Viewer model cache became unavailable.".to_string())
}

fn viewer_cache_key(
    relative_path: &str,
    source_path: &Path,
    source_fingerprint: &str,
    theme: ViewerTheme,
    focus: Option<FocusPair>,
    motion: Option<&str>,
) -> String {
    format!(
        "burr-viewer-v1\nburr={}\nsource={}\nfingerprint={}\nrelative={}\ntheme={}\nfocus={}\nmotion={}",
        env!("CARGO_PKG_VERSION"),
        source_path.display(),
        source_fingerprint,
        relative_path,
        theme.name(),
        focus.map_or_else(|| "none".to_string(), FocusPair::label),
        motion.unwrap_or("none"),
    )
}

fn persist_viewer(cache: &ViewerCache, key: &str, html: &str) {
    match cache.store(key, html) {
        Ok(_) => {}
        Err(error) => eprintln!("burr: {error}; continuing without a reusable viewer"),
    }
}

fn remember_viewer(cache: &mut ModelCache, key: String, html: String) {
    if cache.viewers.len() >= MAX_MEMORY_VIEWERS && !cache.viewers.contains_key(&key) {
        cache.viewers.clear();
    }
    cache.viewers.insert(key, html);
}

fn lock_model_cache(cache: &Mutex<ModelCache>) -> Result<MutexGuard<'_, ModelCache>, String> {
    cache
        .lock()
        .map_err(|_| "Burr model cache became unavailable.".to_string())
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
            "gl.uniform3fv(uCamPosLoc, camPos);\n            burrApplyMotionFrame(gl);\n\n            const xRay = burrRenderMode === 'x-ray';\n            gl.uniform1f(uOpacityLoc, xRay ? 0.28 : 1.0);\n            if (xRay) {\n                gl.enable(gl.BLEND);\n                gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);\n                gl.depthMask(false);\n            } else {\n                gl.disable(gl.BLEND);\n                gl.depthMask(true);\n            }\n\n            gl.bindVertexArray(vao);",
            "render-mode state",
        ),
        (
            "gl.drawElements(gl.TRIANGLES, indices.length, gl.UNSIGNED_INT, 0);",
            "gl.drawElements(gl.TRIANGLES, indices.length, gl.UNSIGNED_INT, 0);\n            gl.depthMask(true);\n            burrCaptureSnapshot(canvas);",
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
    let controls = r#"<meta name="burr-render-modes" content="x-ray,solid"><meta name="burr-snapshot-export" content="png"><script id="burr-viewer-controls">
let burrRenderMode = "x-ray";
let burrSnapshotRequest = null;
document.documentElement.dataset.burrRenderMode = burrRenderMode;

function burrApplyMotionFrame() {}

window.addEventListener("message", (event) => {
    if (event.origin !== window.location.origin) return;
    if (event.data?.type === "burr:set-render-mode") {
        const mode = event.data.mode;
        if (mode !== "x-ray" && mode !== "solid") return;
        burrRenderMode = mode;
        document.documentElement.dataset.burrRenderMode = mode;
        return;
    }
    if (event.data?.type !== "burr:export-snapshot") return;
    const filename = event.data.filename;
    if (typeof filename !== "string" || !/^[a-zA-Z0-9._-]+\.png$/.test(filename)) {
        window.parent.postMessage(
            { type: "burr:snapshot-error", message: "Snapshot filename was invalid." },
            window.location.origin,
        );
        return;
    }
    burrSnapshotRequest = filename;
});

function burrCaptureSnapshot(canvas) {
    if (!burrSnapshotRequest) return;
    const filename = burrSnapshotRequest;
    burrSnapshotRequest = null;
    canvas.toBlob((blob) => {
        if (!blob) {
            window.parent.postMessage(
                { type: "burr:snapshot-error", message: "The model canvas could not be captured." },
                window.location.origin,
            );
            return;
        }
        const url = URL.createObjectURL(blob);
        const link = document.createElement("a");
        link.href = url;
        link.download = filename;
        document.body.append(link);
        link.click();
        link.remove();
        setTimeout(() => URL.revokeObjectURL(url), 1000);
        window.parent.postMessage(
            { type: "burr:snapshot-exported", filename },
            window.location.origin,
        );
    }, "image/png");
}
</script>"#;
    html.insert_str(index, controls);
    Ok(html)
}

fn inject_viewer_motion(mut html: String, motion: &PreparedMotion) -> Result<String, String> {
    let shader_header = format!(
        "in vec4 aColor;\n            in float aBurrInstance;\n            uniform mat4 uMVP;\n            uniform mat4 uBurrInstanceTransforms[{MAX_MOTION_COMPONENTS}];"
    );
    let motion_arrays = format!(
        "const colors = b64ToFloat32Array(colorB64);\n        const burrInstanceIds = b64ToFloat32Array(\"{}\");\n        const burrMotionFrames = b64ToFloat32Array(\"{}\");",
        motion.instance_ids_base64, motion.frames_base64
    );
    let replacements = [
        (
            "in vec4 aColor;\n            uniform mat4 uMVP;".to_string(),
            shader_header,
            "motion vertex attributes",
        ),
        (
            "vNormal = mat3(uModel) * aNormal;\n                vFragPos = vec3(uModel * vec4(aPosition, 1.0));\n                vColor = aColor;\n                gl_Position = uMVP * vec4(aPosition, 1.0);".to_string(),
            "mat4 burrTransform = uBurrInstanceTransforms[int(aBurrInstance)];\n                vec4 burrPosition = burrTransform * vec4(aPosition, 1.0);\n                vNormal = mat3(burrTransform) * aNormal;\n                vFragPos = vec3(burrPosition);\n                vColor = aColor;\n                gl_Position = uMVP * burrPosition;".to_string(),
            "motion vertex transform",
        ),
        (
            "const colors = b64ToFloat32Array(colorB64);".to_string(),
            motion_arrays,
            "motion data arrays",
        ),
        (
            "const idxBuffer = gl.createBuffer();".to_string(),
            "const burrInstanceBuffer = gl.createBuffer();\n        gl.bindBuffer(gl.ARRAY_BUFFER, burrInstanceBuffer);\n        gl.bufferData(gl.ARRAY_BUFFER, burrInstanceIds, gl.STATIC_DRAW);\n        const aBurrInstanceLoc = gl.getAttribLocation(program, 'aBurrInstance');\n        gl.enableVertexAttribArray(aBurrInstanceLoc);\n        gl.vertexAttribPointer(aBurrInstanceLoc, 1, gl.FLOAT, false, 0, 0);\n\n        const idxBuffer = gl.createBuffer();".to_string(),
            "motion instance buffer",
        ),
        (
            "const uModelLoc = gl.getUniformLocation(program, 'uModel');".to_string(),
            "const uModelLoc = gl.getUniformLocation(program, 'uModel');\n        const uBurrInstanceTransformsLoc = gl.getUniformLocation(program, 'uBurrInstanceTransforms[0]');".to_string(),
            "motion transform uniform",
        ),
    ];
    for (needle, replacement, label) in replacements {
        if !html.contains(&needle) {
            return Err(format!(
                "Look viewer HTML did not contain the expected {label} hook."
            ));
        }
        html = html.replacen(&needle, &replacement, 1);
    }

    let marker = "</head>";
    let Some(index) = html.find(marker) else {
        return Err("Look viewer HTML did not contain a head element.".to_string());
    };
    let motion_script = format!(
        r#"<meta name="burr-motion" content="rigid-poses"><script id="burr-motion-player">
const burrMotionFrameCount = {frame_count};
const burrMotionInstanceCount = {instance_count};
const burrMotionDuration = {duration_ms};
let burrMotionProgress = {initial_progress};
let burrMotionPlaying = false;
let burrMotionStartedAt = null;
let burrMotionLastReportedFrame = -1;

function burrEmitMotionState() {{
    window.parent.postMessage(
        {{
            type: "burr:motion-state",
            progress: burrMotionProgress,
            playing: burrMotionPlaying,
        }},
        window.location.origin,
    );
}}

function burrApplyMotionFrame(gl) {{
    const wasPlaying = burrMotionPlaying;
    if (burrMotionPlaying) {{
        const now = performance.now();
        if (burrMotionStartedAt === null) {{
            burrMotionStartedAt = now - burrMotionProgress * burrMotionDuration;
        }}
        burrMotionProgress = Math.min(1, (now - burrMotionStartedAt) / burrMotionDuration);
        if (burrMotionProgress >= 1) {{
            burrMotionProgress = 1;
            burrMotionPlaying = false;
            burrMotionStartedAt = null;
        }}
    }}

    const frame = Math.round(burrMotionProgress * (burrMotionFrameCount - 1));
    const start = frame * burrMotionInstanceCount * 16;
    const end = start + burrMotionInstanceCount * 16;
    gl.uniformMatrix4fv(
        uBurrInstanceTransformsLoc,
        false,
        burrMotionFrames.subarray(start, end),
    );
    if (frame !== burrMotionLastReportedFrame || (wasPlaying && !burrMotionPlaying)) {{
        burrMotionLastReportedFrame = frame;
        burrEmitMotionState();
    }}
}}

window.addEventListener("message", (event) => {{
    if (event.origin !== window.location.origin) return;
    if (event.data?.type === "burr:set-motion-progress") {{
        const progress = Number(event.data.progress);
        if (!Number.isFinite(progress)) return;
        burrMotionProgress = Math.max(0, Math.min(1, progress));
        burrMotionPlaying = false;
        burrMotionStartedAt = null;
        burrMotionLastReportedFrame = -1;
        burrEmitMotionState();
        return;
    }}
    if (event.data?.type !== "burr:toggle-motion") return;
    if (burrMotionPlaying) {{
        burrMotionPlaying = false;
        burrMotionStartedAt = null;
    }} else {{
        if (burrMotionProgress >= 1) burrMotionProgress = 0;
        burrMotionPlaying = true;
        burrMotionStartedAt = performance.now() - burrMotionProgress * burrMotionDuration;
    }}
    burrEmitMotionState();
}});
</script>"#,
        frame_count = motion.frame_count,
        instance_count = motion.instance_count,
        duration_ms = motion.duration_ms,
        initial_progress = motion.initial_progress,
    );
    html.insert_str(index, &motion_script);
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
        assert!(rendered.contains("name=\"burr-snapshot-export\" content=\"png\""));
        assert!(rendered.contains("burr:export-snapshot"));
        assert!(rendered.contains("canvas.toBlob"));
        assert!(rendered.contains("burrCaptureSnapshot(canvas);"));
    }

    #[test]
    fn viewer_motion_injects_rigid_component_playback() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/interference/separated.step");
        let mut timings = Timings::default();
        let from = compile_scene(&path, UpAxis::Z, &mut timings).unwrap();
        let mut to = from.clone();
        to.instances[1].transform *= glam::Mat4::from_rotation_z(std::f32::consts::FRAC_PI_2);
        let motion = prepare_motion(&from, &to, 800, 0.0).unwrap();
        let html = generate_html_viewer(
            &motion.scene,
            "motion.step",
            &motion.scene.statistics,
            &LightingConfig::default(),
            "#0c0d10",
        )
        .unwrap();
        let html = inject_viewer_render_modes(html).unwrap();
        let html = inject_viewer_motion(html, &motion).unwrap();

        assert!(html.contains("name=\"burr-motion\" content=\"rigid-poses\""));
        assert!(html.contains("in float aBurrInstance;"));
        assert!(html.contains("uniform mat4 uBurrInstanceTransforms[32];"));
        assert!(html.contains("burrMotionFrames.subarray(start, end)"));
        assert!(html.contains("burr:set-motion-progress"));
        assert!(html.contains("burr:toggle-motion"));
        assert!(html.contains("burrApplyMotionFrame(gl);"));
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
