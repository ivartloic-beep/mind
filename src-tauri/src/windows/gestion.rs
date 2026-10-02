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
    /// Token session gestion (login.php).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth_token: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_id: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_name: Option<String>,
    /// Horodatage du dernier import one-shot SQLite → personal_tasks.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tasks_migrated_at: Option<String>,
    /// Dernière erreur API (mode cache / hors-ligne).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
}

impl Default for GestionPrefs {
    fn default() -> Self {
        Self {
            api_url: String::new(),
            auth_token: None,
            user_id: None,
            user_name: None,
            tasks_migrated_at: None,
            last_error: None,
        }
    }
}

impl GestionPrefs {
    pub fn is_logged_in(&self) -> bool {
        self.auth_token
            .as_deref()
            .map(|t| !t.trim().is_empty())
            .unwrap_or(false)
            && !self.api_url.trim().is_empty()
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

pub fn save_prefs(app: &AppHandle, prefs: &GestionPrefs) -> Result<(), String> {
    let path = prefs_path(app)?;
    let raw = serde_json::to_string_pretty(prefs).map_err(|e| e.to_string())?;
    fs::write(path, raw).map_err(|e| e.to_string())
}

fn inject_prefs(window: &WebviewWindow, prefs: &GestionPrefs) {
    let api_url = prefs.api_url.trim();
    if api_url.is_empty() && prefs.auth_token.is_none() {
        return;
    }
    let escaped_url = api_url.replace('\\', "\\\\").replace('\'', "\\'");
    let token = prefs.auth_token.clone().unwrap_or_default();
    let escaped_token = token.replace('\\', "\\\\").replace('\'', "\\'");
    let js = format!(
        r#"(function(){{
  try {{
    if ('{url}') {{
      localStorage.setItem('mind_gestion_api_url', '{url}');
      window.API_URL = '{url}';
      window.__MIND_GESTION_API_URL__ = '{url}';
    }}
    if ('{token}') {{
      localStorage.setItem('authToken', '{token}');
      window.authToken = '{token}';
    }}
  }} catch (e) {{}}
}})();"#,
        url = escaped_url,
        token = escaped_token
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

/// Ouvre / focus la fenêtre Gestion et injecte l’URL API + session.
pub fn show_gestion(app: &AppHandle) -> Result<(), String> {
    // Empêche la capture always-on-top invisible de bloquer clavier/souris.
    crate::windows::capture::release_capture_overlay(app);

    let prefs = load_prefs(app);
    let window = ensure_window(app)?;
    inject_prefs(&window, &prefs);
    let _ = window.unminimize();
    window.show().map_err(|e| e.to_string())?;
    window.set_focus().map_err(|e| e.to_string())?;
    // Ré-injecte après chargement (localStorage prêt) + re-focus WebView.
    let prefs2 = prefs.clone();
    let win = window.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(400));
        inject_prefs(&win, &prefs2);
        let _ = win.set_focus();
        std::thread::sleep(std::time::Duration::from_millis(200));
        let _ = win.set_focus();
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
        inject_prefs(&window, &prefs);
    }
    Ok(prefs)
}

#[tauri::command]
pub async fn gestion_login(
    app: AppHandle,
    username: String,
    password: String,
) -> Result<GestionPrefs, String> {
    let prefs = crate::gestion::login(&app, username, password).await?;
    if let Some(window) = app.get_webview_window(GESTION_LABEL) {
        inject_prefs(&window, &prefs);
    }
    Ok(prefs)
}

#[tauri::command]
pub fn gestion_logout(app: AppHandle) -> Result<GestionPrefs, String> {
    let prefs = crate::gestion::logout(&app)?;
    if let Some(window) = app.get_webview_window(GESTION_LABEL) {
        let _ = window.eval(
            r#"(function(){try{localStorage.removeItem('authToken');window.authToken=null;}catch(e){}})();"#,
        );
    }
    Ok(prefs)
}

#[tauri::command]
pub fn gestion_set_session(
    app: AppHandle,
    token: String,
    user_id: Option<i64>,
    user_name: Option<String>,
) -> Result<GestionPrefs, String> {
    crate::gestion::set_session(&app, token, user_id, user_name)
}

#[tauri::command]
pub async fn gestion_migrate_local_tasks(app: AppHandle) -> Result<crate::gestion::MigrateReport, String> {
    let state = app.state::<crate::state::AppState>();
    crate::gestion::migrate_local_tasks(&app, &state).await
}

#[tauri::command]
pub fn gestion_tasks_backend_active(app: AppHandle) -> bool {
    crate::gestion::tasks_backend_active(&app)
}
