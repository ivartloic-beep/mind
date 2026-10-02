//! HTTP client pour login.php + personal_tasks.php + projects.php + work_projects.php.

use std::collections::HashSet;

use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::domain::{now_iso, Note, NoteKind, Project, Task, TaskPriority, TaskStatus};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GestionAuthUser {
    pub id: i64,
    pub username: String,
    #[serde(default)]
    pub nom: String,
    #[serde(default)]
    pub prenom: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GestionLoginResponse {
    pub token: String,
    pub user: GestionAuthUser,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrateReport {
    pub created: u32,
    pub skipped: u32,
    pub errors: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub migrated_at: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ApiTasksResponse {
    success: Option<bool>,
    tasks: Option<Vec<GestionTaskPayload>>,
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ApiOkResponse {
    success: Option<bool>,
    id: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiWorkProjectsResponse {
    success: Option<bool>,
    work_projects: Option<WorkProjectsData>,
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ApiProjectsResponse {
    success: Option<bool>,
    projects: Option<Vec<Value>>,
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ApiWorkspaceResponse {
    success: Option<bool>,
    elements: Option<Vec<Value>>,
    error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct WorkProjectsData {
    /// Entrées brutes (on préserve les champs Gestion au round-trip).
    #[serde(default)]
    pub projects: Vec<Value>,
    #[serde(default)]
    pub templates: Vec<Value>,
}

fn json_id(value: &Value) -> Option<String> {
    value.get("id").and_then(|v| {
        v.as_str()
            .map(|s| s.to_string())
            .or_else(|| v.as_i64().map(|n| n.to_string()))
            .or_else(|| v.as_u64().map(|n| n.to_string()))
            .or_else(|| v.as_f64().map(|n| n.to_string()))
    })
    .filter(|s| !s.is_empty())
}

fn json_str_any(value: &Value, keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Some(s) = value
            .get(*key)
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            return Some(s.to_string());
        }
    }
    None
}

/// Espace de travail Gestion (`work_projects`) — champ principal = `title`.
pub fn work_project_to_mind(value: &Value) -> Option<Project> {
    if value.get("archived").and_then(|v| v.as_bool()) == Some(true) {
        return None;
    }
    let id = json_id(value)?;
    let name = json_str_any(value, &["title", "name", "nom"])?;
    let color = json_str_any(value, &["color"]);
    let created_at = json_str_any(value, &["createdAt", "created_at"]).unwrap_or_else(now_iso);
    Some(Project {
        id,
        name,
        color,
        created_at,
    })
}

/// Projets Production Gestion (`projects.php`) — `name` / `nom` / spectacles imbriqués.
pub fn production_projects_to_mind(projects: &[Value]) -> Vec<Project> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();

    let push = |value: &Value, out: &mut Vec<Project>, seen: &mut HashSet<String>| {
        let id = match json_id(value) {
            Some(id) => id,
            None => return,
        };
        if !seen.insert(id.clone()) {
            return;
        }
        let name = match json_str_any(value, &["name", "nom", "title", "lieu"]) {
            Some(n) => n,
            None => return,
        };
        let color = json_str_any(value, &["color"]);
        let created_at =
            json_str_any(value, &["createdAt", "created_at"]).unwrap_or_else(now_iso);
        out.push(Project {
            id,
            name,
            color,
            created_at,
        });
    };

    for p in projects {
        push(p, &mut out, &mut seen);
        if let Some(specs) = p.get("spectacles").and_then(|s| s.as_array()) {
            for s in specs {
                push(s, &mut out, &mut seen);
            }
        }
    }
    out
}

pub fn upsert_work_project_value(data: &mut WorkProjectsData, project: &Project) {
    let mut found = false;
    for entry in &mut data.projects {
        if json_id(entry).as_deref() == Some(project.id.as_str()) {
            if let Some(obj) = entry.as_object_mut() {
                // Gestion lit `title` (pas `name`).
                obj.insert("title".into(), Value::String(project.name.clone()));
                obj.insert("name".into(), Value::String(project.name.clone()));
                if let Some(color) = &project.color {
                    obj.insert("color".into(), Value::String(color.clone()));
                }
                obj.insert("updatedAt".into(), Value::String(now_iso()));
            }
            found = true;
            break;
        }
    }
    if !found {
        data.projects.push(serde_json::json!({
            "id": project.id,
            "title": project.name,
            "name": project.name,
            "color": project.color.clone().unwrap_or_else(|| "#4a90d9".into()),
            "icon": "📁",
            "description": "",
            "tasks": [],
            "members": [],
            "team": [],
            "archived": false,
            "createdAt": project.created_at,
            "updatedAt": now_iso(),
        }));
    }
}

pub fn remove_work_project_value(data: &mut WorkProjectsData, id: &str) {
    data.projects
        .retain(|e| json_id(e).as_deref() != Some(id));
}

fn plain_to_html(text: &str) -> String {
    let escaped = text
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;");
    format!("<p>{}</p>", escaped.replace('\n', "<br>"))
}

fn html_to_plain(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut in_tag = false;
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out.replace("&nbsp;", " ")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
        .trim()
        .to_string()
}

fn parse_workspace_content(raw: &str, typ: &str) -> String {
    if raw.trim().is_empty() {
        return String::new();
    }
    if let Ok(v) = serde_json::from_str::<Value>(raw) {
        if typ == "quicknote" {
            if let Some(text) = v.get("text").and_then(|t| t.as_str()) {
                return text.to_string();
            }
        }
        if let Some(html) = v.get("html").and_then(|h| h.as_str()) {
            return html_to_plain(html);
        }
        if let Some(text) = v.get("text").and_then(|t| t.as_str()) {
            return text.to_string();
        }
    }
    html_to_plain(raw)
}

pub fn note_title_for_workspace(note: &Note) -> String {
    if let Some(t) = note
        .title
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
    {
        return t.to_string();
    }
    let first = note.body.lines().next().unwrap_or("").trim();
    if first.is_empty() {
        "Sans titre".into()
    } else {
        let count = first.chars().count();
        if count > 80 {
            format!("{}…", first.chars().take(77).collect::<String>())
        } else {
            first.to_string()
        }
    }
}

/// Élément workspace → Note MIND (`page`/`quicknote`/`idea`).
/// `fallback_project_id` : projet du listing team si l’élément n’en porte pas.
pub fn workspace_element_to_note(
    value: &Value,
    fallback_project_id: Option<&str>,
) -> Option<Note> {
    let typ = value.get("type").and_then(|t| t.as_str())?;
    let kind = match typ {
        "page" | "quicknote" => NoteKind::Note,
        "idea" => NoteKind::Idea,
        _ => return None,
    };
    let id = json_id(value)?;
    let title = json_str_any(value, &["title"]).filter(|t| t != "Sans titre");
    let content_raw = value
        .get("content")
        .and_then(|c| c.as_str())
        .unwrap_or("");
    let body = parse_workspace_content(content_raw, typ);
    let created_at =
        json_str_any(value, &["created_at", "createdAt"]).unwrap_or_else(now_iso);
    let updated_at = json_str_any(value, &["updated_at", "updatedAt"])
        .unwrap_or_else(|| created_at.clone());
    let project_id = value
        .get("project_id")
        .and_then(|v| {
            v.as_str()
                .map(|s| s.to_string())
                .or_else(|| v.as_i64().map(|n| n.to_string()))
                .or_else(|| v.as_u64().map(|n| n.to_string()))
        })
        .filter(|s| !s.is_empty())
        .or_else(|| fallback_project_id.map(|s| s.to_string()));
    Some(Note {
        id,
        title,
        body,
        kind,
        project_id,
        created_at,
        updated_at,
    })
}

pub fn note_to_workspace_payload(note: &Note, token: &str) -> Value {
    let typ = match note.kind {
        NoteKind::Note => "page",
        NoteKind::Idea => "idea",
    };
    let content = serde_json::json!({
        "html": plain_to_html(&note.body),
        "attachments": [],
    });
    let has_project = note
        .project_id
        .as_ref()
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false);
    // Projet assigné → notes du projet (team) ; sinon bureau (personal).
    let visibility = if has_project { "team" } else { "personal" };
    let mut payload = serde_json::json!({
        "id": note.id,
        "type": typ,
        "title": note_title_for_workspace(note),
        "content": content.to_string(),
        "visibility": visibility,
        "tags": "[]",
        "folder_id": null,
        "token": token,
    });
    if let Some(obj) = payload.as_object_mut() {
        if has_project {
            obj.insert(
                "project_id".into(),
                Value::String(note.project_id.clone().unwrap_or_default()),
            );
        }
        if matches!(note.kind, NoteKind::Idea) {
            obj.insert("status".into(), Value::String("draft".into()));
        }
    }
    payload
}

/// Payload brut personal_tasks (API PHP).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GestionTaskPayload {
    pub id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub priority: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub completed: bool,
    #[serde(default)]
    pub due_date: Option<String>,
    #[serde(default)]
    pub assigned_to: Option<i64>,
    #[serde(default)]
    pub created_by: Option<i64>,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub documents: Vec<Value>,
    #[serde(default)]
    pub activities: Vec<Value>,
    #[serde(default)]
    pub created_at: Option<String>,
}

impl GestionTaskPayload {
    pub fn into_task(self) -> Task {
        let status = TaskStatus::parse(&self.status).unwrap_or_else(|_| {
            if self.completed {
                TaskStatus::Done
            } else {
                TaskStatus::Todo
            }
        });
        let priority = if self.priority.is_empty() {
            Some(TaskPriority::Medium)
        } else {
            TaskPriority::parse(&self.priority).ok()
        };
        let created = self
            .created_at
            .clone()
            .unwrap_or_else(crate::domain::now_iso);
        let mut task = Task {
            id: self.id,
            title: self.title,
            description: self.description,
            category: self.category,
            status,
            completed: self.completed || status.is_done(),
            priority,
            due_date: self.due_date,
            assigned_to: self.assigned_to,
            created_by: self.created_by,
            notes: self.notes,
            documents: self.documents,
            activities: self.activities,
            project_id: None,
            reminder: None,
            created_at: created.clone(),
            updated_at: created,
        };
        task.normalize();
        task
    }

    pub fn from_task(task: &Task) -> Self {
        Self {
            id: task.id.clone(),
            title: task.title.clone(),
            description: task.description.clone(),
            category: task.category.clone(),
            priority: task
                .priority
                .unwrap_or(TaskPriority::Medium)
                .as_str()
                .to_string(),
            status: task.status.as_str().to_string(),
            completed: task.completed || task.status.is_done(),
            due_date: task.due_date.clone(),
            assigned_to: task.assigned_to,
            created_by: task.created_by,
            notes: task.notes.clone(),
            documents: task.documents.clone(),
            activities: task.activities.clone(),
            created_at: Some(task.created_at.clone()),
        }
    }
}

#[derive(Clone)]
pub struct GestionClient {
    http: reqwest::Client,
    base_url: String,
    token: String,
}

impl GestionClient {
    pub fn new(api_url: &str, token: &str) -> Result<Self, String> {
        let base = api_url.trim().trim_end_matches('/').to_string();
        if base.is_empty() {
            return Err("URL API Gestion manquante".into());
        }
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| e.to_string())?;
        Ok(Self {
            http,
            base_url: base,
            token: token.trim().to_string(),
        })
    }

    fn url(&self, endpoint: &str) -> String {
        format!("{}/{}", self.base_url, endpoint.trim_start_matches('/'))
    }

    pub async fn login(
        &self,
        username: &str,
        password: &str,
    ) -> Result<GestionLoginResponse, String> {
        let url = self.url("login.php");
        let res = self
            .http
            .post(&url)
            .header(CONTENT_TYPE, "application/json")
            .json(&serde_json::json!({
                "username": username,
                "password": password,
            }))
            .send()
            .await
            .map_err(|e| e.to_string())?;
        let status = res.status();
        let body = res.text().await.map_err(|e| e.to_string())?;
        if !status.is_success() {
            let err = serde_json::from_str::<Value>(&body)
                .ok()
                .and_then(|v| v.get("error").and_then(|e| e.as_str()).map(|s| s.to_string()))
                .unwrap_or_else(|| format!("login HTTP {status}"));
            return Err(err);
        }
        let parsed: Value = serde_json::from_str(&body).map_err(|e| e.to_string())?;
        if parsed.get("success").and_then(|v| v.as_bool()) == Some(false) {
            return Err(parsed
                .get("error")
                .and_then(|e| e.as_str())
                .unwrap_or("Identifiants incorrects")
                .to_string());
        }
        let token = parsed
            .get("token")
            .and_then(|t| t.as_str())
            .ok_or_else(|| "Réponse login sans token".to_string())?
            .to_string();
        let user_val = parsed
            .get("user")
            .cloned()
            .ok_or_else(|| "Réponse login sans user".to_string())?;
        let user: GestionAuthUser =
            serde_json::from_value(user_val).map_err(|e| e.to_string())?;
        Ok(GestionLoginResponse { token, user })
    }

    fn auth_headers(&self) -> Result<(String, String), String> {
        if self.token.is_empty() {
            return Err("Session Gestion absente — connecte-toi".into());
        }
        Ok((
            format!("Bearer {}", self.token),
            self.token.clone(),
        ))
    }

    pub async fn list_tasks(&self, scope: Option<&str>) -> Result<Vec<Task>, String> {
        let (bearer, raw) = self.auth_headers()?;
        let mut endpoint = "personal_tasks.php".to_string();
        if let Some(s) = scope {
            endpoint.push_str(&format!("?scope={s}&token={}", urlencoding_lite(&raw)));
        } else {
            endpoint.push_str(&format!("?token={}", urlencoding_lite(&raw)));
        }
        let url = self.url(&endpoint);
        let res = self
            .http
            .get(&url)
            .header(AUTHORIZATION, bearer)
            .header("X-Auth-Token", &raw)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        let status = res.status();
        let body = res.text().await.map_err(|e| e.to_string())?;
        if status.as_u16() == 401 {
            return Err("Session Gestion expirée".into());
        }
        if !status.is_success() {
            return Err(format!("personal_tasks GET HTTP {status}: {body}"));
        }
        let parsed: ApiTasksResponse = serde_json::from_str(&body).map_err(|e| {
            format!("personal_tasks JSON: {e} — {}", body.chars().take(200).collect::<String>())
        })?;
        if let Some(err) = parsed.error {
            return Err(err);
        }
        Ok(parsed
            .tasks
            .unwrap_or_default()
            .into_iter()
            .map(GestionTaskPayload::into_task)
            .collect())
    }

    pub async fn create_task(&self, task: &Task) -> Result<(), String> {
        let (bearer, raw) = self.auth_headers()?;
        let url = self.url("personal_tasks.php");
        let mut payload = GestionTaskPayload::from_task(task);
        // POST force status todo côté PHP — on envoie l'essentiel.
        let body = serde_json::json!({
            "id": payload.id,
            "title": payload.title,
            "description": payload.description,
            "category": payload.category,
            "priority": payload.priority,
            "dueDate": payload.due_date,
            "assignedTo": payload.assigned_to,
            "token": raw,
        });
        let _ = &mut payload;
        let res = self
            .http
            .post(&url)
            .header(AUTHORIZATION, bearer)
            .header("X-Auth-Token", &raw)
            .header(CONTENT_TYPE, "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        let status = res.status();
        let text = res.text().await.map_err(|e| e.to_string())?;
        if status.as_u16() == 401 {
            return Err("Session Gestion expirée".into());
        }
        if !status.is_success() {
            let err = serde_json::from_str::<ApiOkResponse>(&text)
                .ok()
                .and_then(|r| r.error)
                .unwrap_or(text);
            return Err(format!("personal_tasks POST: {err}"));
        }
        Ok(())
    }

    pub async fn update_task(&self, task: &Task) -> Result<(), String> {
        let (bearer, raw) = self.auth_headers()?;
        let url = self.url("personal_tasks.php");
        let payload = GestionTaskPayload::from_task(task);
        let body = serde_json::json!({
            "id": payload.id,
            "title": payload.title,
            "description": payload.description,
            "category": payload.category,
            "priority": payload.priority,
            "status": payload.status,
            "completed": payload.completed,
            "dueDate": payload.due_date,
            "assignedTo": payload.assigned_to,
            "notes": payload.notes,
            "documents": payload.documents,
            "activities": payload.activities,
            "token": raw,
        });
        let res = self
            .http
            .put(&url)
            .header(AUTHORIZATION, bearer)
            .header("X-Auth-Token", &raw)
            .header(CONTENT_TYPE, "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        let status = res.status();
        let text = res.text().await.map_err(|e| e.to_string())?;
        if status.as_u16() == 401 {
            return Err("Session Gestion expirée".into());
        }
        if !status.is_success() {
            let err = serde_json::from_str::<ApiOkResponse>(&text)
                .ok()
                .and_then(|r| r.error)
                .unwrap_or(text);
            return Err(format!("personal_tasks PUT: {err}"));
        }
        Ok(())
    }

    pub async fn delete_task(&self, id: &str) -> Result<(), String> {
        let (bearer, raw) = self.auth_headers()?;
        let url = self.url("personal_tasks.php");
        let body = serde_json::json!({ "id": id, "token": raw });
        let res = self
            .http
            .delete(&url)
            .header(AUTHORIZATION, bearer)
            .header("X-Auth-Token", &raw)
            .header(CONTENT_TYPE, "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        let status = res.status();
        if status.as_u16() == 401 {
            return Err("Session Gestion expirée".into());
        }
        if !status.is_success() {
            let text = res.text().await.unwrap_or_default();
            return Err(format!("personal_tasks DELETE HTTP {status}: {text}"));
        }
        Ok(())
    }

    /// Projets Production (tournées / spectacles) — `projects.php`.
    pub async fn get_production_projects(&self) -> Result<Vec<Value>, String> {
        let (bearer, raw) = self.auth_headers()?;
        let endpoint = format!("projects.php?token={}", urlencoding_lite(&raw));
        let url = self.url(&endpoint);
        let res = self
            .http
            .get(&url)
            .header(AUTHORIZATION, bearer)
            .header("X-Auth-Token", &raw)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        let status = res.status();
        let body = res.text().await.map_err(|e| e.to_string())?;
        if status.as_u16() == 401 {
            return Err("Session Gestion expirée".into());
        }
        if !status.is_success() {
            return Err(format!("projects GET HTTP {status}: {body}"));
        }
        let parsed: ApiProjectsResponse = serde_json::from_str(&body).map_err(|e| {
            format!(
                "projects JSON: {e} — {}",
                body.chars().take(200).collect::<String>()
            )
        })?;
        if let Some(err) = parsed.error {
            return Err(err);
        }
        Ok(parsed.projects.unwrap_or_default())
    }

    pub async fn get_work_projects(&self) -> Result<WorkProjectsData, String> {
        let (bearer, raw) = self.auth_headers()?;
        let endpoint = format!("work_projects.php?token={}", urlencoding_lite(&raw));
        let url = self.url(&endpoint);
        let res = self
            .http
            .get(&url)
            .header(AUTHORIZATION, bearer)
            .header("X-Auth-Token", &raw)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        let status = res.status();
        let body = res.text().await.map_err(|e| e.to_string())?;
        if status.as_u16() == 401 {
            return Err("Session Gestion expirée".into());
        }
        if !status.is_success() {
            return Err(format!("work_projects GET HTTP {status}: {body}"));
        }
        let parsed: ApiWorkProjectsResponse = serde_json::from_str(&body).map_err(|e| {
            format!(
                "work_projects JSON: {e} — {}",
                body.chars().take(200).collect::<String>()
            )
        })?;
        if let Some(err) = parsed.error {
            return Err(err);
        }
        Ok(parsed.work_projects.unwrap_or_default())
    }

    pub async fn save_work_projects(&self, data: &WorkProjectsData) -> Result<(), String> {
        let (bearer, raw) = self.auth_headers()?;
        let url = self.url("work_projects.php");
        let body = serde_json::json!({
            "workProjects": data,
            "token": raw,
        });
        let res = self
            .http
            .post(&url)
            .header(AUTHORIZATION, bearer)
            .header("X-Auth-Token", &raw)
            .header(CONTENT_TYPE, "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        let status = res.status();
        let text = res.text().await.map_err(|e| e.to_string())?;
        if status.as_u16() == 401 {
            return Err("Session Gestion expirée".into());
        }
        if !status.is_success() {
            let err = serde_json::from_str::<ApiOkResponse>(&text)
                .ok()
                .and_then(|r| r.error)
                .unwrap_or(text);
            return Err(format!("work_projects POST: {err}"));
        }
        Ok(())
    }

    /// Notes / idées du bureau (`workspace.php?visibility=personal`).
    pub async fn list_workspace_personal(&self) -> Result<Vec<Value>, String> {
        self.list_workspace("personal", None).await
    }

    /// Notes d’un projet Gestion (`visibility=team&project_id=…`).
    pub async fn list_workspace_for_project(
        &self,
        project_id: &str,
    ) -> Result<Vec<Value>, String> {
        self.list_workspace("team", Some(project_id)).await
    }

    async fn list_workspace(
        &self,
        visibility: &str,
        project_id: Option<&str>,
    ) -> Result<Vec<Value>, String> {
        let (bearer, raw) = self.auth_headers()?;
        let mut endpoint = format!(
            "workspace.php?visibility={}&token={}",
            urlencoding_lite(visibility),
            urlencoding_lite(&raw)
        );
        if let Some(pid) = project_id {
            endpoint.push_str(&format!("&project_id={}", urlencoding_lite(pid)));
        }
        let url = self.url(&endpoint);
        let res = self
            .http
            .get(&url)
            .header(AUTHORIZATION, bearer)
            .header("X-Auth-Token", &raw)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        let status = res.status();
        let body = res.text().await.map_err(|e| e.to_string())?;
        if status.as_u16() == 401 {
            return Err("Session Gestion expirée".into());
        }
        if !status.is_success() {
            return Err(format!("workspace GET HTTP {status}: {body}"));
        }
        let parsed: ApiWorkspaceResponse = serde_json::from_str(&body).map_err(|e| {
            format!(
                "workspace JSON: {e} — {}",
                body.chars().take(200).collect::<String>()
            )
        })?;
        if let Some(err) = parsed.error {
            return Err(err);
        }
        Ok(parsed.elements.unwrap_or_default())
    }

    pub async fn save_workspace_element(&self, note: &Note) -> Result<(), String> {
        let (bearer, raw) = self.auth_headers()?;
        let url = self.url("workspace.php");
        let payload = note_to_workspace_payload(note, &raw);

        // PUT d’abord (update), sinon POST (création) — même schéma que personal_tasks.
        let put_res = self
            .http
            .put(&url)
            .header(AUTHORIZATION, &bearer)
            .header("X-Auth-Token", &raw)
            .header(CONTENT_TYPE, "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        let put_status = put_res.status();
        let put_text = put_res.text().await.map_err(|e| e.to_string())?;
        if put_status.as_u16() == 401 {
            return Err("Session Gestion expirée".into());
        }
        if put_status.is_success() {
            return Ok(());
        }

        let post_res = self
            .http
            .post(&url)
            .header(AUTHORIZATION, bearer)
            .header("X-Auth-Token", &raw)
            .header(CONTENT_TYPE, "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        let post_status = post_res.status();
        let post_text = post_res.text().await.map_err(|e| e.to_string())?;
        if post_status.as_u16() == 401 {
            return Err("Session Gestion expirée".into());
        }
        if !post_status.is_success() {
            let err = serde_json::from_str::<ApiOkResponse>(&post_text)
                .ok()
                .and_then(|r| r.error)
                .or_else(|| {
                    serde_json::from_str::<ApiOkResponse>(&put_text)
                        .ok()
                        .and_then(|r| r.error)
                })
                .unwrap_or_else(|| format!("PUT {put_status}: {put_text} / POST {post_status}: {post_text}"));
            return Err(format!("workspace save: {err}"));
        }
        Ok(())
    }

    pub async fn delete_workspace_element(&self, id: &str) -> Result<(), String> {
        let (bearer, raw) = self.auth_headers()?;
        let url = self.url("workspace.php");
        let body = serde_json::json!({ "id": id, "token": raw });
        let res = self
            .http
            .delete(&url)
            .header(AUTHORIZATION, bearer)
            .header("X-Auth-Token", &raw)
            .header(CONTENT_TYPE, "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        let status = res.status();
        let text = res.text().await.map_err(|e| e.to_string())?;
        if status.as_u16() == 401 {
            return Err("Session Gestion expirée".into());
        }
        if !status.is_success() {
            let err = serde_json::from_str::<ApiOkResponse>(&text)
                .ok()
                .and_then(|r| r.error)
                .unwrap_or(text);
            return Err(format!("workspace DELETE: {err}"));
        }
        Ok(())
    }

    /// Télécharge un fichier workspace (`workspace_file.php`) avec la session MIND.
    pub async fn fetch_workspace_file(
        &self,
        id: &str,
    ) -> Result<(Vec<u8>, String, Option<String>), String> {
        let id = id.trim();
        if id.is_empty() {
            return Err("id fichier manquant".into());
        }
        let (bearer, raw) = self.auth_headers()?;
        let url = format!(
            "{}?id={}&token={}&download=1",
            self.url("workspace_file.php"),
            urlencoding_lite(id),
            urlencoding_lite(&raw)
        );
        let res = self
            .http
            .get(&url)
            .header(AUTHORIZATION, bearer)
            .header("X-Auth-Token", &raw)
            .timeout(std::time::Duration::from_secs(120))
            .send()
            .await
            .map_err(|e| format!("réseau fichier: {e}"))?;
        let status = res.status();
        if status.as_u16() == 401 {
            return Err("Session Gestion expirée — reconnecte-toi".into());
        }
        if !status.is_success() {
            let text = res.text().await.unwrap_or_default();
            let snippet: String = text.chars().take(160).collect();
            return Err(format!(
                "fichier HTTP {status}{}",
                if snippet.is_empty() {
                    String::new()
                } else {
                    format!(": {snippet}")
                }
            ));
        }
        let mime = res
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.split(';').next().unwrap_or(s).trim().to_string())
            .filter(|s| !s.is_empty());
        let filename = res
            .headers()
            .get(reqwest::header::CONTENT_DISPOSITION)
            .and_then(|v| v.to_str().ok())
            .and_then(filename_from_content_disposition)
            .unwrap_or_else(|| format!("gestion-{id}"));
        let bytes = res
            .bytes()
            .await
            .map_err(|e| format!("lecture fichier: {e}"))?
            .to_vec();
        if bytes.is_empty() {
            return Err("Fichier vide".into());
        }
        // Évite d’ouvrir une page d’erreur HTML comme « document ».
        if let Some(m) = mime.as_deref() {
            if m.starts_with("text/html") {
                let snippet: String = String::from_utf8_lossy(&bytes).chars().take(120).collect();
                return Err(format!("réponse HTML au lieu du fichier: {snippet}"));
            }
        }
        Ok((bytes, filename, mime))
    }

    /// Télécharge un fichier upload générique (`download.php`) — PJ fiche tâche, etc.
    pub async fn fetch_uploaded_file(
        &self,
        id: &str,
        preferred_name: Option<&str>,
    ) -> Result<(Vec<u8>, String, Option<String>), String> {
        let id = id.trim();
        if id.is_empty() {
            return Err("id fichier manquant".into());
        }
        let (bearer, raw) = self.auth_headers()?;
        let url = self.url("download.php");
        let form = reqwest::multipart::Form::new()
            .text("id", id.to_string())
            .text("token", raw.clone());
        let res = self
            .http
            .post(&url)
            .header(AUTHORIZATION, &bearer)
            .header("X-Auth-Token", &raw)
            .multipart(form)
            .timeout(std::time::Duration::from_secs(120))
            .send()
            .await
            .map_err(|e| format!("réseau download: {e}"))?;
        let status = res.status();
        if status.as_u16() == 401 {
            return Err("Session Gestion expirée — reconnecte-toi".into());
        }
        if !status.is_success() {
            // Fallback GET ?id=&auth=
            let get_url = format!(
                "{}?id={}&auth={}",
                self.url("download.php"),
                urlencoding_lite(id),
                urlencoding_lite(&raw)
            );
            let res2 = self
                .http
                .get(&get_url)
                .header(AUTHORIZATION, bearer)
                .header("X-Auth-Token", &raw)
                .timeout(std::time::Duration::from_secs(120))
                .send()
                .await
                .map_err(|e| format!("réseau download GET: {e}"))?;
            let status2 = res2.status();
            if !status2.is_success() {
                let text = res2.text().await.unwrap_or_default();
                let snippet: String = text.chars().take(160).collect();
                return Err(format!(
                    "download HTTP {status2}{}",
                    if snippet.is_empty() {
                        String::new()
                    } else {
                        format!(": {snippet}")
                    }
                ));
            }
            return Self::read_file_response(res2, id, preferred_name).await;
        }
        Self::read_file_response(res, id, preferred_name).await
    }

    async fn read_file_response(
        res: reqwest::Response,
        id: &str,
        preferred_name: Option<&str>,
    ) -> Result<(Vec<u8>, String, Option<String>), String> {
        let mime = res
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.split(';').next().unwrap_or(s).trim().to_string())
            .filter(|s| !s.is_empty());
        let filename = preferred_name
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .or_else(|| {
                res.headers()
                    .get(reqwest::header::CONTENT_DISPOSITION)
                    .and_then(|v| v.to_str().ok())
                    .and_then(filename_from_content_disposition)
            })
            .unwrap_or_else(|| format!("upload-{id}"));
        let bytes = res
            .bytes()
            .await
            .map_err(|e| format!("lecture fichier: {e}"))?
            .to_vec();
        if bytes.is_empty() {
            return Err("Fichier vide".into());
        }
        if let Some(m) = mime.as_deref() {
            if m.starts_with("text/html") || m.contains("json") {
                let snippet: String = String::from_utf8_lossy(&bytes).chars().take(120).collect();
                return Err(format!("réponse non-fichier: {snippet}"));
            }
        }
        Ok((bytes, filename, mime))
    }

    /// Upload fichier → `workspace_upload.php` (bureau personal ou projet team).
    pub async fn upload_workspace_file(
        &self,
        filename: &str,
        mime: Option<&str>,
        bytes: Vec<u8>,
        visibility: &str,
        project_id: Option<&str>,
    ) -> Result<Value, String> {
        let (bearer, raw) = self.auth_headers()?;
        let url = self.url("workspace_upload.php");
        let safe_name = {
            let cleaned: String = filename
                .chars()
                .map(|c| {
                    if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-' | ' ') {
                        c
                    } else {
                        '_'
                    }
                })
                .collect();
            let t = cleaned.trim().trim_matches('.');
            if t.is_empty() {
                "fichier.bin".to_string()
            } else {
                t.to_string()
            }
        };
        let mime_clean = {
            let m = mime.unwrap_or("application/octet-stream").trim();
            if m.is_empty() || !m.is_ascii() || m.bytes().any(|b| b <= 32) {
                "application/octet-stream".to_string()
            } else {
                m.to_string()
            }
        };
        let file_part = reqwest::multipart::Part::bytes(bytes)
            .file_name(safe_name)
            .mime_str(&mime_clean)
            .map_err(|e| format!("mime: {e}"))?;
        let mut form = reqwest::multipart::Form::new()
            .text("visibility", visibility.to_string())
            .text("token", raw.clone())
            .part("file", file_part);
        if let Some(pid) = project_id {
            if !pid.is_empty() {
                form = form.text("project_id", pid.to_string());
            }
        }
        let res = self
            .http
            .post(&url)
            .header(AUTHORIZATION, bearer)
            .header("X-Auth-Token", &raw)
            .multipart(form)
            .timeout(std::time::Duration::from_secs(120))
            .send()
            .await
            .map_err(|e| format!("réseau upload: {e}"))?;
        let status = res.status();
        let text = res.text().await.map_err(|e| e.to_string())?;
        if status.as_u16() == 401 {
            return Err("Session Gestion expirée — reconnecte-toi".into());
        }
        if !status.is_success() {
            let err = serde_json::from_str::<Value>(&text)
                .ok()
                .and_then(|v| {
                    v.get("error")
                        .and_then(|e| e.as_str())
                        .map(|s| s.to_string())
                })
                .unwrap_or_else(|| {
                    let snippet: String = text.chars().take(180).collect();
                    if snippet.is_empty() {
                        format!("HTTP {status}")
                    } else {
                        snippet
                    }
                });
            return Err(format!("upload {status}: {err}"));
        }
        let parsed: Value = serde_json::from_str(&text).map_err(|e| {
            format!(
                "upload JSON: {e} — {}",
                text.chars().take(200).collect::<String>()
            )
        })?;
        if parsed.get("success").and_then(|s| s.as_bool()) == Some(false) {
            let err = parsed
                .get("error")
                .and_then(|e| e.as_str())
                .unwrap_or("échec upload");
            return Err(err.to_string());
        }
        Ok(parsed)
    }
}

fn filename_from_content_disposition(header: &str) -> Option<String> {
    // filename="x.pdf" ou filename*=UTF-8''x.pdf
    if let Some(rest) = header
        .split("filename*=UTF-8''")
        .nth(1)
        .or_else(|| header.split("filename*=utf-8''").nth(1))
    {
        let raw = rest.split(';').next()?.trim().trim_matches('"');
        if !raw.is_empty() {
            return Some(
                percent_decode_lite(raw)
                    .chars()
                    .map(|c| if matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') { '_' } else { c })
                    .collect(),
            );
        }
    }
    let lower = header.to_ascii_lowercase();
    let idx = lower.find("filename=")?;
    let mut raw = header[idx + "filename=".len()..].trim();
    if let Some(end) = raw.find(';') {
        raw = &raw[..end];
    }
    raw = raw.trim().trim_matches('"');
    if raw.is_empty() {
        None
    } else {
        Some(
            raw.chars()
                .map(|c| if matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') { '_' } else { c })
                .collect(),
        )
    }
}

fn percent_decode_lite(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let h = |c: u8| -> Option<u8> {
                match c {
                    b'0'..=b'9' => Some(c - b'0'),
                    b'a'..=b'f' => Some(c - b'a' + 10),
                    b'A'..=b'F' => Some(c - b'A' + 10),
                    _ => None,
                }
            };
            if let (Some(a), Some(b)) = (h(bytes[i + 1]), h(bytes[i + 2])) {
                out.push((a << 4) | b);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Encodage query minimal (évite dépendance urlencoding).
fn urlencoding_lite(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 2);
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}
