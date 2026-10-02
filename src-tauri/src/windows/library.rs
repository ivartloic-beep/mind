//! Fenêtre Bibliothèque — vue complète tâches / notes.

use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

pub const LIBRARY_LABEL: &str = "library";
const LIBRARY_W: f64 = 920.0;
const LIBRARY_H: f64 = 700.0;

fn ensure_library(app: &AppHandle) -> Result<WebviewWindow, String> {
    if let Some(existing) = app.get_webview_window(LIBRARY_LABEL) {
        return Ok(existing);
    }

    WebviewWindowBuilder::new(app, LIBRARY_LABEL, WebviewUrl::App("library.html".into()))
        .title("MIND — Bibliothèque")
        .inner_size(LIBRARY_W, LIBRARY_H)
        .resizable(true)
        .decorations(true)
        .skip_taskbar(false)
        .center()
        .visible(true)
        .build()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn library_show(app: AppHandle) -> Result<(), String> {
    let window = ensure_library(&app)?;
    let _ = window.unminimize();
    window.show().map_err(|e| e.to_string())?;
    window.set_focus().map_err(|e| e.to_string())?;
    Ok(())
}
