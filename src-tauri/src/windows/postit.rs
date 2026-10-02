//! Fenêtres post-it natives `postit-{id}` — pensées immédiates autonomes.

use std::sync::atomic::{AtomicBool, Ordering};

use tauri::{
    AppHandle, Emitter, LogicalPosition, LogicalSize, Manager, State, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder, Window,
};

use crate::domain::{new_id, DataChangedPayload, PostIt, PostItFilter};
use crate::state::AppState;
use crate::storage::Storage;

pub const LABEL_PREFIX: &str = "postit-";
const DEFAULT_W: f64 = 280.0;
const DEFAULT_H: f64 = 240.0;
const MAX_OPEN: usize = 20;

/// Évite de supprimer les post-its quand l’app quitte (Destroy/Close de toutes les fenêtres).
static APP_EXITING: AtomicBool = AtomicBool::new(false);

pub fn mark_app_exiting() {
    APP_EXITING.store(true, Ordering::SeqCst);
}

fn id_from_label(label: &str) -> Option<&str> {
    label.strip_prefix(LABEL_PREFIX)
}

fn label_for(id: &str) -> String {
    format!("{LABEL_PREFIX}{id}")
}

fn open_count(app: &AppHandle) -> usize {
    app.webview_windows()
        .keys()
        .filter(|l| l.starts_with(LABEL_PREFIX))
        .count()
}

fn cascade_offset(app: &AppHandle) -> f64 {
    (open_count(app) as f64) * 28.0
}

fn ensure_window(app: &AppHandle, postit: &PostIt) -> Result<WebviewWindow, String> {
    let label = label_for(&postit.id);
    if let Some(existing) = app.get_webview_window(&label) {
        existing
            .set_always_on_top(postit.always_on_top)
            .map_err(|e| e.to_string())?;
        existing
            .set_position(LogicalPosition::new(postit.x, postit.y))
            .map_err(|e| e.to_string())?;
        existing
            .set_size(LogicalSize::new(postit.w.max(200.0), postit.h.max(160.0)))
            .map_err(|e| e.to_string())?;
        let _ = existing.unminimize();
        existing.show().map_err(|e| e.to_string())?;
        existing.set_focus().map_err(|e| e.to_string())?;
        return Ok(existing);
    }

    if open_count(app) >= MAX_OPEN {
        return Err(format!("limite de {MAX_OPEN} post-its ouverts"));
    }

    let url = WebviewUrl::App(format!("postit.html?postitId={}", postit.id).into());
    let window = WebviewWindowBuilder::new(app, &label, url)
        .title("MIND — Post-it")
        .inner_size(postit.w.max(200.0), postit.h.max(160.0))
        .position(postit.x, postit.y)
        .resizable(true)
        .minimizable(true)
        .maximizable(false)
        .closable(true)
        .decorations(true)
        .always_on_top(postit.always_on_top)
        .skip_taskbar(false)
        .visible(true)
        .build()
        .map_err(|e| e.to_string())?;
    window.set_focus().map_err(|e| e.to_string())?;
    Ok(window)
}

/// Restaure les post-its `open=true` après redémarrage.
pub fn restore_open_postits(app: &AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();
    let open = state
        .storage
        .list_postits(&PostItFilter {
            open: Some(true),
            ..Default::default()
        })
        .map_err(|e| e.to_string())?;
    for postit in open.into_iter().take(MAX_OPEN) {
        if let Err(err) = ensure_window(app, &postit) {
            eprintln!("postit restore {}: {err}", postit.id);
        }
    }
    Ok(())
}

fn find_for_note(state: &AppState, note_id: &str) -> Result<Option<PostIt>, String> {
    let rows = state
        .storage
        .list_postits(&PostItFilter {
            note_id: Some(note_id.to_string()),
            ..Default::default()
        })
        .map_err(|e| e.to_string())?;
    Ok(rows.into_iter().next())
}

/// Crée un post-it scratch vide, toujours au-dessus, et l’ouvre.
#[tauri::command]
pub fn create_scratch_postit(app: AppHandle) -> Result<PostIt, String> {
    let state = app.state::<AppState>();
    let offset = cascade_offset(&app);
    let postit = PostIt {
        id: new_id(),
        note_id: None,
        title: None,
        body: String::new(),
        x: 80.0 + offset,
        y: 80.0 + offset,
        w: DEFAULT_W,
        h: DEFAULT_H,
        always_on_top: true,
        open: true,
    };
    state
        .storage
        .upsert_postit(&postit)
        .map_err(|e| e.to_string())?;
    ensure_window(&app, &postit)?;
    let _ = app.emit(
        "data-changed",
        DataChangedPayload {
            entity: "postit".into(),
            id: postit.id.clone(),
        },
    );
    Ok(postit)
}

/// Legacy : ouvre (ou crée) un post-it pour une Note.
#[tauri::command]
pub fn postit_open_for_note(app: AppHandle, note_id: String) -> Result<PostIt, String> {
    open_for_note_id(&app, &note_id)
}

pub fn open_for_note_id(app: &AppHandle, note_id: &str) -> Result<PostIt, String> {
    let state = app.state::<AppState>();
    let note = state
        .storage
        .get_note(note_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("note introuvable: {note_id}"))?;

    let offset = cascade_offset(app);
    let mut postit = match find_for_note(&state, note_id)? {
        Some(existing) => existing,
        None => PostIt {
            id: new_id(),
            note_id: Some(note_id.to_string()),
            title: note.title.clone(),
            body: note.body.clone(),
            x: 80.0 + offset,
            y: 80.0 + offset,
            w: DEFAULT_W,
            h: DEFAULT_H,
            always_on_top: true,
            open: true,
        },
    };
    postit.open = true;
    if postit.body.is_empty() {
        postit.body = note.body;
    }
    if postit.w < 200.0 {
        postit.w = DEFAULT_W;
    }
    if postit.h < 160.0 {
        postit.h = DEFAULT_H;
    }
    state
        .storage
        .upsert_postit(&postit)
        .map_err(|e| e.to_string())?;
    ensure_window(app, &postit)?;
    let _ = app.emit(
        "data-changed",
        DataChangedPayload {
            entity: "postit".into(),
            id: postit.id.clone(),
        },
    );
    Ok(postit)
}

/// Croix utilisateur : supprimer l’entité (la fenêtre se ferme nativement).
pub fn on_user_close_requested(window: &Window) {
    if APP_EXITING.load(Ordering::SeqCst) {
        return;
    }
    let Some(id) = id_from_label(window.label()) else {
        return;
    };
    if let Err(err) = delete_postit_entity(&window.app_handle(), id) {
        eprintln!("postit close cleanup {id}: {err}");
    }
}

/// Supprime le post-it en base + note orpheline legacy. N’ouvre / ne détruit aucune fenêtre.
fn delete_postit_entity(app: &AppHandle, id: &str) -> Result<(), String> {
    let state = app.state::<AppState>();
    let postit = match state.storage.get_postit(id).map_err(|e| e.to_string())? {
        Some(p) => p,
        None => return Ok(()),
    };

    let legacy_note = postit.note_id.clone();
    state
        .storage
        .delete_postit(id)
        .map_err(|e| e.to_string())?;
    let _ = app.emit(
        "data-changed",
        DataChangedPayload {
            entity: "postit".into(),
            id: id.to_string(),
        },
    );

    // Nettoie une note orpheline créée uniquement pour un ancien post-it tray.
    if let Some(note_id) = legacy_note {
        let still_linked = state
            .storage
            .list_postits(&PostItFilter {
                note_id: Some(note_id.clone()),
                ..Default::default()
            })
            .map_err(|e| e.to_string())?;
        if still_linked.is_empty() {
            if let Ok(Some(note)) = state.storage.get_note(&note_id) {
                if note.title.as_deref() == Some("Post-it") && note.body.is_empty() {
                    let _ = state.storage.delete_note(&note_id);
                    let _ = app.emit(
                        "data-changed",
                        DataChangedPayload {
                            entity: "note".into(),
                            id: note_id,
                        },
                    );
                }
            }
        }
    }
    Ok(())
}

/// Fermeture programmatique : `close()` natif (cleanup via CloseRequested).
#[tauri::command]
pub fn postit_close(app: AppHandle, id: String) -> Result<(), String> {
    if let Some(window) = app.get_webview_window(&label_for(&id)) {
        window.close().map_err(|e| e.to_string())?;
    } else {
        delete_postit_entity(&app, &id)?;
    }
    Ok(())
}

/// CTRL+ALT+H — masque tous les post-its visibles, ou les réaffiche tous.
pub fn toggle_postits_visibility(app: &AppHandle) {
    let postits: Vec<_> = app
        .webview_windows()
        .into_iter()
        .filter(|(label, _)| label.starts_with(LABEL_PREFIX))
        .map(|(_, w)| w)
        .collect();
    if postits.is_empty() {
        return;
    }
    let any_visible = postits.iter().any(|w| w.is_visible().unwrap_or(false));
    for window in postits {
        if any_visible {
            let _ = window.hide();
        } else {
            let _ = window.unminimize();
            let _ = window.show();
        }
    }
}

#[tauri::command]
pub fn postit_update_geometry(
    state: State<'_, AppState>,
    id: String,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
) -> Result<PostIt, String> {
    let mut postit = state
        .storage
        .get_postit(&id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("postit introuvable: {id}"))?;
    postit.x = x;
    postit.y = y;
    postit.w = w.max(160.0);
    postit.h = h.max(120.0);
    state
        .storage
        .upsert_postit(&postit)
        .map_err(|e| e.to_string())?;
    Ok(postit)
}

#[tauri::command]
pub fn postit_set_always_on_top(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    always_on_top: bool,
) -> Result<PostIt, String> {
    let mut postit = state
        .storage
        .get_postit(&id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("postit introuvable: {id}"))?;
    postit.always_on_top = always_on_top;
    state
        .storage
        .upsert_postit(&postit)
        .map_err(|e| e.to_string())?;
    if let Some(window) = app.get_webview_window(&label_for(&id)) {
        window
            .set_always_on_top(always_on_top)
            .map_err(|e| e.to_string())?;
    }
    Ok(postit)
}
