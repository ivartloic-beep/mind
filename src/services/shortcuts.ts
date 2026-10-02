/**
 * Raccourcis clavier globaux — liste pour l’UI Paramètres.
 */

import { invoke } from "@tauri-apps/api/core";

export type ShortcutInfo = {
  id: string;
  label: string;
  keys: string;
};

export function listShortcuts(): Promise<ShortcutInfo[]> {
  return invoke<ShortcutInfo[]>("list_shortcuts");
}
