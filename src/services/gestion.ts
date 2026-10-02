/**
 * Shell Gestion — config API PHP distante.
 */

import { invoke } from "@tauri-apps/api/core";

export type GestionConfig = {
  apiUrl: string;
};

export function gestionShow(): Promise<void> {
  return invoke("gestion_show");
}

export function gestionGetConfig(): Promise<GestionConfig> {
  return invoke<GestionConfig>("gestion_get_config");
}

export function gestionSetConfig(apiUrl: string): Promise<GestionConfig> {
  return invoke<GestionConfig>("gestion_set_config", { apiUrl });
}
