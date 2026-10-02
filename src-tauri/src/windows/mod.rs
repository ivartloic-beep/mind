//! Création / focus / always-on-top / panel edge / capture / post-it / gestion.

pub mod capture;
pub mod gestion;
pub mod library;
pub mod panel;
pub mod postit;

use tauri::{Manager, Window, WindowEvent};

/// Événements fenêtre partagés — hide library/gestion, delete post-it on native close.
pub fn handle_window_event(window: &Window, event: &WindowEvent) {
    match event {
        WindowEvent::CloseRequested { api, .. } => {
            let label = window.label();
            if label == library::LIBRARY_LABEL || label == gestion::GESTION_LABEL {
                api.prevent_close();
                let _ = window.hide();
                if label == gestion::GESTION_LABEL {
                    // Gestion → panneau : recharger tâches / notes / projets.
                    gestion::on_gestion_hidden(window.app_handle());
                }
            } else if label.starts_with(postit::LABEL_PREFIX) {
                postit::on_user_close_requested(window);
                // Ne pas prevent_close / destroy ici : fermeture native → pas de deadlock IPC.
            }
        }
        _ => {}
    }
}
