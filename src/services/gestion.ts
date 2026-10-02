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

export function isGestionLoggedIn(cfg: GestionConfig | null | undefined): boolean {
  return Boolean(cfg?.apiUrl?.trim() && cfg?.authToken?.trim());
}
