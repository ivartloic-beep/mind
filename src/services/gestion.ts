/**
 * Shell Gestion — config API PHP, session, migration tâches.
 */

import { invoke } from "@tauri-apps/api/core";

export type GestionConfig = {
  apiUrl: string;
  authToken?: string | null;
  userId?: number | null;
  userName?: string | null;
  tasksMigratedAt?: string | null;
  lastError?: string | null;
};

export type GestionMigrateReport = {
  created: number;
  skipped: number;
  errors: string[];
  migratedAt?: string | null;
};

export function gestionShow(): Promise<void> {
  return invoke("gestion_show");
}

/** Deep-link Gestion : bureau | crm | projects. */
export function gestionShowPage(
  page: "bureau" | "crm" | "projects",
): Promise<void> {
  return invoke("gestion_show_page", { page });
}

/** Ouvre la fiche tâche Gestion (docs, notes, activités). */
export function gestionShowTask(taskId: string): Promise<void> {
  return invoke("gestion_show_task", { taskId });
}

export function gestionGetConfig(): Promise<GestionConfig> {
  return invoke<GestionConfig>("gestion_get_config");
}

export function gestionSetConfig(apiUrl: string): Promise<GestionConfig> {
  return invoke<GestionConfig>("gestion_set_config", { apiUrl });
}

export function gestionLogin(
  username: string,
  password: string,
): Promise<GestionConfig> {
  return invoke<GestionConfig>("gestion_login", { username, password });
}

export function gestionLogout(): Promise<GestionConfig> {
  return invoke<GestionConfig>("gestion_logout");
}

export function gestionMigrateLocalTasks(): Promise<GestionMigrateReport> {
  return invoke<GestionMigrateReport>("gestion_migrate_local_tasks");
}

export function gestionTasksBackendActive(): Promise<boolean> {
  return invoke<boolean>("gestion_tasks_backend_active");
}

export type GestionSyncReport = {
  tasks: number;
  projects: number;
  notes: number;
  active: boolean;
};

/** Sync bidirectionnel panneau ↔ Gestion (tâches, projets, notes bureau). */
export function gestionSyncNow(): Promise<GestionSyncReport> {
  return invoke<GestionSyncReport>("gestion_sync_now");
}

export type GestionUploadReport = {
  filename: string;
  visibility: string;
  projectId?: string | null;
  elementId?: string | null;
};

export type DroppedFilePayload = {
  filename: string;
  mime: string;
  dataBase64: string;
  size: number;
};

export type DroppedFileMeta = {
  filename: string;
  mime: string;
  size: number;
  path: string;
};

/** Métadonnées d’un fichier OS déposé (sans charger le contenu). */
export function peekDroppedFile(path: string): Promise<DroppedFileMeta> {
  return invoke<DroppedFileMeta>("peek_dropped_file", { path });
}

/** Lit un fichier OS déposé (chemins Tauri drag-drop). */
export function readDroppedFile(path: string): Promise<DroppedFilePayload> {
  return invoke<DroppedFilePayload>("read_dropped_file", { path });
}

/** Dépose un fichier dans le workspace Gestion (bureau personal ou projet team). */
export function gestionUploadFile(args: {
  filename: string;
  mime?: string | null;
  dataBase64: string;
  visibility: "personal" | "team";
  projectId?: string | null;
}): Promise<GestionUploadReport> {
  return invoke<GestionUploadReport>("gestion_upload_file", {
    filename: args.filename,
    mime: args.mime ?? null,
    dataBase64: args.dataBase64,
    visibility: args.visibility,
    projectId: args.projectId ?? null,
  });
}

/** Upload depuis un chemin OS (drag-and-drop) sans re-passer le base64. */
export function gestionUploadFilePath(args: {
  path: string;
  visibility: "personal" | "team";
  projectId?: string | null;
}): Promise<GestionUploadReport> {
  return invoke<GestionUploadReport>("gestion_upload_file_path", {
    path: args.path,
    visibility: args.visibility,
    projectId: args.projectId ?? null,
  });
}

/** Message d’erreur Tauri (souvent une string, pas un Error). */
export function formatInvokeError(err: unknown, fallback: string): string {
  if (typeof err === "string" && err.trim()) return err.trim();
  if (err instanceof Error && err.message.trim()) return err.message.trim();
  if (err && typeof err === "object") {
    const o = err as { message?: unknown; error?: unknown };
    if (typeof o.message === "string" && o.message.trim()) return o.message.trim();
    if (typeof o.error === "string" && o.error.trim()) return o.error.trim();
  }
  try {
    const s = JSON.stringify(err);
    if (s && s !== "{}") return s;
  } catch {
    /* ignore */
  }
  return fallback;
}

export function isGestionLoggedIn(cfg: GestionConfig | null | undefined): boolean {
  return Boolean(cfg?.apiUrl?.trim() && cfg?.authToken?.trim());
}
