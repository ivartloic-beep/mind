import { invoke } from "@tauri-apps/api/core";

function inTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

export async function captureShow(): Promise<void> {
  if (!inTauri()) return;
  await invoke("capture_show");
}

/** Ouvre la capture avec type pré-sélectionné. */
export async function captureShowKind(
  kind: "task" | "note" | "idea",
): Promise<void> {
  if (!inTauri()) return;
  await invoke("capture_show_with_kind", { kind });
}

export async function captureHide(): Promise<void> {
  if (!inTauri()) return;
  await invoke("capture_hide");
}
