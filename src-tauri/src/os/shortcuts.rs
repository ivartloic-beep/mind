//! Global shortcuts — étape 11 + CTRL+ALT+P / CTRL+ALT+H post-it.
//!
//! Hook config : [`load_bindings`] / [`ShortcutBindings`] — prefs fichier plus tard.

use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_notification::NotificationExt;

use crate::windows::panel::{self, PANEL_LABEL};
use crate::windows::postit;

/// Bindings par défaut (configurables plus tard).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShortcutBindings {
    /// Ouvre la capture rapide.
    pub capture: &'static str,
    /// Ouvre / ferme le panneau.
    pub toggle_panel: &'static str,
    /// Crée un post-it scratch.
    pub scratch_postit: &'static str,
    /// Masque / réaffiche tous les post-its.
    pub toggle_postits: &'static str,
}

impl Default for ShortcutBindings {
    fn default() -> Self {
        Self {
            capture: "Ctrl+Alt+N",
            toggle_panel: "Ctrl+Alt+Space",
            scratch_postit: "Ctrl+Alt+P",
            toggle_postits: "Ctrl+Alt+H",
        }
    }
}

/// Charge les bindings — V1 = défauts ; hook pour prefs ultérieures.
pub fn load_bindings() -> ShortcutBindings {
    ShortcutBindings::default()
}

/// Enregistre les raccourcis globaux au boot. Log + notif si conflit / échec.
pub fn register_shortcuts(app: &AppHandle) {
    let bindings = load_bindings();
    if let Err(err) = register_with_bindings(app, &bindings) {
        eprintln!("shortcuts register: {err}");
        notify_shortcut_failure(app, &err);
    }
}

fn notify_shortcut_failure(app: &AppHandle, err: &str) {
    let _ = app
        .notification()
        .builder()
        .title("Ma Tête — Raccourcis")
        .body(format!("Impossible d’enregistrer un raccourci : {err}"))
        .show();
}

fn register_with_bindings(app: &AppHandle, bindings: &ShortcutBindings) -> Result<(), String> {
    use tauri_plugin_global_shortcut::{
        Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState,
    };

    let capture = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyN);
    let toggle = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::Space);
    let postit_key = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyP);
    let hide_postits = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyH);

    let capture_sc = capture;
    let toggle_sc = toggle;
    let postit_sc = postit_key;
    let hide_sc = hide_postits;

    app.plugin(
        tauri_plugin_global_shortcut::Builder::new()
            .with_handler(move |app, shortcut, event| {
                if event.state() != ShortcutState::Pressed {
                    return;
                }
                if shortcut == &capture_sc {
                    if let Err(err) = crate::windows::capture::capture_show(app.clone()) {
                        eprintln!("shortcut capture: {err}");
                    }
                } else if shortcut == &toggle_sc {
                    toggle_panel(app);
                } else if shortcut == &postit_sc {
                    if let Err(err) = postit::create_scratch_postit(app.clone()) {
                        eprintln!("shortcut postit: {err}");
                    }
                } else if shortcut == &hide_sc {
                    postit::toggle_postits_visibility(app);
                }
            })
            .build(),
    )
    .map_err(|e| e.to_string())?;

    let mut failures: Vec<String> = Vec::new();

    if let Err(err) = app.global_shortcut().register(capture) {
        failures.push(format!("{} ({})", bindings.capture, err));
    }
    if let Err(err) = app.global_shortcut().register(toggle) {
        failures.push(format!("{} ({})", bindings.toggle_panel, err));
    }
    if let Err(err) = app.global_shortcut().register(postit_key) {
        failures.push(format!("{} ({})", bindings.scratch_postit, err));
    }
    if let Err(err) = app.global_shortcut().register(hide_postits) {
        failures.push(format!("{} ({})", bindings.toggle_postits, err));
    }

    if failures.is_empty() {
        eprintln!(
            "shortcuts ok: {} → capture, {} → panneau, {} → post-it, {} → masquer post-its",
            bindings.capture,
            bindings.toggle_panel,
            bindings.scratch_postit,
            bindings.toggle_postits
        );
        Ok(())
    } else {
        Err(failures.join(" ; "))
    }
}

fn toggle_panel(app: &AppHandle) {
    let prefs = panel::load_prefs(app);
    let next = !prefs.open;
    match panel::panel_set_open(app.clone(), next) {
        Ok(_) => {
            if let Some(window) = app.get_webview_window(PANEL_LABEL) {
                let _ = window.show();
                if next {
                    let _ = window.set_focus();
                }
            }
            let _ = app.emit("shortcut-panel-toggled", next);
        }
        Err(err) => eprintln!("shortcut panel toggle: {err}"),
    }
}
