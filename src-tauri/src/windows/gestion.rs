//! Fenêtre shell Gestion — charge le front en ligne (ex. https://gestion.louetline.fr)
//! et pointe l’API PHP (`…/api`) pour MIND / personal_tasks.

use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};
use url::Url;

pub const GESTION_LABEL: &str = "gestion";
/// Défaut Loïc / louetline — overridable via prefs.
pub const DEFAULT_API_URL: &str = "https://gestion.louetline.fr/api";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GestionPrefs {
    /// URL absolue de l’API PHP, ex. https://gestion.louetline.fr/api
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
            api_url: DEFAULT_API_URL.to_string(),
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

    pub fn ensure_api_url(&mut self) {
        if self.api_url.trim().is_empty() {
            self.api_url = DEFAULT_API_URL.to_string();
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
    let mut prefs: GestionPrefs = fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default();
    prefs.ensure_api_url();
    prefs
}

pub fn save_prefs(app: &AppHandle, prefs: &GestionPrefs) -> Result<(), String> {
    let path = prefs_path(app)?;
    let raw = serde_json::to_string_pretty(prefs).map_err(|e| e.to_string())?;
    fs::write(path, raw).map_err(|e| e.to_string())
}

/// `https://gestion.louetline.fr/api` → `https://gestion.louetline.fr/`
pub fn front_url_from_api(api_url: &str) -> String {
    let trimmed = api_url.trim().trim_end_matches('/');
    let base = trimmed
        .strip_suffix("/api")
        .unwrap_or(trimmed)
        .trim_end_matches('/');
    if base.is_empty() {
        "https://gestion.louetline.fr".into()
    } else {
        base.to_string()
    }
}

fn release_input_blockers(app: &AppHandle) {
    crate::windows::capture::release_capture_overlay(app);
    // Panneau / post-its always-on-top peuvent aussi voler clics/clavier.
    if let Some(panel) = app.get_webview_window(crate::windows::panel::PANEL_LABEL) {
        let _ = panel.set_always_on_top(false);
    }
    for (label, window) in app.webview_windows() {
        if label.starts_with(crate::windows::postit::LABEL_PREFIX) {
            let _ = window.set_always_on_top(false);
        }
    }
}

/// Script d’init (chaque navigation Gestion) — pousse authToken → prefs MIND.
const SESSION_BRIDGE_INIT: &str = r#"(function(){
  if (window.__MIND_SESSION_BRIDGE__) return;
  window.__MIND_SESSION_BRIDGE__ = true;
  function mindInvoke(cmd, args) {
    try {
      var c = window.__TAURI__ && window.__TAURI__.core;
      if (c && typeof c.invoke === 'function') return c.invoke(cmd, args);
      if (window.__TAURI__ && typeof window.__TAURI__.invoke === 'function') {
        return window.__TAURI__.invoke(cmd, args);
      }
    } catch (e) {}
    return null;
  }
  function pushSession() {
    try {
      var t = localStorage.getItem('authToken') || window.authToken || '';
      var u = localStorage.getItem('currentUser') || '';
      var userId = null, userName = null;
      if (u) {
        try {
          var o = JSON.parse(u);
          if (o && o.id != null && !isNaN(Number(o.id))) userId = Number(o.id);
          if (o) userName = ((o.prenom||'')+' '+(o.nom||'')).trim() || o.username || null;
        } catch (e2) {}
      }
      mindInvoke('gestion_set_session', { token: t || '', userId: userId, userName: userName });
    } catch (e) {}
  }
  var last = '';
  setInterval(function(){
    try {
      var t = localStorage.getItem('authToken') || '';
      var u = localStorage.getItem('currentUser') || '';
      var cur = t + '|' + u;
      if (cur === last) return;
      last = cur;
      pushSession();
    } catch (e) {}
  }, 1000);
  setTimeout(pushSession, 150);
  setTimeout(pushSession, 800);
  setTimeout(pushSession, 2000);
})();"#;

fn inject_session_bridge(window: &WebviewWindow, prefs: &GestionPrefs) {
    let token = prefs.auth_token.clone().unwrap_or_default();
    let escaped_token = token.replace('\\', "\\\\").replace('\'', "\\'");
    let api = prefs.api_url.replace('\\', "\\\\").replace('\'', "\\'");
    // Prefs → localStorage (si panneau déjà connecté), puis (ré)installe le pont.
    let js = format!(
        r#"(function(){{
  try {{
    if ('{api}') {{
      try {{ localStorage.setItem('mind_gestion_api_url', '{api}'); }} catch (e) {{}}
      window.__MIND_GESTION_API_URL__ = '{api}';
    }}
    if ('{token}') {{
      try {{
        localStorage.setItem('authToken', '{token}');
        window.authToken = '{token}';
      }} catch (e) {{}}
    }}
  }} catch (e) {{}}
  {bridge}
  try {{
    var t = localStorage.getItem('authToken') || window.authToken || '';
    var u = localStorage.getItem('currentUser') || '';
    var userId = null, userName = null;
    if (u) {{
      try {{
        var o = JSON.parse(u);
        if (o && o.id != null && !isNaN(Number(o.id))) userId = Number(o.id);
        if (o) userName = ((o.prenom||'')+' '+(o.nom||'')).trim() || o.username || null;
      }} catch (e2) {{}}
    }}
    var c = window.__TAURI__ && window.__TAURI__.core;
    if (c && c.invoke) c.invoke('gestion_set_session', {{ token: t || '', userId: userId, userName: userName }});
  }} catch (e) {{}}
}})();"#,
        api = api,
        token = escaped_token,
        bridge = SESSION_BRIDGE_INIT
    );
    let _ = window.eval(&js);
}

fn parse_front_url(api_url: &str) -> Result<Url, String> {
    let front = front_url_from_api(api_url);
    Url::parse(&front).map_err(|e| e.to_string())
}

fn ensure_window(app: &AppHandle, front: &Url) -> Result<WebviewWindow, String> {
    if let Some(existing) = app.get_webview_window(GESTION_LABEL) {
        // Navigue vers le front distant (évite le shell local embarqué qui bloque la saisie).
        let _ = existing.navigate(front.clone());
        return Ok(existing);
    }

    let window = WebviewWindowBuilder::new(
        app,
        GESTION_LABEL,
        WebviewUrl::External(front.clone()),
    )
    .title("MIND — Gestion")
    .inner_size(1280.0, 840.0)
    .min_inner_size(960.0, 640.0)
    .resizable(true)
    .decorations(true)
    .skip_taskbar(false)
    .focused(true)
    .center()
    .visible(false)
    .initialization_script(SESSION_BRIDGE_INIT)
    .build()
    .map_err(|e| e.to_string())?;

    Ok(window)
}

/// Ouvre / focus la fenêtre Gestion (front distant) et synchronise la session.
pub fn show_gestion(app: &AppHandle) -> Result<(), String> {
    release_input_blockers(app);

    let mut prefs = load_prefs(app);
    prefs.ensure_api_url();
    save_prefs(app, &prefs)?;

    let front = parse_front_url(&prefs.api_url)?;
    let window = ensure_window(app, &front)?;
    inject_session_bridge(&window, &prefs);
    let _ = window.unminimize();
    window.show().map_err(|e| e.to_string())?;
    window.set_focus().map_err(|e| e.to_string())?;

    let prefs2 = prefs.clone();
    let win = window.clone();
    std::thread::spawn(move || {
        for delay in [300u64, 800, 1600] {
            std::thread::sleep(std::time::Duration::from_millis(delay));
            inject_session_bridge(&win, &prefs2);
            let _ = win.set_focus();
        }
    });
    Ok(())
}

pub fn init_gestion(app: &AppHandle, background: bool) -> Result<(), String> {
    let mut prefs = load_prefs(app);
    prefs.ensure_api_url();
    let _ = save_prefs(app, &prefs);
    let front = parse_front_url(&prefs.api_url)?;
    let _ = ensure_window(app, &front)?;
    if !background {
        show_gestion(app)?;
    }
    Ok(())
}

#[tauri::command]
pub fn gestion_show(app: AppHandle) -> Result<(), String> {
    show_gestion(&app)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GestionUploadReport {
    pub filename: String,
    pub visibility: String,
    pub project_id: Option<String>,
    pub element_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DroppedFilePayload {
    pub filename: String,
    pub mime: String,
    pub data_base64: String,
    pub size: u64,
}

fn mime_from_path(path: &std::path::Path) -> String {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .as_deref()
    {
        Some("pdf") => "application/pdf",
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("txt") => "text/plain",
        Some("csv") => "text/csv",
        Some("json") => "application/json",
        Some("doc") => "application/msword",
        Some("docx") => {
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
        }
        Some("xls") => "application/vnd.ms-excel",
        Some("xlsx") => {
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
        }
        Some("ppt") => "application/vnd.ms-powerpoint",
        Some("pptx") => {
            "application/vnd.openxmlformats-officedocument.presentationml.presentation"
        }
        Some("zip") => "application/zip",
        Some("mp3") => "audio/mpeg",
        Some("mp4") => "video/mp4",
        _ => "application/octet-stream",
    }
    .to_string()
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DroppedFileMeta {
    pub filename: String,
    pub mime: String,
    pub size: u64,
    pub path: String,
}

/// Métadonnées d’un fichier déposé (sans charger le contenu).
#[tauri::command]
pub fn peek_dropped_file(path: String) -> Result<DroppedFileMeta, String> {
    let path_buf = std::path::PathBuf::from(path.trim());
    if !path_buf.is_file() {
        return Err("Ce n’est pas un fichier".into());
    }
    let meta = fs::metadata(&path_buf).map_err(|e| e.to_string())?;
    if meta.len() > 25 * 1024 * 1024 {
        return Err("Fichier trop volumineux (max 25 Mo)".into());
    }
    if meta.len() == 0 {
        return Err("Fichier vide".into());
    }
    let filename = path_buf
        .file_name()
        .and_then(|n| n.to_str())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| "Nom de fichier manquant".to_string())?
        .to_string();
    Ok(DroppedFileMeta {
        mime: mime_from_path(&path_buf),
        size: meta.len(),
        path: path_buf.to_string_lossy().into_owned(),
        filename,
    })
}

/// Lit un fichier déposé (drag-and-drop Tauri → chemins OS) pour l’upload Gestion.
#[tauri::command]
pub fn read_dropped_file(path: String) -> Result<DroppedFilePayload, String> {
    use base64::Engine;
    let path = std::path::PathBuf::from(path.trim());
    if !path.is_file() {
        return Err("Ce n’est pas un fichier".into());
    }
    let meta = fs::metadata(&path).map_err(|e| e.to_string())?;
    if meta.len() > 25 * 1024 * 1024 {
        return Err("Fichier trop volumineux (max 25 Mo)".into());
    }
    if meta.len() == 0 {
        return Err("Fichier vide".into());
    }
    let bytes = fs::read(&path).map_err(|e| format!("lecture: {e}"))?;
    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| "Nom de fichier manquant".to_string())?
        .to_string();
    Ok(DroppedFilePayload {
        mime: mime_from_path(&path),
        data_base64: base64::engine::general_purpose::STANDARD.encode(&bytes),
        size: bytes.len() as u64,
        filename,
    })
}

fn sanitize_upload_filename(name: &str) -> String {
    let trimmed = name.trim();
    let base = std::path::Path::new(trimmed)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(trimmed);
    let cleaned: String = base
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-' | ' ') {
                c
            } else {
                '_'
            }
        })
        .collect();
    let cleaned = cleaned.trim().trim_matches('.');
    if cleaned.is_empty() {
        "fichier.bin".into()
    } else {
        cleaned.to_string()
    }
}

async fn upload_bytes_to_gestion(
    app: &AppHandle,
    filename: &str,
    mime: Option<&str>,
    bytes: Vec<u8>,
    visibility: &str,
    project_id: Option<&str>,
) -> Result<GestionUploadReport, String> {
    let client = crate::gestion::try_client(app)
        .ok_or_else(|| "Connecte-toi à Gestion pour déposer un fichier".to_string())?;
    let vis = visibility.trim();
    if vis != "personal" && vis != "team" {
        return Err("visibility doit être personal ou team".into());
    }
    let pid = project_id
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());
    if vis == "team" && pid.is_none() {
        return Err("Choisis un projet pour déposer en team".into());
    }
    if bytes.is_empty() {
        return Err("Fichier vide".into());
    }
    if bytes.len() > 25 * 1024 * 1024 {
        return Err("Fichier trop volumineux (max 25 Mo)".into());
    }
    let name = sanitize_upload_filename(filename);
    let parsed = client
        .upload_workspace_file(
            &name,
            mime,
            bytes,
            vis,
            pid.as_deref(),
        )
        .await?;
    let element_id = parsed
        .get("element")
        .and_then(|e| e.get("id"))
        .and_then(|id| {
            id.as_str()
                .map(|s| s.to_string())
                .or_else(|| id.as_i64().map(|n| n.to_string()))
        });
    Ok(GestionUploadReport {
        filename: name,
        visibility: vis.to_string(),
        project_id: pid,
        element_id,
    })
}

/// Dépose un fichier (base64) dans le workspace Gestion (bureau ou projet).
#[tauri::command]
pub async fn gestion_upload_file(
    app: AppHandle,
    filename: String,
    mime: Option<String>,
    data_base64: String,
    visibility: String,
    project_id: Option<String>,
) -> Result<GestionUploadReport, String> {
    use base64::Engine;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(data_base64.trim())
        .map_err(|e| format!("base64: {e}"))?;
    upload_bytes_to_gestion(
        &app,
        &filename,
        mime.as_deref(),
        bytes,
        &visibility,
        project_id.as_deref(),
    )
    .await
}

/// Dépose un fichier depuis un chemin OS (drag-and-drop) — évite le aller-retour base64.
#[tauri::command]
pub async fn gestion_upload_file_path(
    app: AppHandle,
    path: String,
    visibility: String,
    project_id: Option<String>,
) -> Result<GestionUploadReport, String> {
    let path = std::path::PathBuf::from(path.trim());
    if !path.is_file() {
        return Err("Ce n’est pas un fichier".into());
    }
    let meta = fs::metadata(&path).map_err(|e| e.to_string())?;
    if meta.len() > 25 * 1024 * 1024 {
        return Err("Fichier trop volumineux (max 25 Mo)".into());
    }
    let bytes = fs::read(&path).map_err(|e| format!("lecture: {e}"))?;
    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("fichier.bin");
    let mime = mime_from_path(&path);
    upload_bytes_to_gestion(
        &app,
        filename,
        Some(mime.as_str()),
        bytes,
        &visibility,
        project_id.as_deref(),
    )
    .await
}

/// Ouvre Gestion sur une page (bureau | crm | projects).
#[tauri::command]
pub fn gestion_show_page(app: AppHandle, page: String) -> Result<(), String> {
    let page = page.trim().to_ascii_lowercase();
    let fn_name = match page.as_str() {
        "bureau" => "openMyBureauPage",
        "crm" => "openCrmPage",
        "projects" | "projets" | "production" => "openWorkProjectsListPage",
        _ => {
            return Err(
                "page Gestion inconnue (bureau, crm, projects)".into(),
            )
        }
    };
    show_gestion(&app)?;
    let window = app
        .get_webview_window(GESTION_LABEL)
        .ok_or_else(|| "fenêtre Gestion introuvable".to_string())?;
    let js = format!(
        r#"(function(){{
  var fn = '{fn}';
  function tryOpen(n) {{
    try {{
      if (typeof window[fn] === 'function') {{
        window[fn]();
        return;
      }}
    }} catch (e) {{}}
    if (n < 48) setTimeout(function(){{ tryOpen(n + 1); }}, 250);
  }}
  tryOpen(0);
}})();"#,
        fn = fn_name
    );
    let win = window.clone();
    let js2 = js.clone();
    let _ = window.eval(&js);
    std::thread::spawn(move || {
        for delay in [400u64, 1000, 2000, 3500] {
            std::thread::sleep(std::time::Duration::from_millis(delay));
            let _ = win.eval(&js2);
        }
    });
    Ok(())
}

/// Ouvre Gestion sur la fiche tâche (documents, notes, activités… comme le bureau).
#[tauri::command]
pub fn gestion_show_task(app: AppHandle, task_id: String) -> Result<(), String> {
    let task_id = task_id.trim().to_string();
    if task_id.is_empty() {
        return Err("task_id vide".into());
    }
    show_gestion(&app)?;
    let window = app
        .get_webview_window(GESTION_LABEL)
        .ok_or_else(|| "fenêtre Gestion introuvable".to_string())?;
    let escaped = task_id.replace('\\', "\\\\").replace('\'', "\\'");
    let js = format!(
        r#"(function(){{
  var id = '{id}';
  function tryOpen(n) {{
    try {{
      if (typeof openTaskFiche === 'function') {{
        Promise.resolve(openTaskFiche('personal', '', id)).catch(function(){{}});
        return;
      }}
    }} catch (e) {{}}
    if (n < 48) setTimeout(function(){{ tryOpen(n + 1); }}, 250);
  }}
  tryOpen(0);
}})();"#,
        id = escaped
    );
    let win = window.clone();
    let js2 = js.clone();
    let _ = window.eval(&js);
    std::thread::spawn(move || {
        for delay in [400u64, 1000, 2000, 3500] {
            std::thread::sleep(std::time::Duration::from_millis(delay));
            let _ = win.eval(&js2);
        }
    });
    Ok(())
}

#[tauri::command]
pub fn gestion_get_config(app: AppHandle) -> Result<GestionPrefs, String> {
    Ok(load_prefs(&app))
}

#[tauri::command]
pub fn gestion_set_config(app: AppHandle, api_url: String) -> Result<GestionPrefs, String> {
    let mut prefs = load_prefs(&app);
    prefs.api_url = api_url.trim().trim_end_matches('/').to_string();
    prefs.ensure_api_url();
    save_prefs(&app, &prefs)?;
    if let Some(window) = app.get_webview_window(GESTION_LABEL) {
        if let Ok(front) = parse_front_url(&prefs.api_url) {
            let _ = window.navigate(front);
        }
        inject_session_bridge(&window, &prefs);
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
        inject_session_bridge(&window, &prefs);
    }
    // Tire projets + notes bureau dans le cache local (panneau / capture).
    let state = app.state::<crate::state::AppState>();
    if let Ok(n) = crate::gestion::sync_projects_after_login(&app, &state).await {
        let _ = app.emit(
            "data-changed",
            crate::domain::DataChangedPayload {
                entity: "project".into(),
                id: format!("sync:{n}"),
            },
        );
        let _ = app.emit(
            "data-changed",
            crate::domain::DataChangedPayload {
                entity: "note".into(),
                id: "sync:bureau".into(),
            },
        );
    }
    Ok(prefs)
}

#[tauri::command]
pub fn gestion_logout(app: AppHandle) -> Result<GestionPrefs, String> {
    let prefs = crate::gestion::logout(&app)?;
    if let Some(window) = app.get_webview_window(GESTION_LABEL) {
        let _ = window.eval(
            r#"(function(){try{localStorage.removeItem('authToken');localStorage.removeItem('currentUser');window.authToken=null;}catch(e){}})();"#,
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
    let prefs = crate::gestion::set_session(&app, token, user_id, user_name)?;
    if prefs.auth_token.as_deref().map(|t| !t.is_empty()).unwrap_or(false) {
        crate::gestion::schedule_sync(&app);
        // Le panneau doit rafraîchir `gestionLoggedIn` (dépôt fichier, sync…).
        let _ = app.emit(
            "data-changed",
            crate::domain::DataChangedPayload {
                entity: "sync".into(),
                id: "session".into(),
            },
        );
    }
    Ok(prefs)
}

#[tauri::command]
pub async fn gestion_migrate_local_tasks(
    app: AppHandle,
) -> Result<crate::gestion::MigrateReport, String> {
    let state = app.state::<crate::state::AppState>();
    crate::gestion::migrate_local_tasks(&app, &state).await
}

#[tauri::command]
pub fn gestion_tasks_backend_active(app: AppHandle) -> bool {
    crate::gestion::tasks_backend_active(&app)
}

/// S’assure qu’une session prefs existe — tire le token depuis la fenêtre Gestion si besoin.
#[tauri::command]
pub async fn gestion_ensure_session(app: AppHandle) -> Result<GestionPrefs, String> {
    if crate::gestion::try_client(&app).is_some() {
        return Ok(load_prefs(&app));
    }
    let prefs = load_prefs(&app);
    let Some(window) = app.get_webview_window(GESTION_LABEL) else {
        return Ok(prefs);
    };
    inject_session_bridge(&window, &prefs);
    let _ = window.eval(
        r#"(function(){
  try {
    var t = localStorage.getItem('authToken') || window.authToken || '';
    var u = localStorage.getItem('currentUser') || '';
    var userId = null, userName = null;
    if (u) {
      try {
        var o = JSON.parse(u);
        if (o && o.id != null && !isNaN(Number(o.id))) userId = Number(o.id);
        if (o) userName = ((o.prenom||'')+' '+(o.nom||'')).trim() || o.username || null;
      } catch (e2) {}
    }
    var c = window.__TAURI__ && window.__TAURI__.core;
    if (c && c.invoke) {
      c.invoke('gestion_set_session', { token: t || '', userId: userId, userName: userName });
    }
  } catch (e) {}
})();"#,
    );
    for _ in 0..30 {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        if crate::gestion::try_client(&app).is_some() {
            return Ok(load_prefs(&app));
        }
    }
    Ok(load_prefs(&app))
}

/// Sync bidirectionnel immédiat (panneau ↔ Gestion).
#[tauri::command]
pub async fn gestion_sync_now(
    app: AppHandle,
) -> Result<crate::gestion::SyncReport, String> {
    let state = app.state::<crate::state::AppState>();
    let report = crate::gestion::sync_bidirectional(&app, &state).await?;
    if report.active {
        let _ = app.emit(
            "data-changed",
            crate::domain::DataChangedPayload {
                entity: "sync".into(),
                id: format!(
                    "gestion:{}:{}:{}",
                    report.tasks, report.projects, report.notes
                ),
            },
        );
        let _ = app.emit(
            "data-changed",
            crate::domain::DataChangedPayload {
                entity: "task".into(),
                id: "pull".into(),
            },
        );
        let _ = app.emit(
            "data-changed",
            crate::domain::DataChangedPayload {
                entity: "note".into(),
                id: "pull".into(),
            },
        );
        let _ = app.emit(
            "data-changed",
            crate::domain::DataChangedPayload {
                entity: "project".into(),
                id: "pull".into(),
            },
        );
    }
    Ok(report)
}

/// Après fermeture / masquage de la fenêtre Gestion : re-tire les données.
pub fn on_gestion_hidden(app: &AppHandle) {
    crate::gestion::schedule_sync(app);
}
