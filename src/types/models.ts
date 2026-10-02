/**
 * Modèles V1 (contrat TS ↔ Rust) — camelCase, alignés sur le domaine Rust.
 */

export type Project = {
  id: string;
  name: string;
  color?: string;
  createdAt: string;
};

export type TaskStatus = "active" | "done";
export type TaskPriority = "low" | "normal" | "high";

export type Task = {
  id: string;
  title: string;
  status: TaskStatus;
  projectId: string | null;
  createdAt: string;
  updatedAt: string;
  dueDate?: string;
  reminder?: string;
  priority?: TaskPriority;
  notes?: string;
};

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
