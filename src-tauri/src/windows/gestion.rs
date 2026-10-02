//! Fenêtre shell Gestion (front embarqué `public/gestion/` → API PHP distante).

use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

pub const GESTION_LABEL: &str = "gestion";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GestionPrefs {
    /// URL absolue de l’API PHP, ex. https://exemple.fr/api
    pub api_url: String,
}

impl Default for GestionPrefs {
    fn default() -> Self {
        Self {
            api_url: String::new(),
        }
    }
}

fn prefs_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("gestion-prefs.json"))
}

pub fn load_prefs(app: &AppHandle) -> GestionPrefs {
    let Ok(path) = prefs_path(app) else {
        return GestionPrefs::default();
    };
    fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

fn save_prefs(app: &AppHandle, prefs: &GestionPrefs) -> Result<(), String> {
    let path = prefs_path(app)?;
    let raw = serde_json::to_string_pretty(prefs).map_err(|e| e.to_string())?;
    fs::write(path, raw).map_err(|e| e.to_string())
}

fn inject_api_url(window: &WebviewWindow, api_url: &str) {
    if api_url.trim().is_empty() {
        return;
    }
    let escaped = api_url.replace('\\', "\\\\").replace('\'', "\\'");
    let js = format!(
        r#"(function(){{
  try {{
    localStorage.setItem('mind_gestion_api_url', '{url}');
    window.API_URL = '{url}';
    window.__MIND_GESTION_API_URL__ = '{url}';
  }} catch (e) {{}}
}})();"#,
        url = escaped
    );
    let _ = window.eval(&js);
}

fn ensure_window(app: &AppHandle) -> Result<WebviewWindow, String> {
    if let Some(existing) = app.get_webview_window(GESTION_LABEL) {
        return Ok(existing);
    }

    let window = WebviewWindowBuilder::new(
        app,
        GESTION_LABEL,
        WebviewUrl::App("gestion/index.html".into()),
    )
    .title("MIND — Gestion")
    .inner_size(1280.0, 840.0)
    .min_inner_size(960.0, 640.0)
    .resizable(true)
    .decorations(true)
    .skip_taskbar(false)
    .center()
    .visible(false)
    .build()
    .map_err(|e| e.to_string())?;

    Ok(window)
}

/// Ouvre / focus la fenêtre Gestion et injecte l’URL API.
pub fn show_gestion(app: &AppHandle) -> Result<(), String> {
    let prefs = load_prefs(app);
    let window = ensure_window(app)?;
    inject_api_url(&window, &prefs.api_url);
    let _ = window.unminimize();
    window.show().map_err(|e| e.to_string())?;
    window.set_focus().map_err(|e| e.to_string())?;
    // Ré-injecte après chargement (localStorage prêt).
    let api = prefs.api_url.clone();
    let win = window.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(400));
        inject_api_url(&win, &api);
    });
    Ok(())
}

pub fn init_gestion(app: &AppHandle, background: bool) -> Result<(), String> {
    let _ = ensure_window(app)?;
    if !background {
        show_gestion(app)?;
    }
    Ok(())
}

#[tauri::command]
pub fn gestion_show(app: AppHandle) -> Result<(), String> {
    show_gestion(&app)
}

#[tauri::command]
pub fn gestion_get_config(app: AppHandle) -> Result<GestionPrefs, String> {
    Ok(load_prefs(&app))
}

#[tauri::command]
pub fn gestion_set_config(app: AppHandle, api_url: String) -> Result<GestionPrefs, String> {
    let mut prefs = load_prefs(&app);
    prefs.api_url = api_url.trim().trim_end_matches('/').to_string();
    save_prefs(&app, &prefs)?;
    if let Some(window) = app.get_webview_window(GESTION_LABEL) {
        inject_api_url(&window, &prefs.api_url);
    }
    Ok(prefs)
}
