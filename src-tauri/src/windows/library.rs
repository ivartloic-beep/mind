//! Fenêtre Bibliothèque — vue complète tâches / notes.

use tauri::{AppHandle, Manager, WebviewWindow};

pub const LIBRARY_LABEL: &str = "library";

fn library_window(app: &AppHandle) -> Result<WebviewWindow, String> {
    app.get_webview_window(LIBRARY_LABEL)
        .ok_or_else(|| "fenêtre library introuvable".into())
}

#[tauri::command]
pub fn library_show(app: AppHandle) -> Result<(), String> {
    let window = library_window(&app)?;
    let _ = window.unminimize();
    window.show().map_err(|e| e.to_string())?;
    window.set_focus().map_err(|e| e.to_string())?;
    Ok(())
}
