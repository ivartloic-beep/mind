/**
 * Wrappers invoke Tauri — CRUD agrégats + capture directe Task/Note.
 */

import { invoke } from "@tauri-apps/api/core";
import type {
  Note,
  NoteFilter,
  NoteKind,
  PostIt,
  PostItFilter,
  Project,
  Reminder,
  ReminderFilter,
  Task,
  TaskFilter,
} from "../types/models";

export async function ping(): Promise<string> {
  return invoke<string>("ping");
}

// Projects
export const listProjects = () => invoke<Project[]>("list_projects");
export const getProject = (id: string) =>
  invoke<Project | null>("get_project", { id });
export const upsertProject = (project: Project) =>
  invoke<Project>("upsert_project", { project });
export const deleteProject = (id: string) =>
  invoke<void>("delete_project", { id });

// Tasks
export const listTasks = (filter?: TaskFilter) =>
  invoke<Task[]>("list_tasks", { filter: filter ?? null });
export const getTask = (id: string) => invoke<Task | null>("get_task", { id });
export const upsertTask = (task: Task) => invoke<Task>("upsert_task", { task });
export const deleteTask = (id: string) => invoke<void>("delete_task", { id });
/** Capture → Task directe. */
export const createTask = (title: string, projectId?: string | null) =>
  invoke<Task>("create_task", { title, projectId: projectId ?? null });
/** Coche / décoche immédiate. */
export const setTaskDone = (id: string, done: boolean) =>
  invoke<Task>("set_task_done", { id, done });

// Notes
export const listNotes = (filter?: NoteFilter) =>
  invoke<Note[]>("list_notes", { filter: filter ?? null });
export const getNote = (id: string) => invoke<Note | null>("get_note", { id });
export const upsertNote = (note: Note) => invoke<Note>("upsert_note", { note });
export const deleteNote = (id: string) => invoke<void>("delete_note", { id });
/** Capture / panneau → Note directe (note | idea). */
export const createNote = (
  body: string,
  kind: NoteKind,
  projectId?: string | null,
  title?: string | null,
) =>
  invoke<Note>("create_note", {
    body,
    kind,
    projectId: projectId ?? null,
    title: title ?? null,
  });

// Reminders
export const listReminders = (filter?: ReminderFilter) =>
  invoke<Reminder[]>("list_reminders", { filter: filter ?? null });
export const getReminder = (id: string) =>
  invoke<Reminder | null>("get_reminder", { id });
export const upsertReminder = (reminder: Reminder) =>
  invoke<Reminder>("upsert_reminder", { reminder });
export const deleteReminder = (id: string) =>
  invoke<void>("delete_reminder", { id });

export type SnoozeKind = "10m" | "1h" | "tomorrow";

/** Associe / met à jour un rappel sur une tâche. */
export const setTaskReminder = (taskId: string, fireAt: string) =>
  invoke<Reminder>("set_task_reminder", { taskId, fireAt });
export const clearTaskReminder = (taskId: string) =>
  invoke<void>("clear_task_reminder", { taskId });
export const snoozeReminder = (id: string, kind: SnoozeKind) =>
  invoke<Reminder>("snooze_reminder", { id, kind });
export const dismissReminder = (id: string) =>
  invoke<void>("dismiss_reminder", { id });
export const openTaskFromReminder = (taskId: string) =>
  invoke<void>("open_task_from_reminder", { taskId });

// Autostart (étape 12)
export const autostartIsEnabled = () => invoke<boolean>("autostart_is_enabled");
export const autostartSetEnabled = (enabled: boolean) =>
  invoke<boolean>("autostart_set_enabled", { enabled });

// PostIts
export const listPostits = (filter?: PostItFilter) =>
  invoke<PostIt[]>("list_postits", { filter: filter ?? null });
export const getPostit = (id: string) =>
  invoke<PostIt | null>("get_postit", { id });
export const upsertPostit = (postit: PostIt) =>
  invoke<PostIt>("upsert_postit", { postit });
export const deletePostit = (id: string) =>
  invoke<void>("delete_postit", { id });
