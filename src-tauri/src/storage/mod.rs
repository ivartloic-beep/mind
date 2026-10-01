//! Abstraction storage (swap cloud V2 derrière le même trait).

pub mod local;

use crate::domain::{
    Note, NoteFilter, PostIt, PostItFilter, Project, Reminder, ReminderFilter, Task, TaskFilter,
};

pub type StorageResult<T> = Result<T, StorageError>;

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Message(String),
}

impl serde::Serialize for StorageError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

pub trait Storage: Send {
    fn list_projects(&self) -> StorageResult<Vec<Project>>;
    fn get_project(&self, id: &str) -> StorageResult<Option<Project>>;
    fn upsert_project(&self, project: &Project) -> StorageResult<()>;
    fn delete_project(&self, id: &str) -> StorageResult<()>;

    fn list_tasks(&self, filter: &TaskFilter) -> StorageResult<Vec<Task>>;
    fn get_task(&self, id: &str) -> StorageResult<Option<Task>>;
    fn upsert_task(&self, task: &Task) -> StorageResult<()>;
    fn delete_task(&self, id: &str) -> StorageResult<()>;

    fn list_notes(&self, filter: &NoteFilter) -> StorageResult<Vec<Note>>;
    fn get_note(&self, id: &str) -> StorageResult<Option<Note>>;
    fn upsert_note(&self, note: &Note) -> StorageResult<()>;
    fn delete_note(&self, id: &str) -> StorageResult<()>;

    fn list_reminders(&self, filter: &ReminderFilter) -> StorageResult<Vec<Reminder>>;
    fn get_reminder(&self, id: &str) -> StorageResult<Option<Reminder>>;
    fn upsert_reminder(&self, reminder: &Reminder) -> StorageResult<()>;
    fn delete_reminder(&self, id: &str) -> StorageResult<()>;

    fn list_postits(&self, filter: &PostItFilter) -> StorageResult<Vec<PostIt>>;
    fn get_postit(&self, id: &str) -> StorageResult<Option<PostIt>>;
    fn upsert_postit(&self, postit: &PostIt) -> StorageResult<()>;
    fn delete_postit(&self, id: &str) -> StorageResult<()>;
}
