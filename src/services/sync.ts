import { invoke } from "@tauri-apps/api/core";

export type SyncConfigView = {
  enabled: boolean;
  baseUrl: string;
  hasToken: boolean;
  token: string;
  lastSyncAt?: string | null;
  lastError?: string | null;
};

export type SyncReport = {
  pulled: number;
  pushed: number;
  deletedRemote: number;
  lastSyncAt: string;
};

export type SyncConfigInput = {
  enabled: boolean;
  baseUrl: string;
  token: string;
};

function inTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

export async function syncGetConfig(): Promise<SyncConfigView | null> {
  if (!inTauri()) return null;
  return invoke<SyncConfigView>("sync_get_config");
}

export async function syncSetConfig(
  input: SyncConfigInput,
): Promise<SyncConfigView> {
  return invoke<SyncConfigView>("sync_set_config", { input });
}

export async function syncTest(): Promise<string> {
  return invoke<string>("sync_test");
}

export async function syncNow(): Promise<SyncReport> {
  return invoke<SyncReport>("sync_now");
}
