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

/** Range la Capture, ouvre le panneau sur Contact express (CRM). */
export async function captureOpenCrm(): Promise<void> {
  if (!inTauri()) return;
  await invoke("capture_open_crm");
}

export type ScreenCrmDraft = {
  name: string;
  organisme: string;
  phone: string;
  email: string;
};

export type AnalyzeScreenResult = ScreenCrmDraft & {
  foundText: boolean;
};

/** Screenshot + OCR Windows local → brouillon CRM (rien n’est créé). */
export async function analyzeScreenCrm(): Promise<AnalyzeScreenResult> {
  if (!inTauri()) {
    throw new Error("Analyse d’écran disponible uniquement dans l’app Windows.");
  }
  return invoke<AnalyzeScreenResult>("analyze_screen_crm");
}
