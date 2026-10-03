//! Sync desktop ↔ mind-api (https://mind.louetline.fr).

mod client;
mod config;
mod engine;
mod map;

use std::time::Duration;

use tauri::AppHandle;

pub use config::{SyncConfig, SyncConfigView};
pub use engine::SyncReport;

#[tauri::command]
pub fn sync_get_config(app: AppHandle) -> SyncConfigView {
    config::load(&app).into()
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncConfigInput {
    pub enabled: bool,
    pub base_url: String,
    pub token: String,
}

#[tauri::command]
pub fn sync_set_config(app: AppHandle, input: SyncConfigInput) -> Result<SyncConfigView, String> {
    let prev = config::load(&app);
    let cfg = SyncConfig {
        enabled: input.enabled,
        base_url: if input.base_url.trim().is_empty() {
            config::DEFAULT_BASE_URL.to_string()
        } else {
            input.base_url.trim().to_string()
        },
        token: input.token,
        last_sync_at: prev.last_sync_at,
        last_error: None,
    };
    config::save(&app, &cfg)?;
    Ok(cfg.into())
}

#[tauri::command]
pub async fn sync_test(app: AppHandle) -> Result<String, String> {
    let cfg = config::load(&app);
    engine::test_connection(&cfg).await
}

#[tauri::command]
pub async fn sync_now(app: AppHandle) -> Result<SyncReport, String> {
    match engine::run_sync(&app).await {
        Ok(report) => Ok(report),
        Err(err) => {
            let mut cfg = config::load(&app);
            cfg.last_error = Some(err.clone());
            let _ = config::save(&app, &cfg);
            Err(err)
        }
    }
}

/// Sync périodique si activée (toutes les 5 s — aligné mobile / Gestion).
pub fn start_sync_scheduler(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(5));
        interval.tick().await; // skip immédiat
        loop {
            interval.tick().await;
            let cfg = config::load(&app);
            if !cfg.enabled || cfg.token.trim().is_empty() {
                continue;
            }
            if let Err(err) = engine::run_sync(&app).await {
                eprintln!("sync périodique: {err}");
                let mut cfg = config::load(&app);
                cfg.last_error = Some(err);
                let _ = config::save(&app, &cfg);
            }
        }
    });
}

/// Soft-delete distant après une suppression locale (si sync ON).
pub fn schedule_remote_delete(app: &AppHandle, entity: &'static str, id: String) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let cfg = config::load(&app);
        if !cfg.enabled || cfg.token.trim().is_empty() {
            return;
        }
        let Ok(client) = client::MindClient::from_config(&cfg) else {
            return;
        };
        if let Err(err) = client.delete(entity, &id).await {
            eprintln!("sync delete {entity}/{id}: {err}");
        }
    });
}
