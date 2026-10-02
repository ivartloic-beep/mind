//! Mapping desktop V1 ↔ mind-api cloud.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::domain::{
    Note, NoteKind, PostIt, Project, Reminder, Task, TaskPriority, TaskStatus,
};

const META_PREFIX: &str = "\n\n<!--mind-meta:";
const META_SUFFIX: &str = "-->";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudProject {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(default)]
    pub archived: bool,
    #[serde(default)]
    pub sort_order: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deleted_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudTask {
    pub id: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    #[serde(default)]
    pub notes: String,
    #[serde(default = "default_todo")]
    pub status: String,
    #[serde(default)]
    pub priority: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub due_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<String>,
    #[serde(default)]
    pub sort_order: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deleted_at: Option<String>,
}

fn default_todo() -> String {
    "todo".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudNote {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub body: String,
    #[serde(default)]
    pub pinned: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deleted_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudReminder {
    pub id: String,
    pub title: String,
    pub remind_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
    #[serde(default)]
    pub body: String,
    #[serde(default)]
    pub done: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deleted_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudPostIt {
    pub id: String,
    #[serde(default)]
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(default)]
    pub x: f64,
    #[serde(default)]
    pub y: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<f64>,
    #[serde(default)]
    pub z_index: i64,
    #[serde(default)]
    pub pinned: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deleted_at: Option<String>,
}

fn attach_meta(base: &str, meta: Value) -> String {
    let raw = serde_json::to_string(&meta).unwrap_or_else(|_| "{}".into());
    format!("{base}{META_PREFIX}{raw}{META_SUFFIX}")
}

fn split_meta(raw: &str) -> (String, Value) {
    if let Some(start) = raw.rfind(META_PREFIX) {
        let after = &raw[start + META_PREFIX.len()..];
        if let Some(end) = after.find(META_SUFFIX) {
            let json_str = &after[..end];
            let meta: Value = serde_json::from_str(json_str).unwrap_or(json!({}));
            let body = raw[..start].to_string();
            return (body, meta);
        }
    }
    (raw.to_string(), json!({}))
}

fn meta_str(meta: &Value, key: &str) -> Option<String> {
    meta.get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

fn meta_bool(meta: &Value, key: &str, default: bool) -> bool {
    meta.get(key).and_then(|v| v.as_bool()).unwrap_or(default)
}

pub fn project_to_cloud(p: &Project) -> CloudProject {
    CloudProject {
        id: p.id.clone(),
        name: p.name.clone(),
        description: String::new(),
        color: p.color.clone(),
        icon: None,
        archived: false,
        sort_order: 0,
        created_at: Some(p.created_at.clone()),
        updated_at: None,
        deleted_at: None,
    }
}

pub fn project_from_cloud(c: &CloudProject) -> Project {
    Project {
        id: c.id.clone(),
        name: c.name.clone(),
        color: c.color.clone(),
        created_at: c
            .created_at
            .clone()
            .unwrap_or_else(|| chrono::Utc::now().to_rfc3339()),
    }
}

fn priority_to_int(p: Option<TaskPriority>) -> i64 {
    match p {
        Some(TaskPriority::Low) => 0,
        Some(TaskPriority::Medium) | None => 1,
        Some(TaskPriority::High) => 2,
    }
}

fn priority_from_int(n: i64) -> Option<TaskPriority> {
    match n {
        ..=0 => Some(TaskPriority::Low),
        1 => Some(TaskPriority::Medium),
        _ => Some(TaskPriority::High),
    }
}

pub fn task_to_cloud(t: &Task) -> CloudTask {
    let done = t.status.is_done();
    let notes = if let Some(rem) = &t.reminder {
        attach_meta(&t.notes, json!({ "reminder": rem }))
    } else {
        t.notes.clone()
    };
    CloudTask {
        id: t.id.clone(),
        title: t.title.clone(),
        project_id: t.project_id.clone(),
        notes,
        status: if done {
            "done".into()
        } else {
            t.status.as_str().into()
        },
        priority: priority_to_int(t.priority),
        due_at: t.due_date.clone(),
        completed_at: if done {
            Some(t.updated_at.clone())
        } else {
            None
        },
        sort_order: 0,
        created_at: Some(t.created_at.clone()),
        updated_at: Some(t.updated_at.clone()),
        deleted_at: None,
    }
}

pub fn task_from_cloud(c: &CloudTask) -> Task {
    let (notes_body, meta) = split_meta(&c.notes);
    let status = if c.status == "done" || c.completed_at.is_some() {
        TaskStatus::Done
    } else {
        TaskStatus::parse(&c.status).unwrap_or(TaskStatus::Todo)
    };
    let mut task = Task {
        id: c.id.clone(),
        title: c.title.clone(),
        description: String::new(),
        category: String::new(),
        status,
        completed: status.is_done(),
        priority: priority_from_int(c.priority),
        due_date: c.due_at.clone(),
        assigned_to: None,
        created_by: None,
        notes: notes_body,
        documents: Vec::new(),
        activities: Vec::new(),
        project_id: c.project_id.clone(),
        reminder: meta_str(&meta, "reminder"),
        created_at: c
            .created_at
            .clone()
            .unwrap_or_else(|| chrono::Utc::now().to_rfc3339()),
        updated_at: c
            .updated_at
            .clone()
            .unwrap_or_else(|| chrono::Utc::now().to_rfc3339()),
    };
    task.normalize();
    task
}

pub fn note_to_cloud(n: &Note) -> CloudNote {
    let body = attach_meta(
        &n.body,
        json!({ "kind": n.kind.as_str() }),
    );
    CloudNote {
        id: n.id.clone(),
        project_id: n.project_id.clone(),
        title: n.title.clone().unwrap_or_default(),
        body,
        pinned: matches!(n.kind, NoteKind::Idea),
        created_at: Some(n.created_at.clone()),
        updated_at: Some(n.updated_at.clone()),
        deleted_at: None,
    }
}

pub fn note_from_cloud(c: &CloudNote) -> Note {
    let (body, meta) = split_meta(&c.body);
    let kind = match meta_str(&meta, "kind").as_deref() {
        Some("idea") => NoteKind::Idea,
        Some("note") => NoteKind::Note,
        _ if c.pinned => NoteKind::Idea,
        _ => NoteKind::Note,
    };
    Note {
        id: c.id.clone(),
        title: if c.title.is_empty() {
            None
        } else {
            Some(c.title.clone())
        },
        body,
        kind,
        project_id: c.project_id.clone(),
        created_at: c
            .created_at
            .clone()
            .unwrap_or_else(|| chrono::Utc::now().to_rfc3339()),
        updated_at: c
            .updated_at
            .clone()
            .unwrap_or_else(|| chrono::Utc::now().to_rfc3339()),
    }
}

pub fn reminder_to_cloud(r: &Reminder) -> CloudReminder {
    let body = if let Some(s) = &r.snoozed_until {
        attach_meta("", json!({ "snoozedUntil": s }))
    } else {
        String::new()
    };
    CloudReminder {
        id: r.id.clone(),
        title: "Rappel".into(),
        remind_at: r.fire_at.clone(),
        project_id: None,
        task_id: Some(r.target_id.clone()),
        body,
        done: false,
        created_at: None,
        updated_at: None,
        deleted_at: None,
    }
}

pub fn reminder_from_cloud(c: &CloudReminder) -> Option<Reminder> {
    let target = c.task_id.clone()?;
    let (_body, meta) = split_meta(&c.body);
    Some(Reminder {
        id: c.id.clone(),
        target_id: target,
        fire_at: c.remind_at.clone(),
        snoozed_until: meta_str(&meta, "snoozedUntil"),
    })
}

pub fn postit_to_cloud(p: &PostIt) -> CloudPostIt {
    let text = attach_meta(
        &p.body,
        json!({
            "title": p.title,
            "noteId": p.note_id,
            "alwaysOnTop": p.always_on_top,
            "open": p.open,
        }),
    );
    CloudPostIt {
        id: p.id.clone(),
        text,
        color: None,
        x: p.x,
        y: p.y,
        width: Some(p.w),
        height: Some(p.h),
        z_index: if p.open { 0 } else { -1 },
        pinned: p.always_on_top,
        created_at: None,
        updated_at: None,
        deleted_at: None,
    }
}

pub fn postit_from_cloud(c: &CloudPostIt) -> PostIt {
    let (body, meta) = split_meta(&c.text);
    PostIt {
        id: c.id.clone(),
        note_id: meta_str(&meta, "noteId"),
        title: meta_str(&meta, "title"),
        body,
        x: c.x,
        y: c.y,
        w: c.width.unwrap_or(280.0),
        h: c.height.unwrap_or(240.0),
        always_on_top: meta_bool(&meta, "alwaysOnTop", c.pinned),
        open: meta_bool(&meta, "open", c.z_index >= 0),
    }
}
