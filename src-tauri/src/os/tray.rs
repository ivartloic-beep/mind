//! Tray icon + menu — étape 12.

use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, RunEvent,
};

use crate::domain::{new_id, now_iso, DataChangedPayload, Note, NoteKind};
use crate::state::AppState;
use crate::storage::Storage;
use crate::windows::{capture, panel, postit};
use crate::windows::panel::PANEL_LABEL;

const TRAY_ID: &str = "ma-tete-tray";

/// Construit le tray au démarrage.
pub fn init_tray(app: &AppHandle) -> Result<(), String> {
    let open_panel = MenuItem::with_id(app, "open_panel", "Ouvrir le panneau", true, None::<&str>)
        .map_err(|e| e.to_string())?;
    let new_task = MenuItem::with_id(app, "new_task", "Nouvelle tâche", true, None::<&str>)
        .map_err(|e| e.to_string())?;
    let new_note = MenuItem::with_id(app, "new_note", "Nouvelle note", true, None::<&str>)
        .map_err(|e| e.to_string())?;
    let new_postit = MenuItem::with_id(app, "new_postit", "Nouveau post-it", true, None::<&str>)
        .map_err(|e| e.to_string())?;
    let timer_start =
        MenuItem::with_id(app, "timer_start", "Démarrer le minuteur", true, None::<&str>)
            .map_err(|e| e.to_string())?;
    let timer_pause =
        MenuItem::with_id(app, "timer_pause", "Pause", true, None::<&str>).map_err(|e| e.to_string())?;
    let settings =
        MenuItem::with_id(app, "settings", "Paramètres", true, None::<&str>).map_err(|e| e.to_string())?;
    let quit =
        MenuItem::with_id(app, "quit", "Quitter", true, None::<&str>).map_err(|e| e.to_string())?;
    let sep1 = PredefinedMenuItem::separator(app).map_err(|e| e.to_string())?;
    let sep2 = PredefinedMenuItem::separator(app).map_err(|e| e.to_string())?;
    let sep3 = PredefinedMenuItem::separator(app).map_err(|e| e.to_string())?;

    let menu = Menu::with_items(
        app,
        &[
            &open_panel,
            &sep1,
            &new_task,
            &new_note,
            &new_postit,
            &sep2,
            &timer_start,
            &timer_pause,
            &sep3,
            &settings,
            &quit,
        ],
    )
    .map_err(|e| e.to_string())?;

    let icon = app
        .default_window_icon()
        .cloned()
        .ok_or_else(|| "icône fenêtre par défaut introuvable".to_string())?;

    TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon)
        .tooltip("Ma Tête")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "open_panel" => show_panel(app, true),
            "new_task" => open_capture(app, "task"),
            "new_note" => open_capture(app, "note"),
            "new_postit" => {
                if let Err(err) = open_new_postit(app) {
                    eprintln!("tray new_postit: {err}");
                }
            }
            "timer_start" => {
                show_panel(app, true);
                let _ = app.emit("timer-command", "start");
            }
            "timer_pause" => {
                let _ = app.emit("timer-command", "pause");
            }
            "settings" => {
                show_panel(app, true);
                let _ = app.emit("panel-focus-settings", ());
            }
            "quit" => {
                app.exit(0);
            }
            other => eprintln!("tray menu unhandled: {other}"),
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_panel(tray.app_handle(), true);
            }
        })
        .build(app)
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Au second lancement (single-instance) : focus panneau.
pub fn focus_existing_instance(app: &AppHandle) {
    show_panel(app, true);
}

/// Garde le process vivant pour le tray quand une fenêtre se ferme.
pub fn handle_run_event(_app: &AppHandle, event: &RunEvent) {
    if let RunEvent::ExitRequested { api, code, .. } = event {
        if code.is_none() {
            api.prevent_exit();
        }
    }
}

fn show_panel(app: &AppHandle, open: bool) {
    match panel::panel_set_open(app.clone(), open) {
        Ok(_) => {
            if let Some(window) = app.get_webview_window(PANEL_LABEL) {
                let _ = window.show();
                if open {
                    let _ = window.unminimize();
                    let _ = window.set_focus();
                }
            }
        }
        Err(err) => eprintln!("tray show_panel: {err}"),
    }
}

fn open_capture(app: &AppHandle, kind: &str) {
    if let Err(err) = capture::capture_show_kind(app.clone(), Some(kind.to_string())) {
        eprintln!("tray capture ({kind}): {err}");
    }
}

fn open_new_postit(app: &AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();
    let now = now_iso();
    let note_id = new_id();
    let note = Note {
        id: note_id.clone(),
        title: Some("Post-it".into()),
        body: String::new(),
        kind: NoteKind::Note,
        project_id: None,
        created_at: now.clone(),
        updated_at: now,
    };
    state
        .storage
        .upsert_note(&note)
        .map_err(|e| e.to_string())?;
    let _ = app.emit(
        "data-changed",
        DataChangedPayload {
            entity: "note".into(),
            id: note_id.clone(),
        },
    );
    postit::open_for_note_id(app, &note_id)?;
    Ok(())
}
