//! Entités domaine V1 (contrat TS ↔ Rust).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    pub created_at: String,
}

/// Statuts alignés sur gestion `personal_tasks` (`todo` / `in_progress` / `done`).
/// `active` (legacy MIND) est accepté en lecture → `Todo`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Todo,
    InProgress,
    Done,
}

impl TaskStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Todo => "todo",
            Self::InProgress => "in_progress",
            Self::Done => "done",
        }
    }

    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "todo" | "active" => Ok(Self::Todo),
            "in_progress" | "inprogress" => Ok(Self::InProgress),
            "done" => Ok(Self::Done),
            other => Err(format!("invalid task status: {other}")),
        }
    }

    pub fn is_done(self) -> bool {
        matches!(self, Self::Done)
    }

    pub fn is_open(self) -> bool {
        !self.is_done()
    }
}

/// Priorités gestion (`low` / `medium` / `high`). `normal` (legacy) → `Medium`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum TaskPriority {
    Low,
    Medium,
    High,
}

impl TaskPriority {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
        }
    }

    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "low" => Ok(Self::Low),
            "medium" | "normal" => Ok(Self::Medium),
            "high" => Ok(Self::High),
            other => Err(format!("invalid task priority: {other}")),
        }
    }
}

/// Document joint (gestion) — forme souple.
pub type TaskDocument = serde_json::Value;
/// Activité / historique (gestion) — forme souple.
pub type TaskActivity = serde_json::Value;

/// Tâche — schéma gestion `personal_tasks` + extensions MIND (`projectId`, `reminder`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub category: String,
    pub status: TaskStatus,
    #[serde(default)]
    pub completed: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub priority: Option<TaskPriority>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub due_date: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assigned_to: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_by: Option<i64>,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub documents: Vec<TaskDocument>,
    #[serde(default)]
    pub activities: Vec<TaskActivity>,
    /// Overlay MIND (pas dans personal_tasks) — projet local.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    /// Overlay MIND — rappel local / notif Windows.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reminder: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl Task {
    pub fn normalize(&mut self) {
        if self.status.is_done() {
            self.completed = true;
        } else if self.completed {
            self.status = TaskStatus::Done;
        }
    }
}

/// Kind pour une Note (capture Note / Idée). Les tâches vont dans `Task`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum NoteKind {
    Note,
    Idea,
}

impl NoteKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Note => "note",
            Self::Idea => "idea",
        }
    }

    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "note" => Ok(Self::Note),
            "idea" => Ok(Self::Idea),
            other => Err(format!("invalid note kind: {other}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Note {
    pub id: String,
    /// Titre optionnel — projet jamais obligatoire.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Contenu libre (pas de duplication avec les Task).
    pub body: String,
    pub kind: NoteKind,
    pub project_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Reminder {
    pub id: String,
    pub target_id: String,
    pub fire_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snoozed_until: Option<String>,
}

/// Post-it autonome (pensée immédiate) — contenu dans `body`.
/// `note_id` optionnel : legacy des anciennes liaisons Note.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PostIt {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default)]
    pub body: String,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    pub always_on_top: bool,
    pub open: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskFilter {
    pub project_id: Option<String>,
    /// Si `true`, uniquement les tâches sans projet.
    pub no_project: Option<bool>,
    pub done: Option<bool>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteFilter {
    pub project_id: Option<String>,
    pub no_project: Option<bool>,
    pub kind: Option<NoteKind>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReminderFilter {
    pub target_id: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PostItFilter {
    pub note_id: Option<String>,
    pub open: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DataChangedPayload {
    pub entity: String,
    pub id: String,
}

pub fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

pub fn new_id() -> String {
    uuid::Uuid::new_v4().to_string()
}
