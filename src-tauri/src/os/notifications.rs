//! Rappels Windows — timer Rust + check `fireAt` manqués au démarrage.

use std::time::Duration;

use chrono::{DateTime, Duration as ChronoDuration, Local, Timelike, Utc};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_notification::NotificationExt;

use crate::domain::{new_id, now_iso, DataChangedPayload, Reminder, ReminderFilter};
use crate::state::AppState;
use crate::storage::Storage;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReminderDuePayload {
    pub reminder_id: String,
    pub task_id: String,
    pub task_title: String,
    pub fire_at: String,
    pub missed: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SnoozeKind {
    #[serde(rename = "10m")]
    TenMinutes,
    #[serde(rename = "1h")]
    OneHour,
    Tomorrow,
}

fn parse_iso(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}

fn effective_fire_at(reminder: &Reminder) -> Option<DateTime<Utc>> {
    reminder
        .snoozed_until
        .as_deref()
        .and_then(parse_iso)
        .or_else(|| parse_iso(&reminder.fire_at))
}

fn reminder_for_task(state: &AppState, task_id: &str) -> Result<Option<Reminder>, String> {
    let rows = state
        .storage
        .list_reminders(&ReminderFilter {
            target_id: Some(task_id.to_string()),
        })
        .map_err(|e| e.to_string())?;
    Ok(rows.into_iter().next())
}

fn sync_task_reminder_field(
    state: &AppState,
    task_id: &str,
    fire_at: Option<&str>,
) -> Result<(), String> {
    let Some(mut task) = state
        .storage
        .get_task(task_id)
        .map_err(|e| e.to_string())?
    else {
        return Ok(());
    };
    task.reminder = fire_at.map(|s| s.to_string());
    task.updated_at = now_iso();
    state.storage.upsert_task(&task).map_err(|e| e.to_string())
}

fn emit_data_changed(app: &AppHandle, entity: &str, id: &str) {
    let _ = app.emit(
        "data-changed",
        DataChangedPayload {
            entity: entity.into(),
            id: id.into(),
        },
    );
}

fn fire_reminder(app: &AppHandle, state: &AppState, reminder: &Reminder, missed: bool) {
    let Ok(mut fired) = state.fired_reminders.lock() else {
        return;
    };
    if fired.contains(&reminder.id) {
        return;
    }

    let task_title = state
        .storage
        .get_task(&reminder.target_id)
        .ok()
        .flatten()
        .map(|t| t.title)
        .unwrap_or_else(|| "Tâche".into());

    let body = if missed {
        format!("Rappel manqué : {task_title}")
    } else {
        format!("Rappel : {task_title}")
    };

    let _ = app
        .notification()
        .builder()
        .title("Ma Tête — Rappel")
        .body(&body)
        .show();

    let _ = app.emit(
        "reminder-due",
        ReminderDuePayload {
            reminder_id: reminder.id.clone(),
            task_id: reminder.target_id.clone(),
            task_title,
            fire_at: reminder
                .snoozed_until
                .clone()
                .unwrap_or_else(|| reminder.fire_at.clone()),
            missed,
        },
    );

    fired.insert(reminder.id.clone());
}

/// Vérifie les rappels dus / manqués et notifie.
pub fn check_due_reminders(app: &AppHandle) {
    let state = app.state::<AppState>();
    let Ok(reminders) = state.storage.list_reminders(&ReminderFilter::default()) else {
        return;
    };
    let now = Utc::now();
    for reminder in reminders {
        let Some(when) = effective_fire_at(&reminder) else {
            continue;
        };
        if when <= now {
            let missed = now - when > ChronoDuration::minutes(2);
            fire_reminder(app, &state, &reminder, missed);
        }
    }
}

/// Démarre le timer Rust (polling) + check immédiat au boot.
pub fn start_reminder_scheduler(app: AppHandle) {
    check_due_reminders(&app);
    tauri::async_runtime::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(20));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            interval.tick().await;
            check_due_reminders(&app);
        }
    });
}

/// Associe (ou met à jour) un rappel à une tâche.
#[tauri::command]
pub fn set_task_reminder(
    app: AppHandle,
    state: State<'_, AppState>,
    task_id: String,
    fire_at: String,
) -> Result<Reminder, String> {
    if state
        .storage
        .get_task(&task_id)
        .map_err(|e| e.to_string())?
        .is_none()
    {
        return Err(format!("task not found: {task_id}"));
    }
    parse_iso(&fire_at).ok_or_else(|| "fireAt invalide".to_string())?;

    let mut reminder = match reminder_for_task(&state, &task_id)? {
        Some(existing) => existing,
        None => Reminder {
            id: new_id(),
            target_id: task_id.clone(),
            fire_at: fire_at.clone(),
            snoozed_until: None,
        },
    };
    reminder.fire_at = fire_at.clone();
    reminder.snoozed_until = None;
    state
        .storage
        .upsert_reminder(&reminder)
        .map_err(|e| e.to_string())?;
    sync_task_reminder_field(&state, &task_id, Some(&fire_at))?;

    if let Ok(mut fired) = state.fired_reminders.lock() {
        fired.remove(&reminder.id);
    }

    emit_data_changed(&app, "reminder", &reminder.id);
    emit_data_changed(&app, "task", &task_id);
    Ok(reminder)
}

#[tauri::command]
pub fn clear_task_reminder(
    app: AppHandle,
    state: State<'_, AppState>,
    task_id: String,
) -> Result<(), String> {
    if let Some(reminder) = reminder_for_task(&state, &task_id)? {
        state
            .storage
            .delete_reminder(&reminder.id)
            .map_err(|e| e.to_string())?;
        if let Ok(mut fired) = state.fired_reminders.lock() {
            fired.remove(&reminder.id);
        }
        emit_data_changed(&app, "reminder", &reminder.id);
    }
    sync_task_reminder_field(&state, &task_id, None)?;
    emit_data_changed(&app, "task", &task_id);
    Ok(())
}

#[tauri::command]
pub fn snooze_reminder(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    kind: SnoozeKind,
) -> Result<Reminder, String> {
    let mut reminder = state
        .storage
        .get_reminder(&id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("reminder not found: {id}"))?;

    let next = match kind {
        SnoozeKind::TenMinutes => Utc::now() + ChronoDuration::minutes(10),
        SnoozeKind::OneHour => Utc::now() + ChronoDuration::hours(1),
        SnoozeKind::Tomorrow => {
            let local = Local::now() + ChronoDuration::days(1);
            local
                .with_hour(9)
                .and_then(|d| d.with_minute(0))
                .and_then(|d| d.with_second(0))
                .and_then(|d| d.with_nanosecond(0))
                .unwrap_or(local)
                .with_timezone(&Utc)
        }
    };

    let fire_at = next.to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    reminder.fire_at = fire_at.clone();
    reminder.snoozed_until = None;
    state
        .storage
        .upsert_reminder(&reminder)
        .map_err(|e| e.to_string())?;
    sync_task_reminder_field(&state, &reminder.target_id, Some(&fire_at))?;

    if let Ok(mut fired) = state.fired_reminders.lock() {
        fired.remove(&reminder.id);
    }

    emit_data_changed(&app, "reminder", &reminder.id);
    emit_data_changed(&app, "task", &reminder.target_id);
    Ok(reminder)
}

#[tauri::command]
pub fn dismiss_reminder(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    let reminder = state
        .storage
        .get_reminder(&id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("reminder not found: {id}"))?;
    let task_id = reminder.target_id.clone();
    state
        .storage
        .delete_reminder(&id)
        .map_err(|e| e.to_string())?;
    if let Ok(mut fired) = state.fired_reminders.lock() {
        fired.remove(&id);
    }
    sync_task_reminder_field(&state, &task_id, None)?;
    emit_data_changed(&app, "reminder", &id);
    emit_data_changed(&app, "task", &task_id);
    Ok(())
}

/// Ouvre le panneau (focus) pour une tâche rappelée.
#[tauri::command]
pub fn open_task_from_reminder(app: AppHandle, task_id: String) -> Result<(), String> {
    let _ = crate::windows::panel::panel_set_open(app.clone(), true);
    let _ = app.emit(
        "reminder-open-task",
        serde_json::json!({ "taskId": task_id }),
    );
    Ok(())
}
