import { invoke } from "@tauri-apps/api/core";

export type PanelState = {
  open: boolean;
  alwaysOnTop: boolean;
  contentWidth: number;
  handleWidth: number;
};

function inTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

export async function panelGetState(): Promise<PanelState> {
  if (!inTauri()) {
    return {
      open: true,
      alwaysOnTop: false,
      contentWidth: 400,
      handleWidth: 28,
    };
  }
  return invoke<PanelState>("panel_get_state");
}

export async function panelSetOpen(open: boolean): Promise<PanelState> {
  if (!inTauri()) {
    return {
      open,
      alwaysOnTop: false,
      contentWidth: 400,
      handleWidth: 28,
    };
  }
  return invoke<PanelState>("panel_set_open", { open });
}

export async function panelSetAlwaysOnTop(
  alwaysOnTop: boolean,
): Promise<PanelState> {
  if (!inTauri()) {
    return {
      open: true,
      alwaysOnTop,
      contentWidth: 400,
      handleWidth: 28,
    };
  }
  return invoke<PanelState>("panel_set_always_on_top", { alwaysOnTop });
}
