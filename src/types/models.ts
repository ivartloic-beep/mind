/**
 * Modèles V1 (contrat TS ↔ Rust) — stubs étape 1.
 * Implémentation storage / CRUD = étape 2.
 */

export type Project = {
  id: string;
  name: string;
  color?: string;
  createdAt: string;
};

export type Task = {
  id: string;
  title: string;
  done: boolean;
  projectId: string | null;
  createdAt: string;
  updatedAt: string;
};

export type Note = {
  id: string;
  body: string;
  kind: "task" | "note" | "idea";
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

export type PostIt = {
  id: string;
  noteId: string;
  x: number;
  y: number;
  w: number;
  h: number;
  alwaysOnTop: boolean;
  open: boolean;
};
