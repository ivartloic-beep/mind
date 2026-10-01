/**
 * État UI éphémère seulement (filtre, thème, panel open).
 * La vérité métier vit côté Rust — pas de store global métier ici.
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

export function getUiState(): UiState {
  return { ...state };
}

export function setTheme(theme: ThemePreference): void {
  state.theme = theme;
}
