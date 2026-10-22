//! Lectern: Tauri shell. Rust owns file I/O, watching, rendering, sanitizing, TOC and
//! settings; the frontend (ui/) owns presentation.

pub mod config;
pub mod highlight;
pub mod render;
pub mod slug;
pub mod watcher;

use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, RwLock};
use std::thread;
use std::time::Duration;

use percent_encoding::percent_decode_str;
use serde::Serialize;
use tauri::http::{Response, StatusCode, header};
use tauri::{
    AppHandle, DragDropEvent, Emitter, LogicalPosition, LogicalSize, Manager, State, WebviewWindow,
    WindowEvent,
};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;

use config::{Config, UiSettings};
use render::{RenderOptions, TocEntry};

/// URL prefix under which local images are served (see [`serve_image`]).
/// Tauri exposes custom schemes as `http://<scheme>.localhost/` on Windows.
pub const IMAGE_PREFIX: &str = if cfg!(windows) {
    "http://mdfile.localhost/"
} else {
    "mdfile://localhost/"
};

pub const MARKDOWN_EXTENSIONS: &[&str] = &[
    "md", "markdown", "mdown", "mkd", "mkdn", "mdwn", "mdtxt", "mdtext", "text", "txt",
];

const MAX_FILE_BYTES: u64 = 64 * 1024 * 1024;

#[derive(Default)]
struct AppState {
    config: Mutex<Config>,
    config_path: Option<PathBuf>,
    /// The currently displayed file.
    current: Mutex<Option<PathBuf>>,
    /// Images the current document references; the only files `mdfile:` will serve.
    images: RwLock<HashSet<PathBuf>>,
    watcher: Mutex<Option<watcher::FileWatcher>>,
    /// File given on the command line, consumed by `startup`.
    initial_file: Mutex<Option<PathBuf>>,
}

impl AppState {
    fn save_config(&self) {
        if let Some(path) = &self.config_path {
            let cfg = lock(&self.config).clone();
            if let Err(e) = cfg.save(path) {
                eprintln!("lectern: cannot save {}: {e}", path.display());
            }
        }
    }
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Doc {
    path: String,
    file_name: String,
    html: String,
    toc: Vec<TocEntry>,
    title: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RecentFile {
    path: String,
    name: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Startup {
    settings: UiSettings,
    doc: Option<Doc>,
    error: Option<String>,
    platform: &'static str,
    version: &'static str,
}

// ---------------------------------------------------------------------------------------
// Document loading

fn absolute(path: &Path) -> PathBuf {
    render::normalize(&std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf()))
}

fn read_text(path: &Path) -> Result<String, String> {
    let describe = |e: io::Error| match e.kind() {
        io::ErrorKind::NotFound => format!("File not found: {}", path.display()),
        io::ErrorKind::PermissionDenied => format!("Permission denied: {}", path.display()),
        _ => format!("Cannot read {}: {e}", path.display()),
    };
    let meta = fs::metadata(path).map_err(describe)?;
    if meta.is_dir() {
        return Err(format!("{} is a directory", path.display()));
    }
    if meta.len() > MAX_FILE_BYTES {
        return Err(format!(
            "{} is too large ({} MB)",
            path.display(),
            meta.len() / 1024 / 1024
        ));
    }
    let bytes = fs::read(path).map_err(describe)?;
    let text = String::from_utf8(bytes)
        .unwrap_or_else(|e| String::from_utf8_lossy(e.as_bytes()).into_owned());
    Ok(text
        .strip_prefix('\u{feff}')
        .map(str::to_owned)
        .unwrap_or(text))
}

/// Read, render, and make `path` the current document (watching it for changes).
fn load_doc(app: &AppHandle, state: &AppState, path: &Path) -> Result<Doc, String> {
    let path = absolute(path);
    let text = read_text(&path)?;
    let rendered = render::render(
        &text,
        &RenderOptions {
            base_dir: path.parent().map(Path::to_path_buf),
            local_image_prefix: Some(IMAGE_PREFIX.to_owned()),
        },
    );
    *state.images.write().unwrap_or_else(|p| p.into_inner()) =
        rendered.local_images.into_iter().collect();

    let file_name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string());

    let changed = lock(&state.current).as_deref() != Some(path.as_path());
    if changed {
        *lock(&state.current) = Some(path.clone());
        start_watching(app, state, &path);
        lock(&state.config).push_recent(&path);
        state.save_config();
        if let Some(win) = app.get_webview_window("main") {
            let _ = win.set_title(&format!("{file_name} — Lectern"));
        }
    }

    Ok(Doc {
        path: path.display().to_string(),
        file_name,
        html: rendered.html,
        toc: rendered.toc,
        title: rendered.title,
    })
}

fn start_watching(app: &AppHandle, state: &AppState, path: &Path) {
    let app = app.clone();
    let payload = path.display().to_string();
    let watcher = watcher::watch(path, move || {
        let _ = app.emit("file-changed", &payload);
    });
    match watcher {
        Ok(w) => *lock(&state.watcher) = Some(w),
        Err(e) => {
            eprintln!("lectern: cannot watch {}: {e}", path.display());
            *lock(&state.watcher) = None;
        }
    }
}

fn is_markdown(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| MARKDOWN_EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()))
}

// ---------------------------------------------------------------------------------------
// Commands

#[tauri::command]
async fn startup(app: AppHandle, state: State<'_, AppState>) -> Result<Startup, String> {
    let settings = lock(&state.config).ui.clone();
    let initial = lock(&state.initial_file).take();
    // After a webview reload, show the document that was already open.
    let initial = initial.or_else(|| lock(&state.current).clone());
    let (doc, error) = match initial {
        Some(path) => match load_doc(&app, &state, &path) {
            Ok(doc) => (Some(doc), None),
            Err(e) => (None, Some(e)),
        },
        None => (None, None),
    };
    Ok(Startup {
        settings,
        doc,
        error,
        platform: std::env::consts::OS,
        version: env!("CARGO_PKG_VERSION"),
    })
}

/// Called by the frontend once the first paint is ready; avoids a white flash.
#[tauri::command]
fn ready(window: WebviewWindow) {
    let _ = window.show();
    let _ = window.set_focus();
}

#[tauri::command]
async fn open_path(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> Result<Doc, String> {
    load_doc(&app, &state, Path::new(&path))
}

#[tauri::command]
async fn open_dialog(app: AppHandle, state: State<'_, AppState>) -> Result<Option<Doc>, String> {
    let start_dir = lock(&state.current)
        .as_deref()
        .and_then(Path::parent)
        .map(Path::to_path_buf);
    let mut dialog = app
        .dialog()
        .file()
        .set_title("Open Markdown file")
        .add_filter("Markdown", MARKDOWN_EXTENSIONS)
        .add_filter("All files", &["*"]);
    if let Some(dir) = start_dir {
        dialog = dialog.set_directory(dir);
    }
    let Some(picked) = dialog.blocking_pick_file() else {
        return Ok(None);
    };
    let path = picked.into_path().map_err(|e| e.to_string())?;
    load_doc(&app, &state, &path).map(Some)
}

/// Follow a relative link (`other.md`, `../docs/x.md#section`) from the current document.
#[tauri::command]
async fn open_link(
    app: AppHandle,
    state: State<'_, AppState>,
    href: String,
) -> Result<Doc, String> {
    let target = resolve_link(lock(&state.current).as_deref(), &href)?;
    if !is_markdown(&target) {
        return Err(format!("Not a Markdown file: {}", target.display()));
    }
    load_doc(&app, &state, &target)
}

fn resolve_link(current: Option<&Path>, href: &str) -> Result<PathBuf, String> {
    let path_part = href.split(['#', '?']).next().unwrap_or_default();
    let decoded = percent_decode_str(path_part).decode_utf8_lossy();
    let path_part = decoded.strip_prefix("file://").unwrap_or(&decoded);
    if path_part.is_empty() {
        return Err("Link has no path".into());
    }
    let p = PathBuf::from(path_part);
    if p.is_absolute() {
        return Ok(render::normalize(&p));
    }
    let dir = current
        .and_then(Path::parent)
        .ok_or("No document is open")?;
    Ok(render::normalize(&dir.join(p)))
}

#[tauri::command]
async fn reload(app: AppHandle, state: State<'_, AppState>) -> Result<Doc, String> {
    let Some(path) = lock(&state.current).clone() else {
        return Err("No document is open".into());
    };
    // Editors that save atomically may briefly remove the file; retry a few times.
    let mut last_err = String::new();
    for attempt in 0..5 {
        match load_doc(&app, &state, &path) {
            Ok(doc) => return Ok(doc),
            Err(e) => last_err = e,
        }
        thread::sleep(Duration::from_millis(60 * (attempt + 1)));
    }
    Err(last_err)
}

#[tauri::command]
fn recent_files(state: State<'_, AppState>) -> Vec<RecentFile> {
    lock(&state.config)
        .recent_files
        .iter()
        .filter(|p| p.is_file())
        .map(|p| RecentFile {
            path: p.display().to_string(),
            name: p
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default(),
        })
        .collect()
}

#[tauri::command]
fn clear_recent(state: State<'_, AppState>) {
    lock(&state.config).recent_files.clear();
    state.save_config();
}

#[tauri::command]
fn save_settings(state: State<'_, AppState>, settings: UiSettings) {
    lock(&state.config).ui = settings.sanitized();
    state.save_config();
}

#[tauri::command]
fn open_external(app: AppHandle, url: String) -> Result<(), String> {
    let lower = url.trim().to_ascii_lowercase();
    if !["http://", "https://", "mailto:"]
        .iter()
        .any(|s| lower.starts_with(s))
    {
        return Err(format!("Refusing to open {url}"));
    }
    app.opener()
        .open_url(url.trim(), None::<&str>)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn print(window: WebviewWindow) -> Result<(), String> {
    window.print().map_err(|e| e.to_string())
}

#[tauri::command]
fn close_window(window: WebviewWindow) {
    let _ = window.close();
}

// ---------------------------------------------------------------------------------------
// mdfile: protocol (local images)

fn image_mime(path: &Path) -> Option<&'static str> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    Some(match ext.as_str() {
        "png" | "apng" => "image/png",
        "jpg" | "jpeg" | "jfif" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "avif" => "image/avif",
        "bmp" => "image/bmp",
        "ico" => "image/x-icon",
        _ => return None,
    })
}

fn serve_image(state: &AppState, uri_path: &str) -> Response<Vec<u8>> {
    let respond = |status: StatusCode, body: Vec<u8>, mime: &str| {
        Response::builder()
            .status(status)
            .header(header::CONTENT_TYPE, mime)
            .header(header::CACHE_CONTROL, "no-cache")
            .header(
                "Content-Security-Policy",
                "default-src 'none'; style-src 'unsafe-inline'",
            )
            .body(body)
            .unwrap_or_default()
    };
    let decoded = percent_decode_str(uri_path.trim_start_matches('/')).decode_utf8_lossy();
    let path = PathBuf::from(decoded.as_ref());
    let allowed = state
        .images
        .read()
        .unwrap_or_else(|p| p.into_inner())
        .contains(&path);
    if !allowed {
        return respond(StatusCode::FORBIDDEN, Vec::new(), "text/plain");
    }
    let Some(mime) = image_mime(&path) else {
        return respond(StatusCode::UNSUPPORTED_MEDIA_TYPE, Vec::new(), "text/plain");
    };
    match fs::read(&path) {
        Ok(bytes) => respond(StatusCode::OK, bytes, mime),
        Err(_) => respond(StatusCode::NOT_FOUND, Vec::new(), "text/plain"),
    }
}

// ---------------------------------------------------------------------------------------
// Window geometry

fn restore_geometry(win: &WebviewWindow, g: &config::WindowGeometry) {
    let _ = win.set_size(LogicalSize::new(g.width, g.height));
    if let (Some(x), Some(y)) = (g.x, g.y) {
        // Only restore the position if the title bar would land on a connected monitor.
        let visible = win
            .available_monitors()
            .unwrap_or_default()
            .iter()
            .any(|m| {
                let scale = m.scale_factor();
                let pos = m.position().to_logical::<f64>(scale);
                let size = m.size().to_logical::<f64>(scale);
                let (px, py) = (x + 40.0, y + 10.0);
                px >= pos.x && px < pos.x + size.width && py >= pos.y && py < pos.y + size.height
            });
        if visible {
            let _ = win.set_position(LogicalPosition::new(x, y));
        }
    }
    if g.maximized {
        let _ = win.maximize();
    }
}

fn track_geometry(win: &tauri::Window, state: &AppState) {
    if win.is_maximized().unwrap_or(false) || win.is_minimized().unwrap_or(false) {
        return;
    }
    let Ok(scale) = win.scale_factor() else {
        return;
    };
    let mut cfg = lock(&state.config);
    if let Ok(size) = win.inner_size() {
        let size = size.to_logical::<f64>(scale);
        cfg.window.width = size.width;
        cfg.window.height = size.height;
    }
    if let Ok(pos) = win.outer_position() {
        let pos = pos.to_logical::<f64>(scale);
        cfg.window.x = Some(pos.x);
        cfg.window.y = Some(pos.y);
    }
}

// ---------------------------------------------------------------------------------------

fn cli_file() -> Option<PathBuf> {
    let mut file = None;
    for arg in std::env::args_os().skip(1) {
        match arg.to_str() {
            Some("-h" | "--help") => {
                println!("Usage: lectern [FILE.md]\n\nA read-only Markdown viewer.");
                std::process::exit(0);
            }
            Some("-V" | "--version") => {
                println!("lectern {}", env!("CARGO_PKG_VERSION"));
                std::process::exit(0);
            }
            Some(s) if s.starts_with('-') => {}
            _ => {
                if file.is_none() {
                    file = Some(PathBuf::from(arg));
                }
            }
        }
    }
    file
}

pub fn run() {
    let initial_file = cli_file();
    let config_path = config::default_path();
    let config = config_path.as_deref().map(Config::load).unwrap_or_default();

    // Loading syntect's definitions takes a moment; do it while the webview starts.
    thread::spawn(highlight::warm_up);

    let state = AppState {
        config: Mutex::new(config),
        config_path,
        initial_file: Mutex::new(initial_file),
        ..Default::default()
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(state)
        .register_asynchronous_uri_scheme_protocol("mdfile", |ctx, request, responder| {
            let app = ctx.app_handle().clone();
            let path = request.uri().path().to_owned();
            thread::spawn(move || {
                responder.respond(serve_image(&app.state::<AppState>(), &path));
            });
        })
        .invoke_handler(tauri::generate_handler![
            startup,
            ready,
            open_path,
            open_dialog,
            open_link,
            reload,
            recent_files,
            clear_recent,
            save_settings,
            open_external,
            print,
            close_window,
        ])
        .setup(|app| {
            if let Some(win) = app.get_webview_window("main") {
                let geometry = lock(&app.state::<AppState>().config).window.clone();
                restore_geometry(&win, &geometry);
                // Safety net: never leave the window hidden if the frontend fails to call `ready`.
                thread::spawn(move || {
                    thread::sleep(Duration::from_secs(3));
                    if !win.is_visible().unwrap_or(true) {
                        let _ = win.show();
                    }
                });
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            let state = window.state::<AppState>();
            match event {
                WindowEvent::Moved(_) | WindowEvent::Resized(_) => track_geometry(window, &state),
                WindowEvent::CloseRequested { .. } => {
                    lock(&state.config).window.maximized = window.is_maximized().unwrap_or(false);
                    state.save_config();
                }
                WindowEvent::DragDrop(drop) => match drop {
                    DragDropEvent::Enter { .. } => {
                        let _ = window.emit("drag-state", true);
                    }
                    DragDropEvent::Leave => {
                        let _ = window.emit("drag-state", false);
                    }
                    DragDropEvent::Drop { paths, .. } => {
                        let _ = window.emit("drag-state", false);
                        let pick = paths
                            .iter()
                            .find(|p| is_markdown(p))
                            .or_else(|| paths.first());
                        if let Some(p) = pick {
                            let _ = window.emit("open-file", p.display().to_string());
                        }
                    }
                    _ => {}
                },
                _ => {}
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running lectern");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn link_resolution() {
        let cur = if cfg!(windows) {
            Path::new(r"C:\docs\guide\index.md")
        } else {
            Path::new("/docs/guide/index.md")
        };
        let dir = cur.parent().unwrap();
        assert_eq!(
            resolve_link(Some(cur), "other.md").unwrap(),
            dir.join("other.md")
        );
        assert_eq!(
            resolve_link(Some(cur), "../api/x%20y.md#sec").unwrap(),
            dir.parent().unwrap().join("api").join("x y.md")
        );
        assert!(resolve_link(Some(cur), "#only-fragment").is_err());
        assert!(resolve_link(None, "a.md").is_err());
    }

    #[test]
    fn markdown_detection() {
        assert!(is_markdown(Path::new("a.md")));
        assert!(is_markdown(Path::new("A.MARKDOWN")));
        assert!(!is_markdown(Path::new("a.png")));
        assert!(!is_markdown(Path::new("Makefile")));
    }

    #[test]
    fn image_protocol_only_serves_allowlisted_images() {
        let dir = tempfile::tempdir().unwrap();
        let img = dir.path().join("a.png");
        let secret = dir.path().join("secret.txt");
        fs::write(&img, b"\x89PNG").unwrap();
        fs::write(&secret, b"top secret").unwrap();
        let state = AppState::default();
        state.images.write().unwrap().insert(img.clone());
        state.images.write().unwrap().insert(secret.clone());

        let enc = |p: &Path| {
            percent_encoding::utf8_percent_encode(
                &p.to_string_lossy(),
                percent_encoding::NON_ALPHANUMERIC,
            )
            .to_string()
        };
        let ok = serve_image(&state, &format!("/{}", enc(&img)));
        assert_eq!(ok.status(), StatusCode::OK);
        assert_eq!(ok.headers()[header::CONTENT_TYPE], "image/png");
        // Allowlisted but not an image type.
        assert_eq!(
            serve_image(&state, &format!("/{}", enc(&secret))).status(),
            StatusCode::UNSUPPORTED_MEDIA_TYPE
        );
        // Not referenced by the document.
        let other = dir.path().join("b.png");
        fs::write(&other, b"x").unwrap();
        assert_eq!(
            serve_image(&state, &format!("/{}", enc(&other))).status(),
            StatusCode::FORBIDDEN
        );
    }
}
