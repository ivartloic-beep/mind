/**
 * État UI éphémère (filtre, thème, panel open).
 * Always-on-top panel est mémorisé côté Rust (panel-prefs.json).
 */

export type ThemePreference = "system" | "light" | "dark";

export type UiState = {
  panelOpen: boolean;
  panelAlwaysOnTop: boolean;
  theme: ThemePreference;
  activeFilter: "all" | "no-project" | string;
};

const state: UiState = {
  panelOpen: true,
  panelAlwaysOnTop: false,
  theme: "system",
  activeFilter: "all",
};

type Listener = () => void;
const listeners = new Set<Listener>();

function notify(): void {
  for (const listener of listeners) listener();
}

export function getUiState(): UiState {
  return { ...state };
}

export function subscribeUi(listener: Listener): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

export function setTheme(theme: ThemePreference): void {
  state.theme = theme;
  notify();
}

export function setPanelOpen(open: boolean): void {
  state.panelOpen = open;
  notify();
}

export function setPanelAlwaysOnTop(alwaysOnTop: boolean): void {
  state.panelAlwaysOnTop = alwaysOnTop;
  notify();
}
