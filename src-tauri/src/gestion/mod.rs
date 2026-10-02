//! Client API gestion-v2 — tâches (personal_tasks), projets, notes bureau (workspace).

mod client;

pub use client::{GestionClient, MigrateReport};

use tauri::{AppHandle, Manager};

use crate::domain::{
    now_iso, CreateTaskInput, Note, NoteFilter, NoteKind, Project, Task, TaskFilter, TaskPriority,
    TaskStatus,
};
use crate::state::AppState;
use crate::storage::Storage;
use crate::windows::gestion::{load_prefs, save_prefs, GestionPrefs};

use self::client::{
    production_projects_to_mind, remove_work_project_value, upsert_work_project_value,
    work_project_to_mind, workspace_element_to_note,
};

pub fn try_client(app: &AppHandle) -> Option<GestionClient> {
    let prefs = load_prefs(app);
    if prefs.api_url.trim().is_empty() {
        return None;
    }
    let token = prefs.auth_token.as_deref().unwrap_or("").trim();
    if token.is_empty() {
        return None;
    }
    GestionClient::new(&prefs.api_url, token).ok()
}

pub fn tasks_backend_active(app: &AppHandle) -> bool {
    try_client(app).is_some()
}

fn set_last_error(app: &AppHandle, err: Option<String>) {
    let mut prefs = load_prefs(app);
    prefs.last_error = err;
    let _ = save_prefs(app, &prefs);
}

fn link_category_to_project(task: &mut Task, projects: &[Project]) {
    if task.project_id.is_some() {
        return;
    }
    if task.category.trim().is_empty() {
        return;
    }
    let cat = task.category.trim();
    if let Some(p) = projects
        .iter()
        .find(|p| p.name.eq_ignore_ascii_case(cat))
    {
        task.project_id = Some(p.id.clone());
    }
}

/// Convertit une tâche API gestion → modèle MIND, en préservant reminder/project locaux.
pub fn merge_remote_with_local(
    remote: Task,
    local: Option<&Task>,
    projects: &[Project],
) -> Task {
    let mut task = remote;
    if let Some(local) = local {
        if task.project_id.is_none() {
            task.project_id = local.project_id.clone();
        }
        if task.reminder.is_none() {
            task.reminder = local.reminder.clone();
        }
        if task.category.is_empty() {
            if let Some(pid) = &local.project_id {
                if let Some(p) = projects.iter().find(|p| &p.id == pid) {
                    task.category = p.name.clone();
                }
            }
        }
    }
    link_category_to_project(&mut task, projects);
    task.normalize();
    task
}

pub fn apply_local_filter(tasks: Vec<Task>, filter: &TaskFilter) -> Vec<Task> {
    tasks
        .into_iter()
        .filter(|t| {
            if let Some(true) = filter.no_project {
                if t.project_id.is_some() {
                    return false;
                }
            } else if let Some(ref pid) = filter.project_id {
                if t.project_id.as_deref() != Some(pid.as_str()) {
                    return false;
                }
            }
            if let Some(done) = filter.done {
                if t.status.is_done() != done {
                    return false;
                }
            }
            true
        })
        .collect()
}

pub async fn list_tasks_hybrid(
    app: &AppHandle,
    state: &AppState,
    filter: TaskFilter,
) -> Result<Vec<Task>, String> {
    let Some(client) = try_client(app) else {
        return state.storage.list_tasks(&filter).map_err(|e| e.to_string());
    };

    match client.list_tasks(None).await {
        Ok(remote) => {
            use std::collections::HashSet;
            set_last_error(app, None);
            let projects = list_projects_hybrid(app, state).await.unwrap_or_default();
            let mut remote_ids = HashSet::new();
            let mut merged = Vec::with_capacity(remote.len());
            for r in remote {
                remote_ids.insert(r.id.clone());
                let local = state.storage.get_task(&r.id).map_err(|e| e.to_string())?;
                let task = merge_remote_with_local(r, local.as_ref(), &projects);
                state.storage.upsert_task(&task).map_err(|e| e.to_string())?;
                merged.push(task);
            }

            // Gestion → panneau : si absent de personal_tasks, retirer du cache
            // (évite de ressusciter une tâche supprimée dans Gestion).
            // Panneau → Gestion reste assuré par create/upsert/delete.
            let locals = state
                .storage
                .list_tasks(&TaskFilter::default())
                .map_err(|e| e.to_string())?;
            for local in locals {
                if !remote_ids.contains(&local.id) {
                    let _ = state.storage.delete_task(&local.id);
                }
            }

            Ok(apply_local_filter(merged, &filter))
        }
        Err(err) => {
            // Hors-ligne : servir le cache SQLite.
            set_last_error(app, Some(format!("Cache local — {err}")));
            state.storage.list_tasks(&filter).map_err(|e| e.to_string())
        }
    }
}

pub async fn get_task_hybrid(
    app: &AppHandle,
    state: &AppState,
    id: &str,
) -> Result<Option<Task>, String> {
    if try_client(app).is_none() {
        return state.storage.get_task(id).map_err(|e| e.to_string());
    }
    let tasks = list_tasks_hybrid(app, state, TaskFilter::default()).await?;
    Ok(tasks.into_iter().find(|t| t.id == id))
}

pub async fn upsert_task_hybrid(
    app: &AppHandle,
    state: &AppState,
    mut task: Task,
) -> Result<Task, String> {
    let now = now_iso();
    if task.id.is_empty() {
        task.id = crate::domain::new_id();
    }
    if task.created_at.is_empty() {
        task.created_at = now.clone();
    }
    task.updated_at = now;
    task.normalize();

    // Toujours persister en local (cache / hors-ligne).
    let local = state
        .storage
        .get_task(&task.id)
        .map_err(|e| e.to_string())?;
    if task.reminder.is_none() {
        task.reminder = local.as_ref().and_then(|t| t.reminder.clone());
    }
    state.storage.upsert_task(&task).map_err(|e| e.to_string())?;

    if let Some(client) = try_client(app) {
        let api_result: Result<(), String> = async {
            match client.update_task(&task).await {
                Ok(()) => Ok(()),
                Err(_) => {
                    client.create_task(&task).await?;
                    if task.status != TaskStatus::Todo
                        || task.completed
                        || !task.notes.is_empty()
                        || !task.documents.is_empty()
                        || !task.activities.is_empty()
                    {
                        client.update_task(&task).await?;
                    }
                    Ok(())
                }
            }
        }
        .await;
        match api_result {
            Ok(()) => set_last_error(app, None),
            Err(err) => set_last_error(app, Some(format!("Écriture locale — API: {err}"))),
        }
    }

    Ok(task)
}

pub async fn delete_task_hybrid(
    app: &AppHandle,
    state: &AppState,
    id: &str,
) -> Result<(), String> {
    state.storage.delete_task(id).map_err(|e| e.to_string())?;
    if let Some(client) = try_client(app) {
        if let Err(err) = client.delete_task(id).await {
            set_last_error(app, Some(format!("Suppression locale — API: {err}")));
        } else {
            set_last_error(app, None);
        }
    }
    Ok(())
}

pub async fn create_task_hybrid(
    app: &AppHandle,
    state: &AppState,
    input: CreateTaskInput,
) -> Result<Task, String> {
    let now = now_iso();
    let project_id = input.project_id;
    let category = if let Some(cat) = input.category.filter(|c| !c.trim().is_empty()) {
        cat.trim().to_string()
    } else if let Some(ref pid) = project_id {
        state
            .storage
            .get_project(pid)
            .ok()
            .flatten()
            .map(|p| p.name)
            .unwrap_or_default()
    } else {
        String::new()
    };
    let prefs = load_prefs(app);
    let status = input.status.unwrap_or(TaskStatus::Todo);
    let task = Task {
        id: crate::domain::new_id(),
        title: input.title.trim().to_string(),
        description: input.description.unwrap_or_default(),
        category,
        status,
        completed: status.is_done(),
        priority: Some(input.priority.unwrap_or(TaskPriority::Medium)),
        due_date: input.due_date.filter(|d| !d.trim().is_empty()),
        assigned_to: prefs.user_id,
        created_by: prefs.user_id,
        notes: input.notes.unwrap_or_default(),
        documents: Vec::new(),
        activities: Vec::new(),
        project_id,
        reminder: None,
        created_at: now.clone(),
        updated_at: now,
    };
    upsert_task_hybrid(app, state, task).await
}

// --- Projets (work_projects Gestion ↔ SQLite local) ---

pub async fn list_projects_hybrid(
    app: &AppHandle,
    state: &AppState,
) -> Result<Vec<Project>, String> {
    let Some(client) = try_client(app) else {
        return state.storage.list_projects().map_err(|e| e.to_string());
    };

    use std::collections::HashSet;
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    let mut any_ok = false;
    let mut errors: Vec<String> = Vec::new();

    // Même source que le select « Projet » Gestion : Production + espaces de travail.
    match client.get_production_projects().await {
        Ok(rows) => {
            any_ok = true;
            for p in production_projects_to_mind(&rows) {
                if seen.insert(p.id.clone()) {
                    state.storage.upsert_project(&p).map_err(|e| e.to_string())?;
                    out.push(p);
                }
            }
        }
        Err(err) => errors.push(format!("projects.php — {err}")),
    }

    match client.get_work_projects().await {
        Ok(data) => {
            any_ok = true;
            for entry in &data.projects {
                if let Some(p) = work_project_to_mind(entry) {
                    if seen.insert(p.id.clone()) {
                        state
                            .storage
                            .upsert_project(&p)
                            .map_err(|e| e.to_string())?;
                        out.push(p);
                    }
                }
            }
        }
        Err(err) => errors.push(format!("work_projects.php — {err}")),
    }

    if any_ok {
        if errors.is_empty() {
            set_last_error(app, None);
        } else {
            set_last_error(app, Some(format!("Projets partiels — {}", errors.join(" · "))));
        }
        out.sort_by(|a, b| {
            a.name
                .to_lowercase()
                .cmp(&b.name.to_lowercase())
                .then_with(|| a.created_at.cmp(&b.created_at))
        });
        Ok(out)
    } else {
        let detail = if errors.is_empty() {
            "API injoignable".to_string()
        } else {
            errors.join(" · ")
        };
        set_last_error(app, Some(format!("Projets cache — {detail}")));
        state.storage.list_projects().map_err(|e| e.to_string())
    }
}

pub async fn upsert_project_hybrid(
    app: &AppHandle,
    state: &AppState,
    mut project: Project,
) -> Result<Project, String> {
    if project.id.is_empty() {
        project.id = crate::domain::new_id();
    }
    if project.created_at.is_empty() {
        project.created_at = now_iso();
    }
    state
        .storage
        .upsert_project(&project)
        .map_err(|e| e.to_string())?;

    if let Some(client) = try_client(app) {
        match client.get_work_projects().await {
            Ok(mut data) => {
                upsert_work_project_value(&mut data, &project);
                match client.save_work_projects(&data).await {
                    Ok(()) => set_last_error(app, None),
                    Err(err) => {
                        set_last_error(app, Some(format!("Projet local — API: {err}")))
                    }
                }
            }
            Err(err) => set_last_error(app, Some(format!("Projet local — API: {err}"))),
        }
    }
    Ok(project)
}

pub async fn delete_project_hybrid(
    app: &AppHandle,
    state: &AppState,
    id: &str,
) -> Result<(), String> {
    state.storage.delete_project(id).map_err(|e| e.to_string())?;
    if let Some(client) = try_client(app) {
        match client.get_work_projects().await {
            Ok(mut data) => {
                remove_work_project_value(&mut data, id);
                match client.save_work_projects(&data).await {
                    Ok(()) => set_last_error(app, None),
                    Err(err) => {
                        set_last_error(app, Some(format!("Suppression projet locale — API: {err}")))
                    }
                }
            }
            Err(err) => set_last_error(app, Some(format!("Suppression projet locale — API: {err}"))),
        }
    }
    Ok(())
}

// --- Notes / idées → bureau Gestion (workspace.php visibility=personal) ---

pub async fn list_notes_hybrid(
    app: &AppHandle,
    state: &AppState,
    filter: NoteFilter,
) -> Result<Vec<Note>, String> {
    let Some(client) = try_client(app) else {
        return state
            .storage
            .list_notes(&filter)
            .map_err(|e| e.to_string());
    };
    match client.list_workspace_personal().await {
        Ok(elements) => {
            use std::collections::HashSet;
            set_last_error(app, None);
            let mut remote_ids = HashSet::new();
            for el in elements {
                let Some(mut remote) = workspace_element_to_note(&el) else {
                    continue;
                };
                remote_ids.insert(remote.id.clone());
                if let Ok(Some(local)) = state.storage.get_note(&remote.id) {
                    if remote.project_id.is_none() {
                        remote.project_id = local.project_id;
                    }
                }
                state
                    .storage
                    .upsert_note(&remote)
                    .map_err(|e| e.to_string())?;
            }

            // Gestion → panneau : source de vérité = bureau distant.
            // Une note créée dans MIND (UUID) puis supprimée dans Gestion
            // ne doit PAS être re-poussée ni rester affichée.
            let locals = state
                .storage
                .list_notes(&NoteFilter::default())
                .map_err(|e| e.to_string())?;
            for local in locals {
                if !remote_ids.contains(&local.id) {
                    let _ = state.storage.delete_note(&local.id);
                }
            }

            state
                .storage
                .list_notes(&filter)
                .map_err(|e| e.to_string())
        }
        Err(err) => {
            set_last_error(app, Some(format!("Notes cache — {err}")));
            state
                .storage
                .list_notes(&filter)
                .map_err(|e| e.to_string())
        }
    }
}

pub async fn upsert_note_hybrid(
    app: &AppHandle,
    state: &AppState,
    mut note: Note,
) -> Result<Note, String> {
    let now = now_iso();
    if note.id.is_empty() {
        note.id = crate::domain::new_id();
    }
    if note.created_at.is_empty() {
        note.created_at = now.clone();
    }
    note.updated_at = now;
    note.body = note.body.trim().to_string();
    if let Some(ref mut t) = note.title {
        *t = t.trim().to_string();
        if t.is_empty() {
            note.title = None;
        }
    }

    state
        .storage
        .upsert_note(&note)
        .map_err(|e| e.to_string())?;

    if let Some(client) = try_client(app) {
        match client.save_workspace_element(&note).await {
            Ok(()) => set_last_error(app, None),
            Err(err) => set_last_error(app, Some(format!("Note locale — API: {err}"))),
        }
    }
    Ok(note)
}

pub async fn create_note_hybrid(
    app: &AppHandle,
    state: &AppState,
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
        id: crate::domain::new_id(),
        title,
        body,
        kind,
        project_id,
        created_at: now.clone(),
        updated_at: now,
    };
    upsert_note_hybrid(app, state, note).await
}

pub async fn delete_note_hybrid(
    app: &AppHandle,
    state: &AppState,
    id: &str,
) -> Result<(), String> {
    state.storage.delete_note(id).map_err(|e| e.to_string())?;
    if let Some(client) = try_client(app) {
        match client.delete_workspace_element(id).await {
            Ok(()) => set_last_error(app, None),
            Err(err) => set_last_error(app, Some(format!("Suppression note locale — API: {err}"))),
        }
    }
    Ok(())
}

pub async fn set_task_done_hybrid(
    app: &AppHandle,
    state: &AppState,
    id: &str,
    done: bool,
) -> Result<Task, String> {
    let mut task = get_task_hybrid(app, state, id)
        .await?
        .ok_or_else(|| format!("task not found: {id}"))?;
    task.status = if done {
        TaskStatus::Done
    } else {
        TaskStatus::Todo
    };
    task.completed = done;
    task.updated_at = now_iso();
    upsert_task_hybrid(app, state, task).await
}

pub async fn login(
    app: &AppHandle,
    username: String,
    password: String,
) -> Result<GestionPrefs, String> {
    let mut prefs = load_prefs(app);
    if prefs.api_url.trim().is_empty() {
        return Err("Configure d’abord l’URL de l’API Gestion".into());
    }
    let client = GestionClient::new(&prefs.api_url, "")?;
    let res = client.login(&username, &password).await?;
    prefs.auth_token = Some(res.token);
    prefs.user_id = Some(res.user.id);
    prefs.user_name = Some(
        format!("{} {}", res.user.prenom, res.user.nom)
            .trim()
            .to_string(),
    );
    if prefs.user_name.as_deref() == Some("") {
        prefs.user_name = Some(res.user.username);
    }
    save_prefs(app, &prefs)?;
    Ok(prefs)
}

/// Après login : tire projets + notes bureau Gestion dans le cache local.
pub async fn sync_projects_after_login(
    app: &AppHandle,
    state: &AppState,
) -> Result<usize, String> {
    let report = sync_bidirectional(app, state).await?;
    Ok(report.projects)
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncReport {
    pub tasks: usize,
    pub projects: usize,
    pub notes: usize,
    pub active: bool,
}

/// Sync bidirectionnel panneau ↔ Gestion (pull API + push locaux orphelins).
pub async fn sync_bidirectional(
    app: &AppHandle,
    state: &AppState,
) -> Result<SyncReport, String> {
    if try_client(app).is_none() {
        return Ok(SyncReport {
            tasks: 0,
            projects: 0,
            notes: 0,
            active: false,
        });
    }
    let projects = list_projects_hybrid(app, state).await?;
    let tasks = list_tasks_hybrid(app, state, TaskFilter::default()).await?;
    let notes = list_notes_hybrid(app, state, NoteFilter::default()).await?;
    Ok(SyncReport {
        tasks: tasks.len(),
        projects: projects.len(),
        notes: notes.len(),
        active: true,
    })
}

/// Lance un sync en arrière-plan et notifie l’UI (`data-changed`).
/// Debounce ~8s pour éviter de spammer l’API.
pub fn schedule_sync(app: &AppHandle) {
    if try_client(app).is_none() {
        return;
    }
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};
    static LAST_MS: AtomicU64 = AtomicU64::new(0);
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let prev = LAST_MS.load(Ordering::Relaxed);
    if now.saturating_sub(prev) < 8_000 {
        return;
    }
    LAST_MS.store(now, Ordering::Relaxed);

    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let state = app.state::<AppState>();
        match sync_bidirectional(&app, &state).await {
            Ok(report) if report.active => {
                use tauri::Emitter;
                let _ = app.emit(
                    "data-changed",
                    crate::domain::DataChangedPayload {
                        entity: "sync".into(),
                        id: format!(
                            "gestion:{}:{}:{}",
                            report.tasks, report.projects, report.notes
                        ),
                    },
                );
                let _ = app.emit(
                    "data-changed",
                    crate::domain::DataChangedPayload {
                        entity: "task".into(),
                        id: "pull".into(),
                    },
                );
                let _ = app.emit(
                    "data-changed",
                    crate::domain::DataChangedPayload {
                        entity: "note".into(),
                        id: "pull".into(),
                    },
                );
                let _ = app.emit(
                    "data-changed",
                    crate::domain::DataChangedPayload {
                        entity: "project".into(),
                        id: "pull".into(),
                    },
                );
            }
            _ => {}
        }
    });
}

pub fn logout(app: &AppHandle) -> Result<GestionPrefs, String> {
    let mut prefs = load_prefs(app);
    prefs.auth_token = None;
    prefs.user_id = None;
    prefs.user_name = None;
    save_prefs(app, &prefs)?;
    Ok(prefs)
}

pub fn set_session(
    app: &AppHandle,
    token: String,
    user_id: Option<i64>,
    user_name: Option<String>,
) -> Result<GestionPrefs, String> {
    let mut prefs = load_prefs(app);
    let token = token.trim().to_string();
    if token.is_empty() {
        prefs.auth_token = None;
        prefs.user_id = None;
        prefs.user_name = None;
    } else {
        prefs.auth_token = Some(token);
        if user_id.is_some() {
            prefs.user_id = user_id;
        }
        if user_name.is_some() {
            prefs.user_name = user_name;
        }
    }
    save_prefs(app, &prefs)?;
    Ok(prefs)
}

/// Import one-shot des tâches SQLite locales vers `personal_tasks`.
pub async fn migrate_local_tasks(app: &AppHandle, state: &AppState) -> Result<MigrateReport, String> {
    let client = try_client(app).ok_or_else(|| {
        "Connexion Gestion requise (URL API + session) pour migrer les tâches".to_string()
    })?;

    let mut prefs = load_prefs(app);
    let local = state
        .storage
        .list_tasks(&TaskFilter::default())
        .map_err(|e| e.to_string())?;
    let remote = client.list_tasks(None).await?;
    let remote_ids: std::collections::HashSet<String> =
        remote.into_iter().map(|t| t.id).collect();

    let mut created = 0u32;
    let mut skipped = 0u32;
    let mut errors = Vec::new();

    for task in local {
        if remote_ids.contains(&task.id) {
            skipped += 1;
            continue;
        }
        // Ne pas re-pousser une tâche déjà issue de gestion (préfixe pt_).
        // On pousse tout ce qui n'est pas encore distant.
        match client.create_task(&task).await {
            Ok(()) => {
                if task.status != TaskStatus::Todo
                    || task.completed
                    || !task.notes.is_empty()
                    || !task.description.is_empty()
                {
                    if let Err(e) = client.update_task(&task).await {
                        errors.push(format!("{}: {}", task.id, e));
                        continue;
                    }
                }
                created += 1;
            }
            Err(e) => errors.push(format!("{}: {}", task.id, e)),
        }
    }

    prefs.tasks_migrated_at = Some(now_iso());
    save_prefs(app, &prefs)?;

    Ok(MigrateReport {
        created,
        skipped,
        errors,
        migrated_at: prefs.tasks_migrated_at.clone(),
    })
}
