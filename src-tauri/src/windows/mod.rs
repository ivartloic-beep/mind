//! Création / focus / always-on-top / panel edge / capture / post-it.

pub mod capture;
pub mod library;
pub mod panel;
pub mod postit;

use tauri::{Window, WindowEvent};

/// Événements fenêtre partagés — hide library, delete post-it on native close.
pub fn handle_window_event(window: &Window, event: &WindowEvent) {
    match event {
        WindowEvent::CloseRequested { api, .. } => {
            let label = window.label();
            if label == library::LIBRARY_LABEL {
                api.prevent_close();
                let _ = window.hide();
            } else if label.starts_with(postit::LABEL_PREFIX) {
                postit::on_user_close_requested(window);
                // Ne pas prevent_close / destroy ici : fermeture native → pas de deadlock IPC.
            }
        }
        _ => {}
    }
}
