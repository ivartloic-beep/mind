import { invoke } from "@tauri-apps/api/core";

function inTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

export async function captureShow(): Promise<void> {
  if (!inTauri()) return;
  await invoke("capture_show");
}

export async function captureHide(): Promise<void> {
  if (!inTauri()) return;
  await invoke("capture_hide");
}
