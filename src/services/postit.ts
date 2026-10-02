import { invoke } from "@tauri-apps/api/core";
import type { PostIt } from "../types/models";

function inTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

export async function createScratchPostit(): Promise<PostIt | null> {
  if (!inTauri()) return null;
  return invoke<PostIt>("create_scratch_postit");
}

export async function postitOpenForNote(noteId: string): Promise<PostIt | null> {
  if (!inTauri()) return null;
  return invoke<PostIt>("postit_open_for_note", { noteId });
}

/** Ferme et supprime le post-it. */
export async function postitClose(id: string): Promise<void> {
  if (!inTauri()) return;
  await invoke("postit_close", { id });
}

export async function postitUpdateGeometry(
  id: string,
  x: number,
  y: number,
  w: number,
  h: number,
): Promise<PostIt> {
  return invoke<PostIt>("postit_update_geometry", { id, x, y, w, h });
}

export async function postitSetAlwaysOnTop(
  id: string,
  alwaysOnTop: boolean,
): Promise<PostIt> {
  return invoke<PostIt>("postit_set_always_on_top", { id, alwaysOnTop });
}
