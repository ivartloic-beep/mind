//! Prefs sync cloud — URL + token stockés hors repo (app data).

use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

pub const DEFAULT_BASE_URL: &str = "https://mind.louetline.fr";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncConfig {
    pub enabled: bool,
    pub base_url: String,
    pub token: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_sync_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            base_url: DEFAULT_BASE_URL.to_string(),
            token: String::new(),
            last_sync_at: None,
            last_error: None,
        }
    }
}

fn prefs_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("sync-prefs.json"))
}

pub fn load(app: &AppHandle) -> SyncConfig {
    let Ok(path) = prefs_path(app) else {
        return SyncConfig::default();
    };
    fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

pub fn save(app: &AppHandle, cfg: &SyncConfig) -> Result<(), String> {
    let path = prefs_path(app)?;
    let raw = serde_json::to_string_pretty(cfg).map_err(|e| e.to_string())?;
    fs::write(path, raw).map_err(|e| e.to_string())
}

/// Vue UI : masque le token (affiche seulement s’il est non vide via `hasToken`).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncConfigView {
    pub enabled: bool,
    pub base_url: String,
    pub has_token: bool,
    /// Token en clair uniquement pour édition locale (prefs machine).
    pub token: String,
    pub last_sync_at: Option<String>,
    pub last_error: Option<String>,
}

impl From<SyncConfig> for SyncConfigView {
    fn from(c: SyncConfig) -> Self {
        Self {
            enabled: c.enabled,
            base_url: c.base_url,
            has_token: !c.token.is_empty(),
            token: c.token,
            last_sync_at: c.last_sync_at,
            last_error: c.last_error,
        }
    }
}
