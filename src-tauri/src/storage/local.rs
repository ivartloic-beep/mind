//! LocalStorage SQLite (`rusqlite`) — source de vérité V1.

use std::path::Path;
use std::sync::Mutex;

use rusqlite::{params, Connection, OptionalExtension};

use crate::domain::{
    Note, NoteFilter, NoteKind, PostIt, PostItFilter, Project, Reminder, ReminderFilter, Task,
    TaskFilter,
};
use crate::storage::{Storage, StorageError, StorageResult};

pub struct LocalStorage {
    conn: Mutex<Connection>,
}

impl LocalStorage {
    pub fn open(path: impl AsRef<Path>) -> StorageResult<Self> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "
            PRAGMA foreign_keys = ON;
            PRAGMA journal_mode = WAL;
            ",
        )?;
        let storage = Self {
            conn: Mutex::new(conn),
        };
        storage.migrate()?;
        Ok(storage)
    }

    fn with_conn<T>(&self, f: impl FnOnce(&Connection) -> StorageResult<T>) -> StorageResult<T> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| StorageError::Message("storage lock poisoned".into()))?;
        f(&conn)
    }

    fn migrate(&self) -> StorageResult<()> {
        self.with_conn(|conn| {
            conn.execute_batch(
                "
                CREATE TABLE IF NOT EXISTS projects (
                  id TEXT PRIMARY KEY NOT NULL,
                  name TEXT NOT NULL,
                  color TEXT,
                  created_at TEXT NOT NULL
                );

                CREATE TABLE IF NOT EXISTS tasks (
                  id TEXT PRIMARY KEY NOT NULL,
                  title TEXT NOT NULL,
                  done INTEGER NOT NULL DEFAULT 0,
                  status TEXT NOT NULL DEFAULT 'active',
                  project_id TEXT REFERENCES projects(id) ON DELETE SET NULL,
                  created_at TEXT NOT NULL,
                  updated_at TEXT NOT NULL,
                  due_date TEXT,
                  reminder TEXT,
                  priority TEXT,
                  notes TEXT
                );

                CREATE TABLE IF NOT EXISTS notes (
                  id TEXT PRIMARY KEY NOT NULL,
                  title TEXT,
                  body TEXT NOT NULL,
                  kind TEXT NOT NULL,
                  project_id TEXT REFERENCES projects(id) ON DELETE SET NULL,
                  created_at TEXT NOT NULL,
                  updated_at TEXT NOT NULL
                );

                CREATE TABLE IF NOT EXISTS reminders (
                  id TEXT PRIMARY KEY NOT NULL,
                  target_id TEXT NOT NULL,
                  fire_at TEXT NOT NULL,
                  snoozed_until TEXT
                );

                CREATE TABLE IF NOT EXISTS postits (
                  id TEXT PRIMARY KEY NOT NULL,
                  note_id TEXT,
                  title TEXT,
                  body TEXT NOT NULL DEFAULT '',
                  x REAL NOT NULL,
                  y REAL NOT NULL,
                  w REAL NOT NULL,
                  h REAL NOT NULL,
                  always_on_top INTEGER NOT NULL DEFAULT 1,
                  open INTEGER NOT NULL DEFAULT 1
                );

                CREATE INDEX IF NOT EXISTS idx_tasks_project ON tasks(project_id);
                CREATE INDEX IF NOT EXISTS idx_notes_project ON notes(project_id);
                CREATE INDEX IF NOT EXISTS idx_reminders_target ON reminders(target_id);
                CREATE INDEX IF NOT EXISTS idx_postits_note ON postits(note_id);
                ",
            )?;
            migrate_tasks_columns(conn)?;
            migrate_notes_columns(conn)?;
            migrate_postits_autonomous(conn)?;
            Ok(())
        })
    }
}

fn table_columns(conn: &Connection, table: &str) -> StorageResult<Vec<String>> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(1))?;
    let mut cols = Vec::new();
    for col in rows {
        cols.push(col?);
    }
    Ok(cols)
}

fn ensure_column(conn: &Connection, table: &str, column: &str, ddl: &str) -> StorageResult<()> {
    let cols = table_columns(conn, table)?;
    if !cols.iter().any(|c| c == column) {
        conn.execute(&format!("ALTER TABLE {table} ADD COLUMN {ddl}"), [])?;
    }
    Ok(())
}

fn migrate_tasks_columns(conn: &Connection) -> StorageResult<()> {
    ensure_column(conn, "tasks", "status", "status TEXT NOT NULL DEFAULT 'active'")?;
    ensure_column(conn, "tasks", "due_date", "due_date TEXT")?;
    ensure_column(conn, "tasks", "reminder", "reminder TEXT")?;
    ensure_column(conn, "tasks", "priority", "priority TEXT")?;
    ensure_column(conn, "tasks", "notes", "notes TEXT")?;
    // Bases étape 2–4 : synchroniser status depuis l'ancien booléen `done`.
    conn.execute(
        "UPDATE tasks SET status = 'done' WHERE done = 1 AND status != 'done'",
        [],
    )?;
    conn.execute(
        "UPDATE tasks SET done = 1 WHERE status = 'done' AND done != 1",
        [],
    )?;
    conn.execute(
        "UPDATE tasks SET done = 0 WHERE status = 'active' AND done != 0",
        [],
    )?;
    Ok(())
}

fn migrate_notes_columns(conn: &Connection) -> StorageResult<()> {
    ensure_column(conn, "notes", "title", "title TEXT")?;
    Ok(())
}

/// Post-its autonomes : body/title + note_id nullable (rebuild une fois).
fn migrate_postits_autonomous(conn: &Connection) -> StorageResult<()> {
    let cols = table_columns(conn, "postits")?;
    if cols.iter().any(|c| c == "body") {
        return Ok(());
    }
    conn.execute_batch(
        "
        CREATE TABLE postits_new (
          id TEXT PRIMARY KEY NOT NULL,
          note_id TEXT,
          title TEXT,
          body TEXT NOT NULL DEFAULT '',
          x REAL NOT NULL,
          y REAL NOT NULL,
          w REAL NOT NULL,
          h REAL NOT NULL,
          always_on_top INTEGER NOT NULL DEFAULT 1,
          open INTEGER NOT NULL DEFAULT 1
        );
        INSERT INTO postits_new (id, note_id, title, body, x, y, w, h, always_on_top, open)
        SELECT p.id, p.note_id, NULL,
               COALESCE((SELECT n.body FROM notes n WHERE n.id = p.note_id), ''),
               p.x, p.y, p.w, p.h, p.always_on_top, p.open
        FROM postits p;
        DROP TABLE postits;
        ALTER TABLE postits_new RENAME TO postits;
        CREATE INDEX IF NOT EXISTS idx_postits_note ON postits(note_id);
        ",
    )?;
    Ok(())
}

impl Storage for LocalStorage {
    fn list_projects(&self) -> StorageResult<Vec<Project>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, name, color, created_at FROM projects ORDER BY created_at ASC",
            )?;
            let rows = stmt.query_map([], |row| {
                Ok(Project {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    color: row.get(2)?,
                    created_at: row.get(3)?,
                })
            })?;
            rows.collect::<Result<Vec<_>, _>>()
                .map_err(StorageError::from)
        })
    }

    fn get_project(&self, id: &str) -> StorageResult<Option<Project>> {
        self.with_conn(|conn| {
            conn.query_row(
                "SELECT id, name, color, created_at FROM projects WHERE id = ?1",
                params![id],
                |row| {
                    Ok(Project {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        color: row.get(2)?,
                        created_at: row.get(3)?,
                    })
                },
            )
            .optional()
            .map_err(StorageError::from)
        })
    }

    fn upsert_project(&self, project: &Project) -> StorageResult<()> {
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO projects (id, name, color, created_at)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(id) DO UPDATE SET
                   name = excluded.name,
                   color = excluded.color",
                params![project.id, project.name, project.color, project.created_at],
            )?;
            Ok(())
        })
    }

    fn delete_project(&self, id: &str) -> StorageResult<()> {
        self.with_conn(|conn| {
            conn.execute("DELETE FROM projects WHERE id = ?1", params![id])?;
            Ok(())
        })
    }

    fn list_tasks(&self, filter: &TaskFilter) -> StorageResult<Vec<Task>> {
        self.with_conn(|conn| {
            let mut sql = String::from(
                "SELECT id, title, status, project_id, created_at, updated_at, due_date, reminder, priority, notes
                 FROM tasks WHERE 1=1",
            );
            let mut values: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

            if let Some(true) = filter.no_project {
                sql.push_str(" AND project_id IS NULL");
            } else if let Some(ref project_id) = filter.project_id {
                sql.push_str(" AND project_id = ?");
                values.push(Box::new(project_id.clone()));
            }
            if let Some(done) = filter.done {
                sql.push_str(" AND status = ?");
                values.push(Box::new(
                    if done { "done" } else { "active" }.to_string(),
                ));
            }
            sql.push_str(" ORDER BY created_at DESC");

            let mut stmt = conn.prepare(&sql)?;
            let params_refs: Vec<&dyn rusqlite::ToSql> = values.iter().map(|v| v.as_ref()).collect();
            let rows = stmt.query_map(params_refs.as_slice(), map_task)?;
            rows.collect::<Result<Vec<_>, _>>()
                .map_err(StorageError::from)
        })
    }

    fn get_task(&self, id: &str) -> StorageResult<Option<Task>> {
        self.with_conn(|conn| {
            conn.query_row(
                "SELECT id, title, status, project_id, created_at, updated_at, due_date, reminder, priority, notes
                 FROM tasks WHERE id = ?1",
                params![id],
                map_task,
            )
            .optional()
            .map_err(StorageError::from)
        })
    }

    fn upsert_task(&self, task: &Task) -> StorageResult<()> {
        self.with_conn(|conn| {
            let done = if task.status.is_done() { 1 } else { 0 };
            let priority = task.priority.map(|p| p.as_str().to_string());
            conn.execute(
                "INSERT INTO tasks (
                    id, title, done, status, project_id, created_at, updated_at,
                    due_date, reminder, priority, notes
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
                 ON CONFLICT(id) DO UPDATE SET
                   title = excluded.title,
                   done = excluded.done,
                   status = excluded.status,
                   project_id = excluded.project_id,
                   updated_at = excluded.updated_at,
                   due_date = excluded.due_date,
                   reminder = excluded.reminder,
                   priority = excluded.priority,
                   notes = excluded.notes",
                params![
                    task.id,
                    task.title,
                    done,
                    task.status.as_str(),
                    task.project_id,
                    task.created_at,
                    task.updated_at,
                    task.due_date,
                    task.reminder,
                    priority,
                    task.notes
                ],
            )?;
            Ok(())
        })
    }

    fn delete_task(&self, id: &str) -> StorageResult<()> {
        self.with_conn(|conn| {
            conn.execute("DELETE FROM tasks WHERE id = ?1", params![id])?;
            Ok(())
        })
    }

    fn list_notes(&self, filter: &NoteFilter) -> StorageResult<Vec<Note>> {
        self.with_conn(|conn| {
            let mut sql = String::from(
                "SELECT id, title, body, kind, project_id, created_at, updated_at FROM notes WHERE 1=1",
            );
            let mut values: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

            if let Some(true) = filter.no_project {
                sql.push_str(" AND project_id IS NULL");
            } else if let Some(ref project_id) = filter.project_id {
                sql.push_str(" AND project_id = ?");
                values.push(Box::new(project_id.clone()));
            }
            if let Some(kind) = filter.kind {
                sql.push_str(" AND kind = ?");
                values.push(Box::new(kind.as_str().to_string()));
            }
            sql.push_str(" ORDER BY updated_at DESC");

            let mut stmt = conn.prepare(&sql)?;
            let params_refs: Vec<&dyn rusqlite::ToSql> = values.iter().map(|v| v.as_ref()).collect();
            let rows = stmt.query_map(params_refs.as_slice(), map_note)?;
            rows.collect::<Result<Vec<_>, _>>()
                .map_err(StorageError::from)
        })
    }

    fn get_note(&self, id: &str) -> StorageResult<Option<Note>> {
        self.with_conn(|conn| {
            conn.query_row(
                "SELECT id, title, body, kind, project_id, created_at, updated_at FROM notes WHERE id = ?1",
                params![id],
                map_note,
            )
            .optional()
            .map_err(StorageError::from)
        })
    }

    fn upsert_note(&self, note: &Note) -> StorageResult<()> {
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO notes (id, title, body, kind, project_id, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                 ON CONFLICT(id) DO UPDATE SET
                   title = excluded.title,
                   body = excluded.body,
                   kind = excluded.kind,
                   project_id = excluded.project_id,
                   updated_at = excluded.updated_at",
                params![
                    note.id,
                    note.title,
                    note.body,
                    note.kind.as_str(),
                    note.project_id,
                    note.created_at,
                    note.updated_at
                ],
            )?;
            Ok(())
        })
    }

    fn delete_note(&self, id: &str) -> StorageResult<()> {
        self.with_conn(|conn| {
            conn.execute("DELETE FROM notes WHERE id = ?1", params![id])?;
            Ok(())
        })
    }

    fn list_reminders(&self, filter: &ReminderFilter) -> StorageResult<Vec<Reminder>> {
        self.with_conn(|conn| {
            let mut sql =
                String::from("SELECT id, target_id, fire_at, snoozed_until FROM reminders WHERE 1=1");
            let mut values: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
            if let Some(ref target_id) = filter.target_id {
                sql.push_str(" AND target_id = ?");
                values.push(Box::new(target_id.clone()));
            }
            sql.push_str(" ORDER BY fire_at ASC");
            let mut stmt = conn.prepare(&sql)?;
            let params_refs: Vec<&dyn rusqlite::ToSql> = values.iter().map(|v| v.as_ref()).collect();
            let rows = stmt.query_map(params_refs.as_slice(), map_reminder)?;
            rows.collect::<Result<Vec<_>, _>>()
                .map_err(StorageError::from)
        })
    }

    fn get_reminder(&self, id: &str) -> StorageResult<Option<Reminder>> {
        self.with_conn(|conn| {
            conn.query_row(
                "SELECT id, target_id, fire_at, snoozed_until FROM reminders WHERE id = ?1",
                params![id],
                map_reminder,
            )
            .optional()
            .map_err(StorageError::from)
        })
    }

    fn upsert_reminder(&self, reminder: &Reminder) -> StorageResult<()> {
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO reminders (id, target_id, fire_at, snoozed_until)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(id) DO UPDATE SET
                   target_id = excluded.target_id,
                   fire_at = excluded.fire_at,
                   snoozed_until = excluded.snoozed_until",
                params![
                    reminder.id,
                    reminder.target_id,
                    reminder.fire_at,
                    reminder.snoozed_until
                ],
            )?;
            Ok(())
        })
    }

    fn delete_reminder(&self, id: &str) -> StorageResult<()> {
        self.with_conn(|conn| {
            conn.execute("DELETE FROM reminders WHERE id = ?1", params![id])?;
            Ok(())
        })
    }

    fn list_postits(&self, filter: &PostItFilter) -> StorageResult<Vec<PostIt>> {
        self.with_conn(|conn| {
            let mut sql = String::from(
                "SELECT id, note_id, title, body, x, y, w, h, always_on_top, open FROM postits WHERE 1=1",
            );
            let mut values: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
            if let Some(ref note_id) = filter.note_id {
                sql.push_str(" AND note_id = ?");
                values.push(Box::new(note_id.clone()));
            }
            if let Some(open) = filter.open {
                sql.push_str(" AND open = ?");
                values.push(Box::new(if open { 1 } else { 0 }));
            }
            sql.push_str(" ORDER BY id ASC");
            let mut stmt = conn.prepare(&sql)?;
            let params_refs: Vec<&dyn rusqlite::ToSql> = values.iter().map(|v| v.as_ref()).collect();
            let rows = stmt.query_map(params_refs.as_slice(), map_postit)?;
            rows.collect::<Result<Vec<_>, _>>()
                .map_err(StorageError::from)
        })
    }

    fn get_postit(&self, id: &str) -> StorageResult<Option<PostIt>> {
        self.with_conn(|conn| {
            conn.query_row(
                "SELECT id, note_id, title, body, x, y, w, h, always_on_top, open FROM postits WHERE id = ?1",
                params![id],
                map_postit,
            )
            .optional()
            .map_err(StorageError::from)
        })
    }

    fn upsert_postit(&self, postit: &PostIt) -> StorageResult<()> {
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO postits (id, note_id, title, body, x, y, w, h, always_on_top, open)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
                 ON CONFLICT(id) DO UPDATE SET
                   note_id = excluded.note_id,
                   title = excluded.title,
                   body = excluded.body,
                   x = excluded.x,
                   y = excluded.y,
                   w = excluded.w,
                   h = excluded.h,
                   always_on_top = excluded.always_on_top,
                   open = excluded.open",
                params![
                    postit.id,
                    postit.note_id,
                    postit.title,
                    postit.body,
                    postit.x,
                    postit.y,
                    postit.w,
                    postit.h,
                    postit.always_on_top as i32,
                    postit.open as i32
                ],
            )?;
            Ok(())
        })
    }

    fn delete_postit(&self, id: &str) -> StorageResult<()> {
        self.with_conn(|conn| {
            conn.execute("DELETE FROM postits WHERE id = ?1", params![id])?;
            Ok(())
        })
    }
}

fn map_task(row: &rusqlite::Row<'_>) -> rusqlite::Result<Task> {
    use crate::domain::{TaskPriority, TaskStatus};

    let status_raw: String = row.get(2)?;
    let status = TaskStatus::parse(&status_raw).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(2, rusqlite::types::Type::Text, e.into())
    })?;
    let priority_raw: Option<String> = row.get(8)?;
    let priority = match priority_raw {
        Some(raw) => Some(TaskPriority::parse(&raw).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(8, rusqlite::types::Type::Text, e.into())
        })?),
        None => None,
    };
    Ok(Task {
        id: row.get(0)?,
        title: row.get(1)?,
        status,
        project_id: row.get(3)?,
        created_at: row.get(4)?,
        updated_at: row.get(5)?,
        due_date: row.get(6)?,
        reminder: row.get(7)?,
        priority,
        notes: row.get(9)?,
    })
}

fn map_note(row: &rusqlite::Row<'_>) -> rusqlite::Result<Note> {
    let kind_raw: String = row.get(3)?;
    let kind = NoteKind::parse(&kind_raw).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(3, rusqlite::types::Type::Text, e.into())
    })?;
    Ok(Note {
        id: row.get(0)?,
        title: row.get(1)?,
        body: row.get(2)?,
        kind,
        project_id: row.get(4)?,
        created_at: row.get(5)?,
        updated_at: row.get(6)?,
    })
}

fn map_reminder(row: &rusqlite::Row<'_>) -> rusqlite::Result<Reminder> {
    Ok(Reminder {
        id: row.get(0)?,
        target_id: row.get(1)?,
        fire_at: row.get(2)?,
        snoozed_until: row.get(3)?,
    })
}

fn map_postit(row: &rusqlite::Row<'_>) -> rusqlite::Result<PostIt> {
    let note_id: Option<String> = row.get(1)?;
    let note_id = note_id.filter(|s| !s.is_empty());
    Ok(PostIt {
        id: row.get(0)?,
        note_id,
        title: row.get(2)?,
        body: row.get(3)?,
        x: row.get(4)?,
        y: row.get(5)?,
        w: row.get(6)?,
        h: row.get(7)?,
        always_on_top: row.get::<_, i32>(8)? != 0,
        open: row.get::<_, i32>(9)? != 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{new_id, now_iso, NoteKind};

    #[test]
    fn persists_across_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("ma-tete.db");

        {
            let db = LocalStorage::open(&db_path).unwrap();
            let now = now_iso();
            let project = Project {
                id: new_id(),
                name: "Perso".into(),
                color: Some("#2f5d50".into()),
                created_at: now.clone(),
            };
            db.upsert_project(&project).unwrap();

            let task = Task {
                id: new_id(),
                title: "Acheter lait".into(),
                status: crate::domain::TaskStatus::Active,
                project_id: Some(project.id.clone()),
                created_at: now.clone(),
                updated_at: now.clone(),
                due_date: None,
                reminder: None,
                priority: Some(crate::domain::TaskPriority::Normal),
                notes: None,
            };
            db.upsert_task(&task).unwrap();

            let mut done_task = task.clone();
            done_task.id = new_id();
            done_task.title = "Déjà fait".into();
            done_task.status = crate::domain::TaskStatus::Done;
            db.upsert_task(&done_task).unwrap();

            let note = Note {
                id: new_id(),
                title: Some("V2".into()),
                body: "Idée pour V2".into(),
                kind: NoteKind::Idea,
                project_id: None,
                created_at: now.clone(),
                updated_at: now.clone(),
            };
            db.upsert_note(&note).unwrap();

            let postit = PostIt {
                id: new_id(),
                note_id: None,
                title: None,
                body: "Pensée rapide".into(),
                x: 40.0,
                y: 80.0,
                w: 240.0,
                h: 200.0,
                always_on_top: true,
                open: true,
            };
            db.upsert_postit(&postit).unwrap();

            let reminder = Reminder {
                id: new_id(),
                target_id: task.id.clone(),
                fire_at: now.clone(),
                snoozed_until: None,
            };
            db.upsert_reminder(&reminder).unwrap();
        }

        let db = LocalStorage::open(&db_path).unwrap();
        assert_eq!(db.list_projects().unwrap().len(), 1);
        assert_eq!(db.list_tasks(&TaskFilter::default()).unwrap().len(), 2);
        assert_eq!(
            db.list_tasks(&TaskFilter {
                done: Some(false),
                ..Default::default()
            })
            .unwrap()
            .len(),
            1
        );
        assert_eq!(db.list_notes(&NoteFilter::default()).unwrap().len(), 1);
        assert_eq!(db.list_postits(&PostItFilter::default()).unwrap().len(), 1);
        assert_eq!(
            db.list_reminders(&ReminderFilter::default()).unwrap().len(),
            1
        );
        let note = &db.list_notes(&NoteFilter::default()).unwrap()[0];
        assert_eq!(note.kind, NoteKind::Idea);
        assert_eq!(note.title.as_deref(), Some("V2"));
        assert!(note.project_id.is_none());
    }
}
