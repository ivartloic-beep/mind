import { invoke } from "@tauri-apps/api/core";

function inTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

export async function libraryShow(): Promise<void> {
  if (!inTauri()) return;
  await invoke("library_show");
}
