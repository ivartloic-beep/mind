//! Fenêtre capture rapide — pré-créée cachée, show/focus/clear à l'ouverture.

use serde::Serialize;
use tauri::{AppHandle, Emitter, LogicalPosition, LogicalSize, Manager, WebviewWindow};

pub const CAPTURE_LABEL: &str = "capture";
const CAPTURE_WIDTH: f64 = 440.0;
const CAPTURE_HEIGHT: f64 = 168.0;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureOpenedPayload {
    pub clear: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
}

fn capture_window(app: &AppHandle) -> Result<WebviewWindow, String> {
    app.get_webview_window(CAPTURE_LABEL)
        .ok_or_else(|| "fenêtre capture introuvable".into())
}

fn center_on_monitor(window: &WebviewWindow) -> Result<(), String> {
    let monitor = window
        .current_monitor()
        .map_err(|e| e.to_string())?
        .or_else(|| window.primary_monitor().ok().flatten())
        .ok_or_else(|| "aucun moniteur".to_string())?;
    let scale = monitor.scale_factor();
    let area = monitor.work_area();
    let wx = area.position.x as f64 / scale;
    let wy = area.position.y as f64 / scale;
    let ww = area.size.width as f64 / scale;
    let wh = area.size.height as f64 / scale;
    let x = wx + ((ww - CAPTURE_WIDTH) / 2.0).max(0.0);
    let y = wy + ((wh - CAPTURE_HEIGHT) / 3.0).max(0.0);
    window
        .set_size(LogicalSize::new(CAPTURE_WIDTH, CAPTURE_HEIGHT))
        .map_err(|e| e.to_string())?;
    window
        .set_position(LogicalPosition::new(x, y))
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Prépare la fenêtre (cachée, always-on-top, légère) au démarrage.
pub fn init_capture(app: &AppHandle) -> Result<(), String> {
    let window = capture_window(app)?;
    let _ = window.set_decorations(false);
    let _ = window.set_resizable(false);
    let _ = window.set_skip_taskbar(true);
    window
        .set_always_on_top(true)
        .map_err(|e| e.to_string())?;
    center_on_monitor(&window)?;
    let _ = window.hide();
    Ok(())
}

#[tauri::command]
pub fn capture_show(app: AppHandle) -> Result<(), String> {
    capture_show_kind(app, None)
}

/// Ouvre la capture, optionnellement pré-sélectionne le type (task|note|idea).
pub fn capture_show_kind(app: AppHandle, kind: Option<String>) -> Result<(), String> {
    let window = capture_window(&app)?;
    center_on_monitor(&window)?;
    window
        .set_always_on_top(true)
        .map_err(|e| e.to_string())?;
    window.show().map_err(|e| e.to_string())?;
    window.set_focus().map_err(|e| e.to_string())?;
    app.emit(
        "capture-opened",
        CaptureOpenedPayload {
            clear: true,
            kind,
        },
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn capture_hide(app: AppHandle) -> Result<(), String> {
    let window = capture_window(&app)?;
    window.hide().map_err(|e| e.to_string())?;
    Ok(())
}
