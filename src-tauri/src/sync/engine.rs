//! Pull puis push — last PUT wins (desktop prioritaire après pull).

use chrono::Utc;
use tauri::{AppHandle, Emitter, Manager};

use crate::domain::{
    DataChangedPayload, NoteFilter, PostItFilter, ReminderFilter, TaskFilter,
};
use crate::state::AppState;
use crate::storage::Storage;

use super::client::MindClient;
use super::config::{self, SyncConfig};
use super::map::{
    note_from_cloud, note_to_cloud, postit_from_cloud, postit_to_cloud, project_from_cloud,
    project_to_cloud, reminder_from_cloud, reminder_to_cloud, task_from_cloud, task_to_cloud,
    CloudNote, CloudPostIt, CloudProject, CloudReminder, CloudTask,
};

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncReport {
    pub pulled: u32,
    pub pushed: u32,
    pub deleted_remote: u32,
    pub last_sync_at: String,
}

pub async fn run_sync(app: &AppHandle) -> Result<SyncReport, String> {
    let mut cfg = config::load(app);
    if !cfg.enabled {
        return Err("sync désactivée".into());
    }
    let client = MindClient::from_config(&cfg)?;
    let since = cfg.last_sync_at.clone();

    let mut pulled = 0u32;
    let mut deleted_remote = 0u32;

    // --- PULL ---
    pulled += pull_projects(app, &client, since.as_deref(), &mut deleted_remote).await?;
    pulled += pull_tasks(app, &client, since.as_deref(), &mut deleted_remote).await?;
    pulled += pull_notes(app, &client, since.as_deref(), &mut deleted_remote).await?;
    pulled += pull_reminders(app, &client, since.as_deref(), &mut deleted_remote).await?;
    pulled += pull_postits(app, &client, since.as_deref(), &mut deleted_remote).await?;

    // --- PUSH (état local gagne) ---
    let mut pushed = 0u32;
    pushed += push_projects(app, &client).await?;
    pushed += push_tasks(app, &client).await?;
    pushed += push_notes(app, &client).await?;
    pushed += push_reminders(app, &client).await?;
    pushed += push_postits(app, &client).await?;

    let now = Utc::now().to_rfc3339();
    cfg.last_sync_at = Some(now.clone());
    cfg.last_error = None;
    config::save(app, &cfg)?;

    let _ = app.emit(
        "data-changed",
        DataChangedPayload {
            entity: "sync".into(),
            id: now.clone(),
        },
    );

    Ok(SyncReport {
        pulled,
        pushed,
        deleted_remote,
        last_sync_at: now,
    })
}

pub async fn test_connection(cfg: &SyncConfig) -> Result<String, String> {
    let client = MindClient::from_config(cfg)?;
    let health = client.health().await?;
    // Vérifie aussi l’auth
    let _: Vec<CloudProject> = client.list("projects", None).await?;
    Ok(health
        .get("service")
        .and_then(|v| v.as_str())
        .unwrap_or("ok")
        .to_string())
}

async fn pull_projects(
    app: &AppHandle,
    client: &MindClient,
    since: Option<&str>,
    deleted: &mut u32,
) -> Result<u32, String> {
    let items: Vec<CloudProject> = client.list("projects", since).await?;
    let state = app.state::<AppState>();
    let mut n = 0u32;
    for item in items {
        if item.deleted_at.is_some() {
            let _ = state.storage.delete_project(&item.id);
            *deleted += 1;
        } else {
            state
                .storage
                .upsert_project(&project_from_cloud(&item))
                .map_err(|e| e.to_string())?;
            n += 1;
        }
    }
    Ok(n)
}

async fn pull_tasks(
    app: &AppHandle,
    client: &MindClient,
    since: Option<&str>,
    deleted: &mut u32,
) -> Result<u32, String> {
    let items: Vec<CloudTask> = client.list("tasks", since).await?;
    let state = app.state::<AppState>();
    let mut n = 0u32;
    for item in items {
        if item.deleted_at.is_some() {
            let _ = state.storage.delete_task(&item.id);
            *deleted += 1;
        } else {
            state
                .storage
                .upsert_task(&task_from_cloud(&item))
                .map_err(|e| e.to_string())?;
            n += 1;
        }
    }
    Ok(n)
}

async fn pull_notes(
    app: &AppHandle,
    client: &MindClient,
    since: Option<&str>,
    deleted: &mut u32,
) -> Result<u32, String> {
    let items: Vec<CloudNote> = client.list("notes", since).await?;
    let state = app.state::<AppState>();
    let mut n = 0u32;
    for item in items {
        if item.deleted_at.is_some() {
            let _ = state.storage.delete_note(&item.id);
            *deleted += 1;
        } else {
            state
                .storage
                .upsert_note(&note_from_cloud(&item))
                .map_err(|e| e.to_string())?;
            n += 1;
        }
    }
    Ok(n)
}

async fn pull_reminders(
    app: &AppHandle,
    client: &MindClient,
    since: Option<&str>,
    deleted: &mut u32,
) -> Result<u32, String> {
    let items: Vec<CloudReminder> = client.list("reminders", since).await?;
    let state = app.state::<AppState>();
    let mut n = 0u32;
    for item in items {
        if item.deleted_at.is_some() || item.done {
            let _ = state.storage.delete_reminder(&item.id);
            *deleted += 1;
        } else if let Some(r) = reminder_from_cloud(&item) {
            state
                .storage
                .upsert_reminder(&r)
                .map_err(|e| e.to_string())?;
            n += 1;
        }
    }
    Ok(n)
}

async fn pull_postits(
    app: &AppHandle,
    client: &MindClient,
    since: Option<&str>,
    deleted: &mut u32,
) -> Result<u32, String> {
    let items: Vec<CloudPostIt> = client.list("postits", since).await?;
    let state = app.state::<AppState>();
    let mut n = 0u32;
    for item in items {
        if item.deleted_at.is_some() {
            let _ = state.storage.delete_postit(&item.id);
            *deleted += 1;
        } else {
            state
                .storage
                .upsert_postit(&postit_from_cloud(&item))
                .map_err(|e| e.to_string())?;
            n += 1;
        }
    }
    Ok(n)
}

async fn push_projects(app: &AppHandle, client: &MindClient) -> Result<u32, String> {
    let state = app.state::<AppState>();
    let rows = state.storage.list_projects().map_err(|e| e.to_string())?;
    let mut n = 0u32;
    for row in rows {
        client.put("projects", &row.id, &project_to_cloud(&row)).await?;
        n += 1;
    }
    Ok(n)
}

async fn push_tasks(app: &AppHandle, client: &MindClient) -> Result<u32, String> {
    let state = app.state::<AppState>();
    let rows = state
        .storage
        .list_tasks(&TaskFilter::default())
        .map_err(|e| e.to_string())?;
    let mut n = 0u32;
    for row in rows {
        client.put("tasks", &row.id, &task_to_cloud(&row)).await?;
        n += 1;
    }
    Ok(n)
}

async fn push_notes(app: &AppHandle, client: &MindClient) -> Result<u32, String> {
    let state = app.state::<AppState>();
    let rows = state
        .storage
        .list_notes(&NoteFilter::default())
        .map_err(|e| e.to_string())?;
    let mut n = 0u32;
    for row in rows {
        client.put("notes", &row.id, &note_to_cloud(&row)).await?;
        n += 1;
    }
    Ok(n)
}

async fn push_reminders(app: &AppHandle, client: &MindClient) -> Result<u32, String> {
    let state = app.state::<AppState>();
    let rows = state
        .storage
        .list_reminders(&ReminderFilter::default())
        .map_err(|e| e.to_string())?;
    let mut n = 0u32;
    for row in rows {
        client
            .put("reminders", &row.id, &reminder_to_cloud(&row))
            .await?;
        n += 1;
    }
    Ok(n)
}

async fn push_postits(app: &AppHandle, client: &MindClient) -> Result<u32, String> {
    let state = app.state::<AppState>();
    let rows = state
        .storage
        .list_postits(&PostItFilter::default())
        .map_err(|e| e.to_string())?;
    let mut n = 0u32;
    for row in rows {
        client.put("postits", &row.id, &postit_to_cloud(&row)).await?;
        n += 1;
    }
    Ok(n)
}
