//! Panneau latéral — ancrage bord droit, slide open/close, always-on-top mémorisé.

use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, LogicalPosition, LogicalSize, Manager, WebviewWindow};

pub const PANEL_LABEL: &str = "panel";
pub const PANEL_CONTENT_WIDTH: f64 = 400.0;
/// Bandeau gauche du panneau ouvert (fermer).
pub const HANDLE_WIDTH: f64 = 28.0;
/// Onglet flottant coin haut-droit quand le panneau est fermé.
pub const CLOSED_TAB_W: f64 = 40.0;
pub const CLOSED_TAB_H: f64 = 40.0;
pub const CLOSED_TAB_MARGIN: f64 = 12.0;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PanelPrefs {
    pub open: bool,
    pub always_on_top: bool,
}

impl Default for PanelPrefs {
    fn default() -> Self {
        Self {
            // Étape 12 : boot discret — ouvrir via tray / CTRL+ALT+Espace.
            open: false,
            always_on_top: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PanelState {
    pub open: bool,
    pub always_on_top: bool,
    pub content_width: f64,
    pub handle_width: f64,
}

fn prefs_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("panel-prefs.json"))
}

pub fn load_prefs(app: &AppHandle) -> PanelPrefs {
    let Ok(path) = prefs_path(app) else {
        return PanelPrefs::default();
    };
    fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

fn save_prefs(app: &AppHandle, prefs: &PanelPrefs) -> Result<(), String> {
    let path = prefs_path(app)?;
    let raw = serde_json::to_string_pretty(prefs).map_err(|e| e.to_string())?;
    fs::write(path, raw).map_err(|e| e.to_string())
}

fn panel_window(app: &AppHandle) -> Result<WebviewWindow, String> {
    app.get_webview_window(PANEL_LABEL)
        .ok_or_else(|| "fenêtre panel introuvable".into())
}

/// Zone de travail du moniteur courant (x, y, w, h) en logical pixels.
fn work_area_logical(window: &WebviewWindow) -> Result<(f64, f64, f64, f64), String> {
    let monitor = window
        .current_monitor()
        .map_err(|e| e.to_string())?
        .or_else(|| window.primary_monitor().ok().flatten())
        .ok_or_else(|| "aucun moniteur".to_string())?;
    let scale = monitor.scale_factor();
    let area = monitor.work_area();
    Ok((
        area.position.x as f64 / scale,
        area.position.y as f64 / scale,
        area.size.width as f64 / scale,
        area.size.height as f64 / scale,
    ))
}

pub fn apply_panel_geometry(window: &WebviewWindow, open: bool) -> Result<(), String> {
    let (wx, wy, ww, wh) = work_area_logical(window)?;
    if open {
        let width = PANEL_CONTENT_WIDTH;
        let x = wx + ww - width;
        window
            .set_size(LogicalSize::new(width, wh))
            .map_err(|e| e.to_string())?;
        window
            .set_position(LogicalPosition::new(x, wy))
            .map_err(|e| e.to_string())?;
    } else {
        // Petite flèche flottante en haut à droite (plus de barre pleine hauteur).
        let x = wx + ww - CLOSED_TAB_W - CLOSED_TAB_MARGIN;
        let y = wy + CLOSED_TAB_MARGIN;
        window
            .set_size(LogicalSize::new(CLOSED_TAB_W, CLOSED_TAB_H))
            .map_err(|e| e.to_string())?;
        window
            .set_position(LogicalPosition::new(x, y))
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Initialise le panneau. Si `background` (autostart), démarre fermé / sans focus.
pub fn init_panel(app: &AppHandle, background: bool) -> Result<(), String> {
    let mut prefs = load_prefs(app);
    if background {
        prefs.open = false;
    }
    let window = panel_window(app)?;

    let _ = window.set_decorations(false);
    let _ = window.set_resizable(false);
    let _ = window.set_skip_taskbar(true);
    let _ = window.set_shadow(false);
    window
        .set_always_on_top(prefs.always_on_top)
        .map_err(|e| e.to_string())?;
    apply_panel_geometry(&window, prefs.open)?;
    let _ = save_prefs(app, &prefs);

    if background {
        // Autostart : app en arrière-plan, pas de flash panneau.
        let _ = window.hide();
    } else {
        let _ = window.show();
    }
    Ok(())
}

#[tauri::command]
pub fn panel_get_state(app: AppHandle) -> Result<PanelState, String> {
    let prefs = load_prefs(&app);
    Ok(PanelState {
        open: prefs.open,
        always_on_top: prefs.always_on_top,
        content_width: PANEL_CONTENT_WIDTH,
        handle_width: HANDLE_WIDTH,
    })
}

#[tauri::command]
pub fn panel_set_open(app: AppHandle, open: bool) -> Result<PanelState, String> {
    let mut prefs = load_prefs(&app);
    prefs.open = open;
    save_prefs(&app, &prefs)?;
    let window = panel_window(&app)?;
    apply_panel_geometry(&window, open)?;
    let state = PanelState {
        open: prefs.open,
        always_on_top: prefs.always_on_top,
        content_width: PANEL_CONTENT_WIDTH,
        handle_width: HANDLE_WIDTH,
    };
    let _ = app.emit("panel-state-changed", &state);
    Ok(state)
}

#[tauri::command]
pub fn panel_set_always_on_top(app: AppHandle, always_on_top: bool) -> Result<PanelState, String> {
    let mut prefs = load_prefs(&app);
    prefs.always_on_top = always_on_top;
    save_prefs(&app, &prefs)?;
    let window = panel_window(&app)?;
    window
        .set_always_on_top(always_on_top)
        .map_err(|e| e.to_string())?;
    Ok(PanelState {
        open: prefs.open,
        always_on_top: prefs.always_on_top,
        content_width: PANEL_CONTENT_WIDTH,
        handle_width: HANDLE_WIDTH,
    })
}

/// Resync géométrie (ex. changement de moniteur) — garde l'état open courant.
#[tauri::command]
pub fn panel_redock(app: AppHandle) -> Result<PanelState, String> {
    let prefs = load_prefs(&app);
    let window = panel_window(&app)?;
    apply_panel_geometry(&window, prefs.open)?;
    Ok(PanelState {
        open: prefs.open,
        always_on_top: prefs.always_on_top,
        content_width: PANEL_CONTENT_WIDTH,
        handle_width: HANDLE_WIDTH,
    })
}
