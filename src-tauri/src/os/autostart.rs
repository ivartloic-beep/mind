//! Autostart Windows — étape 12. Défaut ON ; prefs locales pour respecter le toggle.

use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use tauri_plugin_autostart::ManagerExt;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AutostartPrefs {
    /// Une fois true, on ne force plus le défaut ON.
    initialized: bool,
    enabled: bool,
}

impl Default for AutostartPrefs {
    fn default() -> Self {
        Self {
            initialized: false,
            enabled: true,
        }
    }
}

fn prefs_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("autostart-prefs.json"))
}

fn load_prefs(app: &AppHandle) -> AutostartPrefs {
    let Ok(path) = prefs_path(app) else {
        return AutostartPrefs::default();
    };
    fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

fn save_prefs(app: &AppHandle, prefs: &AutostartPrefs) -> Result<(), String> {
    let path = prefs_path(app)?;
    let raw = serde_json::to_string_pretty(prefs).map_err(|e| e.to_string())?;
    fs::write(path, raw).map_err(|e| e.to_string())
}

/// Initialise l’autostart : défaut ON au premier lancement, puis respecte la pref.
pub fn init_autostart(app: &AppHandle) {
    let mut prefs = load_prefs(app);
    let manager = app.autolaunch();

    if !prefs.initialized {
        match manager.enable() {
            Ok(()) => {
                prefs.enabled = true;
                prefs.initialized = true;
                let _ = save_prefs(app, &prefs);
                eprintln!("autostart: enabled (default ON)");
            }
            Err(err) => {
                eprintln!("autostart enable (default): {err}");
            }
        }
        return;
    }

    if prefs.enabled {
        if let Err(err) = manager.enable() {
            eprintln!("autostart enable: {err}");
        }
    } else if let Err(err) = manager.disable() {
        eprintln!("autostart disable: {err}");
    }
}

/// Synchronise la pref locale après un toggle UI.
#[tauri::command]
pub fn autostart_set_enabled(app: AppHandle, enabled: bool) -> Result<bool, String> {
    let manager = app.autolaunch();
    if enabled {
        manager.enable().map_err(|e| e.to_string())?;
    } else {
        manager.disable().map_err(|e| e.to_string())?;
    }
    let prefs = AutostartPrefs {
        initialized: true,
        enabled,
    };
    save_prefs(&app, &prefs)?;
    Ok(enabled)
}

#[tauri::command]
pub fn autostart_is_enabled(app: AppHandle) -> Result<bool, String> {
    let manager = app.autolaunch();
    manager.is_enabled().map_err(|e| e.to_string())
}
