//! Fenêtre shell Gestion — charge le front en ligne (ex. https://gestion.louetline.fr)
//! et pointe l’API PHP (`…/api`) pour MIND / personal_tasks.

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::webview::{DownloadEvent, NewWindowResponse};
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};
use url::Url;

static GESTION_DOC_WINDOW_SEQ: AtomicU64 = AtomicU64::new(1);

fn open_url_os(url: &str) -> Result<(), String> {
    open::that(url).map_err(|e| format!("ouverture: {e}"))
}

fn downloads_dir() -> PathBuf {
    if let Some(home) = std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
    {
        let d = PathBuf::from(home).join("Downloads");
        if d.is_dir() {
            return d;
        }
    }
    std::env::temp_dir()
}

fn filename_from_url(url: &Url) -> String {
    url.path_segments()
        .and_then(|mut s| s.next_back())
        .filter(|s| !s.is_empty() && *s != "workspace_file.php")
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            let id = url
                .query_pairs()
                .find(|(k, _)| k == "id")
                .map(|(_, v)| v.to_string())
                .unwrap_or_else(|| "document".into());
            format!("gestion-{id}")
        })
}

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

/// Patch docs (réappliqué souvent — le front Gestion peut redéfinir les fonctions).
const DOC_OPEN_PATCH: &str = r#"(function(){
  function mindInvoke(cmd, args) {
    try {
      var c = window.__TAURI__ && window.__TAURI__.core;
      if (c && typeof c.invoke === 'function') return c.invoke(cmd, args);
      if (window.__TAURI__ && typeof window.__TAURI__.invoke === 'function') {
        return window.__TAURI__.invoke(cmd, args);
      }
    } catch (e) {}
    return Promise.reject(new Error('IPC MIND indisponible'));
  }
  function toast(msg, kind) {
    try {
      if (typeof showToast === 'function') showToast(msg, kind || 'error');
      else console.warn(msg);
    } catch (e) { console.warn(msg); }
  }
  function idFromUrl(url) {
    try {
      var u = new URL(String(url), window.location.href);
      return u.searchParams.get('id');
    } catch (e) {
      var m = String(url).match(/[?&]id=([^&]+)/);
      return m ? decodeURIComponent(m[1]) : null;
    }
  }
  function patchDocFns() {
    window.openWpWorkspaceFile = function(elementId) {
      if (!elementId) return;
      mindInvoke('gestion_open_workspace_file', { id: String(elementId) }).catch(function(err) {
        toast('❌ Ouverture: ' + (err && err.message ? err.message : err));
      });
    };
    window.downloadWpWorkspaceFile = function(elementId) {
      if (!elementId) return;
      mindInvoke('gestion_download_workspace_file', { id: String(elementId) }).catch(function(err) {
        toast('❌ Téléchargement: ' + (err && err.message ? err.message : err));
      });
    };
    // PJ fiche tâche / uploads génériques (download.php) — blob: bloqué dans WebView.
    window.openUploadedFile = function(fileId, opts) {
      opts = opts || {};
      if (!fileId) { toast('❌ Fichier inaccessible'); return Promise.resolve(); }
      var name = opts.name || opts.fileName || null;
      var cmd = opts.download ? 'gestion_download_uploaded_file' : 'gestion_open_uploaded_file';
      return mindInvoke(cmd, { id: String(fileId), filename: name }).catch(function(err) {
        toast('❌ Fichier: ' + (err && err.message ? err.message : err));
      });
    };
    window.openTaskFicheDocument = function(docId) {
      try {
        var ctx = typeof getTaskFicheContext === 'function' ? getTaskFicheContext() : null;
        if (!ctx || !ctx.task) return;
        var doc = (ctx.task.documents || []).find(function(d) { return String(d.id) === String(docId); });
        if (!doc) return;
        var fileId = typeof taskFicheGetDocFileId === 'function' ? taskFicheGetDocFileId(doc) : null;
        if (!fileId) { toast('❌ Fichier inaccessible'); return; }
        window.openUploadedFile(fileId, {
          name: doc.fileName || doc.name || 'Fichier',
          type: doc.fileType || doc.mimeType || ''
        });
      } catch (e) { toast('❌ Fichier: ' + e); }
    };
  }
  patchDocFns();
  if (!window.__MIND_DOC_PATCH_TIMER__) {
    window.__MIND_DOC_PATCH_TIMER__ = setInterval(patchDocFns, 1500);
  }
  if (!window.__MIND_DOC_OPEN_HOOK__) {
    window.__MIND_DOC_OPEN_HOOK__ = true;
    window.__MIND_NATIVE_OPEN__ = window.open;
    window.open = function(url, target, features) {
      if (!url) return null;
      var s = String(url);
      if (s.indexOf('blob:') === 0) {
        toast('Aperçu bloqué dans MIND — utilise Ouvrir sur le fichier');
        return null;
      }
      if (s.indexOf('workspace_file.php') !== -1 || s.indexOf('/uploads/') !== -1
          || s.indexOf('download.php') !== -1) {
        var id = idFromUrl(s);
        var dl = s.indexOf('download=1') !== -1;
        if (id) {
          var isUpload = s.indexOf('download.php') !== -1;
          if (isUpload) {
            mindInvoke(dl ? 'gestion_download_uploaded_file' : 'gestion_open_uploaded_file', { id: id })
              .catch(function(err) { toast('❌ Fichier: ' + (err && err.message ? err.message : err)); });
          } else {
            mindInvoke(dl ? 'gestion_download_workspace_file' : 'gestion_open_workspace_file', { id: id })
              .catch(function(err) { toast('❌ Fichier: ' + (err && err.message ? err.message : err)); });
          }
          return null;
        }
        mindInvoke('open_external_url', { url: s }).catch(function(){});
        return null;
      }
      if (typeof window.__MIND_NATIVE_OPEN__ === 'function') {
        return window.__MIND_NATIVE_OPEN__.call(window, url, target, features);
      }
      return null;
    };
    document.addEventListener('click', function(ev) {
      try {
        var t = ev.target;
        if (!t || !t.closest) return;
        var btn = t.closest('button');
        if (btn) {
          var oc = btn.getAttribute('onclick') || '';
          var mOpen = oc.match(/openWpWorkspaceFile\('([^']+)'\)/);
          var mDl = oc.match(/downloadWpWorkspaceFile\('([^']+)'\)/);
          if (mOpen) {
            ev.preventDefault();
            if (ev.stopImmediatePropagation) ev.stopImmediatePropagation();
            else ev.stopPropagation();
            window.openWpWorkspaceFile(mOpen[1]);
            return;
          }
          if (mDl) {
            ev.preventDefault();
            if (ev.stopImmediatePropagation) ev.stopImmediatePropagation();
            else ev.stopPropagation();
            window.downloadWpWorkspaceFile(mDl[1]);
            return;
          }
        }
        // Clic direct sur aperçu (div onclick=openWpWorkspaceFile) — sans bouton.
        var preview = t.closest('[onclick*="openWpWorkspaceFile"], [onclick*="downloadWpWorkspaceFile"]');
        if (preview) {
          var poc = preview.getAttribute('onclick') || '';
          var pOpen = poc.match(/openWpWorkspaceFile\('([^']+)'\)/);
          var pDl = poc.match(/downloadWpWorkspaceFile\('([^']+)'\)/);
          if (pOpen) {
            ev.preventDefault();
            if (ev.stopImmediatePropagation) ev.stopImmediatePropagation();
            else ev.stopPropagation();
            window.openWpWorkspaceFile(pOpen[1]);
            return;
          }
          if (pDl) {
            ev.preventDefault();
            if (ev.stopImmediatePropagation) ev.stopImmediatePropagation();
            else ev.stopPropagation();
            window.downloadWpWorkspaceFile(pDl[1]);
            return;
          }
        }
        var ficheBtn = t.closest('button[onclick*="openTaskFicheDocument"]');
        if (ficheBtn) {
          var foc = ficheBtn.getAttribute('onclick') || '';
          var mFiche = foc.match(/openTaskFicheDocument\('([^']+)'\)/);
          if (mFiche) {
            ev.preventDefault();
            if (ev.stopImmediatePropagation) ev.stopImmediatePropagation();
            else ev.stopPropagation();
            window.openTaskFicheDocument(mFiche[1]);
            return;
          }
        }
        var a = t.closest('a[href*="workspace_file.php"], a[href*="/uploads/"], a[href*="download.php"]');
        if (a && a.href) {
          ev.preventDefault();
          if (ev.stopImmediatePropagation) ev.stopImmediatePropagation();
          else ev.stopPropagation();
          var id = idFromUrl(a.href);
          var dl = a.href.indexOf('download=1') !== -1 || a.hasAttribute('download');
          if (id) {
            var isUpload = a.href.indexOf('download.php') !== -1;
            if (isUpload) {
              mindInvoke(dl ? 'gestion_download_uploaded_file' : 'gestion_open_uploaded_file', { id: id })
                .catch(function(err) { toast('❌ Fichier: ' + (err && err.message ? err.message : err)); });
            } else {
              mindInvoke(dl ? 'gestion_download_workspace_file' : 'gestion_open_workspace_file', { id: id })
                .catch(function(err) { toast('❌ Fichier: ' + (err && err.message ? err.message : err)); });
            }
          } else {
            mindInvoke('open_external_url', { url: a.href }).catch(function(){});
          }
        }
      } catch (e) {}
    }, true);
  }
})();"#;

/// Script d’init (chaque navigation Gestion) — session + patch docs.
const SESSION_BRIDGE_INIT: &str = r#"(function(){
  if (!window.__MIND_SESSION_BRIDGE__) {
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
  }
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
  {docs}
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
        bridge = SESSION_BRIDGE_INIT,
        docs = DOC_OPEN_PATCH
    );
    let _ = window.eval(&js);
}

fn parse_front_url(api_url: &str) -> Result<Url, String> {
    let front = front_url_from_api(api_url);
    Url::parse(&front).map_err(|e| e.to_string())
}

fn ensure_window(app: &AppHandle, front: &Url) -> Result<WebviewWindow, String> {
    // Recréer si besoin d’attacher on_new_window / on_download (handlers au build seulement).
    if let Some(existing) = app.get_webview_window(GESTION_LABEL) {
        // Réinjecte le patch docs même sur fenêtre déjà ouverte.
        let prefs = load_prefs(app);
        inject_session_bridge(&existing, &prefs);
        return Ok(existing);
    }

    let app_for_new = app.clone();
    let app_for_docs = app.clone();
    let app_for_docs_dl = app.clone();
    let init_script = format!("{SESSION_BRIDGE_INIT}\n{DOC_OPEN_PATCH}");
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
    .initialization_script(init_script)
    // Docs projet / bureau : window.open → fetch Rust + ouverture OS (fallback sans IPC).
    .on_new_window(move |url, _features| {
        let url_str = url.to_string();
        if url_str.contains("workspace_file.php")
            || url_str.contains("/uploads/")
            || url_str.contains("download.php")
            || url_str.contains("download=1")
        {
            let id = url
                .query_pairs()
                .find(|(k, _)| k == "id")
                .map(|(_, v)| v.to_string());
            let download = url_str.contains("download=1");
            let is_upload = url_str.contains("download.php");
            if let Some(id) = id {
                let app = app_for_docs.clone();
                tauri::async_runtime::spawn(async move {
                    let cmd = if is_upload {
                        if download {
                            gestion_download_uploaded_file(app, id, None).await
                        } else {
                            gestion_open_uploaded_file(app, id, None).await
                        }
                    } else if download {
                        gestion_download_workspace_file(app, id).await
                    } else {
                        gestion_open_workspace_file(app, id).await
                    };
                    if let Err(err) = cmd {
                        eprintln!("[mind] doc open: {err}");
                    }
                });
            } else {
                let _ = open_url_os(&url_str);
            }
            return NewWindowResponse::Deny;
        }
        // Autre popup : petite fenêtre Tauri dédiée.
        let n = GESTION_DOC_WINDOW_SEQ.fetch_add(1, Ordering::Relaxed);
        let label = format!("gestion-doc-{n}");
        match WebviewWindowBuilder::new(
            &app_for_new,
            &label,
            WebviewUrl::External(url.clone()),
        )
        .title("MIND — Document")
        .inner_size(960.0, 720.0)
        .build()
        {
            Ok(window) => NewWindowResponse::Create { window },
            Err(_) => {
                let _ = open_url_os(&url_str);
                NewWindowResponse::Deny
            }
        }
    })
    // Téléchargements (Content-Disposition) → dossier Téléchargements.
    .on_download(move |_webview, event| {
        match event {
            DownloadEvent::Requested { url, destination } => {
                // Si c’est un workspace_file, on laisse le JS/IPC gérer (auth MIND).
                let url_str = url.to_string();
                if url_str.contains("workspace_file.php") || url_str.contains("download.php") {
                    if let Some(id) = url
                        .query_pairs()
                        .find(|(k, _)| k == "id")
                        .map(|(_, v)| v.to_string())
                    {
                        let app = app_for_docs_dl.clone();
                        let is_upload = url_str.contains("download.php");
                        tauri::async_runtime::spawn(async move {
                            let res = if is_upload {
                                gestion_download_uploaded_file(app, id, None).await
                            } else {
                                gestion_download_workspace_file(app, id).await
                            };
                            if let Err(err) = res {
                                eprintln!("[mind] download: {err}");
                            }
                        });
                        return false; // annule le download WebView (sans session)
                    }
                }
                let name = filename_from_url(&url);
                *destination = downloads_dir().join(name);
                true
            }
            DownloadEvent::Finished { success, path, .. } => {
                if success {
                    if let Some(path) = path {
                        let _ = open::that(path);
                    }
                }
                true
            }
            _ => true,
        }
    })
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

/// Ouvre une URL hors WebView (docs projet, téléchargements, liens).
#[tauri::command]
pub fn open_external_url(url: String) -> Result<(), String> {
    let url = url.trim();
    if url.is_empty() {
        return Err("URL vide".into());
    }
    if !(url.starts_with("https://")
        || url.starts_with("http://")
        || url.starts_with("blob:")
        || url.starts_with("file:"))
    {
        return Err("URL non autorisée".into());
    }
    // blob: ne s’ouvre pas via OS — ignorer poliment.
    if url.starts_with("blob:") {
        return Err("Aperçu blob non supporté hors WebView".into());
    }
    open_url_os(url)
}

async fn fetch_workspace_bytes(
    app: &AppHandle,
    id: &str,
) -> Result<(Vec<u8>, String), String> {
    // Tente d’abord la session prefs ; sinon tire depuis la fenêtre Gestion.
    if crate::gestion::try_client(app).is_none() {
        let _ = gestion_ensure_session(app.clone()).await?;
    }
    let client = crate::gestion::try_client(app)
        .ok_or_else(|| {
            "Session Gestion absente — ⚙ Paramètres → Se connecter".to_string()
        })?;
    let (bytes, filename, _mime) = client.fetch_workspace_file(id).await?;
    Ok((bytes, filename))
}

/// Ouvre un document workspace (projet / bureau) via l’OS (télécharge puis ouvre).
#[tauri::command]
pub async fn gestion_open_workspace_file(app: AppHandle, id: String) -> Result<(), String> {
    let (bytes, filename) = fetch_workspace_bytes(&app, &id).await?;
    let path = std::env::temp_dir().join(format!("mind-{}", filename));
    fs::write(&path, &bytes).map_err(|e| format!("écriture temp: {e}"))?;
    open::that(&path).map_err(|e| format!("ouverture: {e}"))
}

/// Télécharge un document workspace dans le dossier Téléchargements puis l’ouvre.
#[tauri::command]
pub async fn gestion_download_workspace_file(app: AppHandle, id: String) -> Result<(), String> {
    let (bytes, filename) = fetch_workspace_bytes(&app, &id).await?;
    write_downloads_and_open(&filename, &bytes)
}

async fn fetch_uploaded_bytes(
    app: &AppHandle,
    id: &str,
    filename: Option<&str>,
) -> Result<(Vec<u8>, String), String> {
    if crate::gestion::try_client(app).is_none() {
        let _ = gestion_ensure_session(app.clone()).await?;
    }
    let client = crate::gestion::try_client(app)
        .ok_or_else(|| {
            "Session Gestion absente — ⚙ Paramètres → Se connecter".to_string()
        })?;
    let (bytes, name, _mime) = client.fetch_uploaded_file(id, filename).await?;
    Ok((bytes, name))
}

fn write_downloads_and_open(filename: &str, bytes: &[u8]) -> Result<(), String> {
    let path = downloads_dir().join(filename);
    let path = if path.exists() {
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("fichier");
        let ext = path
            .extension()
            .and_then(|s| s.to_str())
            .map(|e| format!(".{e}"))
            .unwrap_or_default();
        downloads_dir().join(format!(
            "{}-{}{}",
            stem,
            chrono::Utc::now().timestamp(),
            ext
        ))
    } else {
        path
    };
    fs::write(&path, bytes).map_err(|e| format!("écriture: {e}"))?;
    open::that(&path).map_err(|e| format!("ouverture: {e}"))
}

/// Ouvre une PJ upload (`download.php`) via l’OS.
#[tauri::command]
pub async fn gestion_open_uploaded_file(
    app: AppHandle,
    id: String,
    filename: Option<String>,
) -> Result<(), String> {
    let (bytes, name) = fetch_uploaded_bytes(&app, &id, filename.as_deref()).await?;
    let path = std::env::temp_dir().join(format!("mind-{}", name));
    fs::write(&path, &bytes).map_err(|e| format!("écriture temp: {e}"))?;
    open::that(&path).map_err(|e| format!("ouverture: {e}"))
}

/// Télécharge une PJ upload dans Téléchargements puis l’ouvre.
#[tauri::command]
pub async fn gestion_download_uploaded_file(
    app: AppHandle,
    id: String,
    filename: Option<String>,
) -> Result<(), String> {
    let (bytes, name) = fetch_uploaded_bytes(&app, &id, filename.as_deref()).await?;
    write_downloads_and_open(&name, &bytes)
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

fn eval_retry(window: &WebviewWindow, js: String) {
    let win = window.clone();
    let js2 = js.clone();
    let _ = window.eval(&js);
    std::thread::spawn(move || {
        for delay in [400u64, 1000, 2000, 3500] {
            std::thread::sleep(std::time::Duration::from_millis(delay));
            let _ = win.eval(&js2);
        }
    });
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
    eval_retry(&window, js);
    Ok(())
}

/// Ouvre une note / idée workspace dans Gestion (bureau ou projet).
#[tauri::command]
pub fn gestion_show_note(
    app: AppHandle,
    note_id: String,
    project_id: Option<String>,
) -> Result<(), String> {
    let note_id = note_id.trim().to_string();
    if note_id.is_empty() {
        return Err("note_id vide".into());
    }
    show_gestion(&app)?;
    let window = app
        .get_webview_window(GESTION_LABEL)
        .ok_or_else(|| "fenêtre Gestion introuvable".to_string())?;
    let eid = note_id.replace('\\', "\\\\").replace('\'', "\\'");
    let pid = project_id
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.replace('\\', "\\\\").replace('\'', "\\'"));
    let js = if let Some(pid) = pid {
        format!(
            r#"(function(){{
  var eid = '{eid}';
  var pid = '{pid}';
  function tryOpen(n) {{
    try {{
      var go = function() {{
        if (typeof openWorkspaceElement === 'function') {{
          openWorkspaceElement(eid);
          return true;
        }}
        return false;
      }};
      if (typeof openWorkProjectPage === 'function') {{
        Promise.resolve(openWorkProjectPage(pid, {{ tab: 'notes' }})).then(function(){{
          setTimeout(go, 400);
        }}).catch(function(){{ go(); }});
        return;
      }}
      if (go()) return;
    }} catch (e) {{}}
    if (n < 48) setTimeout(function(){{ tryOpen(n + 1); }}, 250);
  }}
  tryOpen(0);
}})();"#,
            eid = eid,
            pid = pid
        )
    } else {
        format!(
            r#"(function(){{
  var eid = '{eid}';
  function tryOpen(n) {{
    try {{
      var go = function() {{
        if (typeof openWorkspaceElement === 'function') {{
          openWorkspaceElement(eid);
          return true;
        }}
        return false;
      }};
      if (typeof openMyBureauPage === 'function') {{
        Promise.resolve(openMyBureauPage()).then(function(){{
          setTimeout(go, 400);
        }}).catch(function(){{ go(); }});
        return;
      }}
      if (go()) return;
    }} catch (e) {{}}
    if (n < 48) setTimeout(function(){{ tryOpen(n + 1); }}, 250);
  }}
  tryOpen(0);
}})();"#,
            eid = eid
        )
    };
    eval_retry(&window, js);
    Ok(())
}

/// Ouvre la fiche prospect CRM dans Gestion.
#[tauri::command]
pub fn gestion_show_prospect(app: AppHandle, prospect_id: String) -> Result<(), String> {
    let prospect_id = prospect_id.trim().to_string();
    if prospect_id.is_empty() {
        return Err("prospect_id vide".into());
    }
    show_gestion(&app)?;
    let window = app
        .get_webview_window(GESTION_LABEL)
        .ok_or_else(|| "fenêtre Gestion introuvable".to_string())?;
    let escaped = prospect_id.replace('\\', "\\\\").replace('\'', "\\'");
    let js = format!(
        r#"(function(){{
  var id = '{id}';
  function tryOpen(n) {{
    try {{
      if (typeof openCrmPage === 'function') {{
        Promise.resolve(openCrmPage()).then(function(){{
          setTimeout(function(){{
            if (typeof openCrmProspectFiche === 'function') openCrmProspectFiche(id);
            else if (typeof editCrmProspect === 'function') editCrmProspect(id);
          }}, 500);
        }}).catch(function(){{}});
        return;
      }}
      if (typeof openCrmProspectFiche === 'function') {{
        openCrmProspectFiche(id);
        return;
      }}
    }} catch (e) {{}}
    if (n < 48) setTimeout(function(){{ tryOpen(n + 1); }}, 250);
  }}
  tryOpen(0);
}})();"#,
        id = escaped
    );
    eval_retry(&window, js);
    Ok(())
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GestionProspectReport {
    pub prospect_id: String,
}

/// Crée un prospect CRM express (3 champs).
#[tauri::command]
pub async fn gestion_create_prospect(
    app: AppHandle,
    name: String,
    organisme: Option<String>,
    contact: Option<String>,
) -> Result<GestionProspectReport, String> {
    if crate::gestion::try_client(&app).is_none() {
        let _ = gestion_ensure_session(app.clone()).await?;
    }
    let client = crate::gestion::try_client(&app)
        .ok_or_else(|| "Session Gestion absente — ⚙ Paramètres → Se connecter".to_string())?;
    let id = client
        .create_prospect_express(
            &name,
            organisme.as_deref(),
            contact.as_deref(),
        )
        .await?;
    Ok(GestionProspectReport { prospect_id: id })
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GestionAttachReport {
    pub task_id: String,
    pub filename: String,
    pub file_id: String,
}

async fn attach_bytes_to_task(
    app: &AppHandle,
    task_id: &str,
    filename: &str,
    mime: Option<&str>,
    bytes: Vec<u8>,
) -> Result<GestionAttachReport, String> {
    if crate::gestion::try_client(app).is_none() {
        let _ = gestion_ensure_session(app.clone()).await?;
    }
    let client = crate::gestion::try_client(app)
        .ok_or_else(|| "Session Gestion absente — ⚙ Paramètres → Se connecter".to_string())?;
    let uploaded = client
        .upload_generic_file(filename, mime, bytes, "documents")
        .await?;
    let file_id = uploaded
        .get("file_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "upload sans file_id".to_string())?
        .to_string();
    let file_name = uploaded
        .get("original_name")
        .and_then(|v| v.as_str())
        .unwrap_or(filename)
        .to_string();
    let file_type = uploaded
        .get("mime_type")
        .and_then(|v| v.as_str())
        .unwrap_or(mime.unwrap_or("application/octet-stream"))
        .to_string();
    let file_size = uploaded.get("size").cloned().unwrap_or(Value::Null);
    let download_url = uploaded
        .get("url")
        .and_then(|v| v.as_str())
        .map(|u| {
            if u.starts_with("http") {
                u.to_string()
            } else {
                format!(
                    "{}/{}",
                    client_base_url(app),
                    u.trim_start_matches('/')
                )
            }
        })
        .unwrap_or_else(|| {
            format!(
                "{}/download.php?id={}",
                client_base_url(app),
                file_id
            )
        });

    let state = app.state::<crate::state::AppState>();
    let mut task = crate::gestion::get_task_hybrid(app, &state, task_id)
        .await?
        .ok_or_else(|| format!("Tâche introuvable: {task_id}"))?;
    let doc_name = std::path::Path::new(&file_name)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(&file_name)
        .to_string();
    let doc = serde_json::json!({
        "id": format!("tdoc_{}", crate::domain::new_id()),
        "name": doc_name,
        "file_id": file_id.clone(),
        "url": download_url.clone(),
        "downloadUrl": download_url,
        "fileName": file_name.clone(),
        "fileType": file_type,
        "fileSize": file_size,
        "uploadedAt": crate::domain::now_iso(),
    });
    task.documents.insert(0, doc);
    crate::gestion::upsert_task_hybrid(app, &state, task).await?;
    Ok(GestionAttachReport {
        task_id: task_id.to_string(),
        filename: file_name,
        file_id,
    })
}

fn client_base_url(app: &AppHandle) -> String {
    let prefs = load_prefs(app);
    prefs.api_url.trim().trim_end_matches('/').to_string()
}

/// Attache un fichier (base64) comme PJ d’une tâche personal_tasks.
#[tauri::command]
pub async fn gestion_attach_file_to_task(
    app: AppHandle,
    task_id: String,
    filename: String,
    mime: Option<String>,
    data_base64: String,
) -> Result<GestionAttachReport, String> {
    use base64::Engine;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(data_base64.trim())
        .map_err(|e| format!("base64: {e}"))?;
    if bytes.is_empty() {
        return Err("Fichier vide".into());
    }
    if bytes.len() > 25 * 1024 * 1024 {
        return Err("Fichier trop volumineux (max 25 Mo)".into());
    }
    attach_bytes_to_task(
        &app,
        task_id.trim(),
        &sanitize_upload_filename(&filename),
        mime.as_deref(),
        bytes,
    )
    .await
}

/// Attache un fichier OS comme PJ d’une tâche.
#[tauri::command]
pub async fn gestion_attach_file_path_to_task(
    app: AppHandle,
    task_id: String,
    path: String,
) -> Result<GestionAttachReport, String> {
    let path_buf = std::path::PathBuf::from(path.trim());
    if !path_buf.is_file() {
        return Err("Ce n’est pas un fichier".into());
    }
    let meta = fs::metadata(&path_buf).map_err(|e| e.to_string())?;
    if meta.len() > 25 * 1024 * 1024 {
        return Err("Fichier trop volumineux (max 25 Mo)".into());
    }
    let bytes = fs::read(&path_buf).map_err(|e| format!("lecture: {e}"))?;
    let filename = path_buf
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("fichier.bin");
    attach_bytes_to_task(
        &app,
        task_id.trim(),
        &sanitize_upload_filename(filename),
        Some(&mime_from_path(&path_buf)),
        bytes,
    )
    .await
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
