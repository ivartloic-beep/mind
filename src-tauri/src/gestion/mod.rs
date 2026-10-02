//! Client API gestion-v2 (`personal_tasks` + login) — source de vérité tâches.

mod client;

pub use client::{GestionClient, MigrateReport};

use tauri::AppHandle;

use crate::domain::{now_iso, CreateTaskInput, Project, Task, TaskFilter, TaskPriority, TaskStatus};
use crate::state::AppState;
use crate::storage::Storage;
use crate::windows::gestion::{load_prefs, save_prefs, GestionPrefs};

use self::client::{
    remove_work_project_value, upsert_work_project_value, work_project_to_mind,
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
            set_last_error(app, None);
            let projects = list_projects_hybrid(app, state).await.unwrap_or_default();
            let mut merged = Vec::with_capacity(remote.len());
            for r in remote {
                let local = state.storage.get_task(&r.id).map_err(|e| e.to_string())?;
                let task = merge_remote_with_local(r, local.as_ref(), &projects);
                state.storage.upsert_task(&task).map_err(|e| e.to_string())?;
                merged.push(task);
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
    match client.get_work_projects().await {
        Ok(data) => {
            set_last_error(app, None);
            let mut out = Vec::new();
            for entry in &data.projects {
                if let Some(p) = work_project_to_mind(entry) {
                    state.storage.upsert_project(&p).map_err(|e| e.to_string())?;
                    out.push(p);
                }
            }
            out.sort_by(|a, b| a.created_at.cmp(&b.created_at));
            Ok(out)
        }
        Err(err) => {
            set_last_error(app, Some(format!("Projets cache — {err}")));
            state.storage.list_projects().map_err(|e| e.to_string())
        }
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

/// Après login : tire les projets Gestion dans le cache local.
pub async fn sync_projects_after_login(
    app: &AppHandle,
    state: &AppState,
) -> Result<usize, String> {
    let rows = list_projects_hybrid(app, state).await?;
    Ok(rows.len())
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
