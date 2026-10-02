/**
 * Modèles V1 (contrat TS ↔ Rust) — camelCase, alignés sur le domaine Rust / gestion personal_tasks.
 */

export type Project = {
  id: string;
  name: string;
  color?: string;
  createdAt: string;
};

/** Statuts gestion (`active` legacy accepté côté Rust → todo). */
export type TaskStatus = "todo" | "in_progress" | "done";
export type TaskPriority = "low" | "medium" | "high";

export type Task = {
  id: string;
  title: string;
  description: string;
  category: string;
  status: TaskStatus;
  completed: boolean;
  priority?: TaskPriority;
  dueDate?: string;
  assignedTo?: number | null;
  createdBy?: number | null;
  notes: string;
  documents: unknown[];
  activities: unknown[];
  /** Overlay MIND — projet local. */
  projectId: string | null;
  /** Overlay MIND — rappel Windows. */
  reminder?: string;
  createdAt: string;
  updatedAt: string;
};

export function isTaskOpen(task: Pick<Task, "status" | "completed">): boolean {
  return task.status !== "done" && !task.completed;
}

export function isTaskDone(task: Pick<Task, "status" | "completed">): boolean {
  return task.status === "done" || task.completed;
}

/** Note / Idée uniquement — les tâches sont des `Task`. */
export type NoteKind = "note" | "idea";

export type Note = {
  id: string;
  /** Titre optionnel. */
  title?: string;
  /** Contenu libre. */
  body: string;
  kind: NoteKind;
  projectId: string | null;
  createdAt: string;
  updatedAt: string;
};

export type Reminder = {
  id: string;
  targetId: string;
  fireAt: string;
  snoozedUntil?: string;
};

/** Post-it autonome (pensée immédiate). `noteId` = legacy optionnel. */
export type PostIt = {
  id: string;
  noteId?: string;
  title?: string;
  body: string;
  x: number;
  y: number;
  w: number;
  h: number;
  alwaysOnTop: boolean;
  open: boolean;
};

export type TaskFilter = {
  projectId?: string | null;
  noProject?: boolean;
  done?: boolean;
};

export type NoteFilter = {
  projectId?: string | null;
  noProject?: boolean;
  kind?: NoteKind;
};

export type ReminderFilter = {
  targetId?: string;
};

export type PostItFilter = {
  noteId?: string;
  open?: boolean;
};

export type DataChangedPayload = {
  entity: string;
  id: string;
};
