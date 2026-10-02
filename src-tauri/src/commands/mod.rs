//! Commands Tauri — CRUD agrégats + capture directe Task/Note.

use tauri::{AppHandle, Emitter, State};

use crate::domain::{
    new_id, now_iso, DataChangedPayload, Note, NoteFilter, NoteKind, PostIt, PostItFilter, Project,
    Reminder, ReminderFilter, Task, TaskFilter, TaskStatus,
};
use crate::state::AppState;
use crate::storage::{Storage, StorageError};
use crate::sync;

fn emit_changed(app: &AppHandle, entity: &str, id: &str) -> Result<(), String> {
    app.emit(
        "data-changed",
        DataChangedPayload {
            entity: entity.to_string(),
            id: id.to_string(),
        },
    )
    .map_err(|e| e.to_string())
}

fn map_err(err: StorageError) -> String {
    err.to_string()
}

// --- Projects ---

#[tauri::command]
pub fn list_projects(state: State<'_, AppState>) -> Result<Vec<Project>, String> {
    state.storage.list_projects().map_err(map_err)
}

#[tauri::command]
pub fn get_project(state: State<'_, AppState>, id: String) -> Result<Option<Project>, String> {
    state.storage.get_project(&id).map_err(map_err)
}

#[tauri::command]
pub fn upsert_project(
    app: AppHandle,
    state: State<'_, AppState>,
    mut project: Project,
) -> Result<Project, String> {
    if project.id.is_empty() {
        project.id = new_id();
    }
    if project.created_at.is_empty() {
        project.created_at = now_iso();
    }
    state.storage.upsert_project(&project).map_err(map_err)?;
    emit_changed(&app, "project", &project.id)?;
    Ok(project)
}

#[tauri::command]
pub fn delete_project(app: AppHandle, state: State<'_, AppState>, id: String) -> Result<(), String> {
    state.storage.delete_project(&id).map_err(map_err)?;
    sync::schedule_remote_delete(&app, "projects", id.clone());
    emit_changed(&app, "project", &id)?;
    Ok(())
}

// --- Tasks ---

#[tauri::command]
pub fn list_tasks(
    state: State<'_, AppState>,
    filter: Option<TaskFilter>,
) -> Result<Vec<Task>, String> {
    state
        .storage
        .list_tasks(&filter.unwrap_or_default())
        .map_err(map_err)
}

#[tauri::command]
pub fn get_task(state: State<'_, AppState>, id: String) -> Result<Option<Task>, String> {
    state.storage.get_task(&id).map_err(map_err)
}

#[tauri::command]
pub fn upsert_task(
    app: AppHandle,
    state: State<'_, AppState>,
    mut task: Task,
) -> Result<Task, String> {
    let now = now_iso();
    if task.id.is_empty() {
        task.id = new_id();
    }
    if task.created_at.is_empty() {
        task.created_at = now.clone();
    }
    task.updated_at = now;
    state.storage.upsert_task(&task).map_err(map_err)?;
    emit_changed(&app, "task", &task.id)?;
    Ok(task)
}

#[tauri::command]
pub fn delete_task(app: AppHandle, state: State<'_, AppState>, id: String) -> Result<(), String> {
    state.storage.delete_task(&id).map_err(map_err)?;
    sync::schedule_remote_delete(&app, "tasks", id.clone());
    emit_changed(&app, "task", &id)?;
    Ok(())
}

/// Capture rapide → création directe d'une Task (pas d'InboxItem).
#[tauri::command]
pub fn create_task(
    app: AppHandle,
    state: State<'_, AppState>,
    title: String,
    project_id: Option<String>,
) -> Result<Task, String> {
    let now = now_iso();
    let task = Task {
        id: new_id(),
        title,
        status: TaskStatus::Active,
        project_id,
        created_at: now.clone(),
        updated_at: now,
        due_date: None,
        reminder: None,
        priority: None,
        notes: None,
    };
    state.storage.upsert_task(&task).map_err(map_err)?;
    emit_changed(&app, "task", &task.id)?;
    Ok(task)
}

/// Coche / décoche immédiate depuis le panneau.
#[tauri::command]
pub fn set_task_done(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    done: bool,
) -> Result<Task, String> {
    let mut task = state
        .storage
        .get_task(&id)
        .map_err(map_err)?
        .ok_or_else(|| format!("task not found: {id}"))?;
    task.status = if done {
        TaskStatus::Done
    } else {
        TaskStatus::Active
    };
    task.updated_at = now_iso();
    state.storage.upsert_task(&task).map_err(map_err)?;
    emit_changed(&app, "task", &task.id)?;
    Ok(task)
}

// --- Notes ---

#[tauri::command]
pub fn list_notes(
    state: State<'_, AppState>,
    filter: Option<NoteFilter>,
) -> Result<Vec<Note>, String> {
    state
        .storage
        .list_notes(&filter.unwrap_or_default())
        .map_err(map_err)
}

#[tauri::command]
pub fn get_note(state: State<'_, AppState>, id: String) -> Result<Option<Note>, String> {
    state.storage.get_note(&id).map_err(map_err)
}

#[tauri::command]
pub fn upsert_note(
    app: AppHandle,
    state: State<'_, AppState>,
    mut note: Note,
) -> Result<Note, String> {
    let now = now_iso();
    if note.id.is_empty() {
        note.id = new_id();
    }
    if note.created_at.is_empty() {
        note.created_at = now.clone();
    }
    note.updated_at = now;
    state.storage.upsert_note(&note).map_err(map_err)?;
    emit_changed(&app, "note", &note.id)?;
    Ok(note)
}

#[tauri::command]
pub fn delete_note(app: AppHandle, state: State<'_, AppState>, id: String) -> Result<(), String> {
    state.storage.delete_note(&id).map_err(map_err)?;
    sync::schedule_remote_delete(&app, "notes", id.clone());
    emit_changed(&app, "note", &id)?;
    Ok(())
}

/// Capture rapide / panneau → création directe d'une Note (kind note|idea).
#[tauri::command]
pub fn create_note(
    app: AppHandle,
    state: State<'_, AppState>,
    body: String,
    kind: NoteKind,
    project_id: Option<String>,
    title: Option<String>,
) -> Result<Note, String> {
    let now = now_iso();
    let title = title
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty());
    let note = Note {
        id: new_id(),
        title,
        body,
        kind,
        project_id,
        created_at: now.clone(),
        updated_at: now,
    };
    state.storage.upsert_note(&note).map_err(map_err)?;
    emit_changed(&app, "note", &note.id)?;
    Ok(note)
}

// --- Reminders ---

#[tauri::command]
pub fn list_reminders(
    state: State<'_, AppState>,
    filter: Option<ReminderFilter>,
) -> Result<Vec<Reminder>, String> {
    state
        .storage
        .list_reminders(&filter.unwrap_or_default())
        .map_err(map_err)
}

#[tauri::command]
pub fn get_reminder(state: State<'_, AppState>, id: String) -> Result<Option<Reminder>, String> {
    state.storage.get_reminder(&id).map_err(map_err)
}

#[tauri::command]
pub fn upsert_reminder(
    app: AppHandle,
    state: State<'_, AppState>,
    mut reminder: Reminder,
) -> Result<Reminder, String> {
    if reminder.id.is_empty() {
        reminder.id = new_id();
    }
    state.storage.upsert_reminder(&reminder).map_err(map_err)?;
    emit_changed(&app, "reminder", &reminder.id)?;
    Ok(reminder)
}

#[tauri::command]
pub fn delete_reminder(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    state.storage.delete_reminder(&id).map_err(map_err)?;
    sync::schedule_remote_delete(&app, "reminders", id.clone());
    emit_changed(&app, "reminder", &id)?;
    Ok(())
}

// --- PostIts ---

#[tauri::command]
pub fn list_postits(
    state: State<'_, AppState>,
    filter: Option<PostItFilter>,
) -> Result<Vec<PostIt>, String> {
    state
        .storage
        .list_postits(&filter.unwrap_or_default())
        .map_err(map_err)
}

#[tauri::command]
pub fn get_postit(state: State<'_, AppState>, id: String) -> Result<Option<PostIt>, String> {
    state.storage.get_postit(&id).map_err(map_err)
}

#[tauri::command]
pub fn upsert_postit(
    app: AppHandle,
    state: State<'_, AppState>,
    mut postit: PostIt,
) -> Result<PostIt, String> {
    if postit.id.is_empty() {
        postit.id = new_id();
    }
    state.storage.upsert_postit(&postit).map_err(map_err)?;
    emit_changed(&app, "postit", &postit.id)?;
    Ok(postit)
}

#[tauri::command]
pub fn delete_postit(app: AppHandle, state: State<'_, AppState>, id: String) -> Result<(), String> {
    state.storage.delete_postit(&id).map_err(map_err)?;
    sync::schedule_remote_delete(&app, "postits", id.clone());
    emit_changed(&app, "postit", &id)?;
    Ok(())
}

#[tauri::command]
pub fn ping() -> String {
    "pong".into()
}
