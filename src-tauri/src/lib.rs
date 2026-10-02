mod commands;
mod domain;
mod gestion;
mod os;
mod state;
mod storage;
mod sync;
mod windows;

use std::path::PathBuf;

use tauri::Manager;

use state::AppState;
use storage::local::LocalStorage;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut builder = tauri::Builder::default();

    // Single-instance en premier (avant les autres plugins).
    #[cfg(any(windows, target_os = "macos", target_os = "linux"))]
    {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            os::tray::focus_existing_instance(app);
        }));
    }

    #[cfg(any(windows, target_os = "macos", target_os = "linux"))]
    {
        builder = builder.plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--autostart"]),
        ));
    }

    builder = builder.plugin(tauri_plugin_notification::init());

    let app = builder
        .setup(|app| {
            let db_path = resolve_db_path(app.handle())?;
            let storage = LocalStorage::open(&db_path).map_err(|e| e.to_string())?;
            app.manage(AppState::new(storage));

            let autostart_launch = std::env::args().any(|a| a == "--autostart");

            if let Err(err) = windows::panel::init_panel(app.handle(), autostart_launch) {
                eprintln!("panel init: {err}");
            }
            if let Err(err) = windows::capture::init_capture(app.handle()) {
                eprintln!("capture init: {err}");
            }
            if let Err(err) = windows::postit::restore_open_postits(app.handle()) {
                eprintln!("postit restore: {err}");
            }
            // Shell Gestion (front embarqué) — ouvert sauf autostart discret.
            if let Err(err) = windows::gestion::init_gestion(app.handle(), autostart_launch) {
                eprintln!("gestion init: {err}");
            }

            os::notifications::start_reminder_scheduler(app.handle().clone());
            sync::start_sync_scheduler(app.handle().clone());
            os::shortcuts::register_shortcuts(app.handle());

            #[cfg(any(windows, target_os = "macos", target_os = "linux"))]
            {
                os::autostart::init_autostart(app.handle());
            }

            if let Err(err) = os::tray::init_tray(app.handle()) {
                eprintln!("tray init: {err}");
            }

            // Main reste cachée (host tray / shortcuts).
            if let Some(main) = app.get_webview_window("main") {
                let _ = main.hide();
            }

            Ok(())
        })
        .on_window_event(|window, event| {
            windows::handle_window_event(window, event);
        })
        .invoke_handler(tauri::generate_handler![
            commands::ping,
            commands::list_projects,
            commands::get_project,
            commands::upsert_project,
            commands::delete_project,
            commands::list_tasks,
            commands::get_task,
            commands::upsert_task,
            commands::delete_task,
            commands::create_task,
            commands::set_task_done,
            commands::list_notes,
            commands::get_note,
            commands::upsert_note,
            commands::delete_note,
            commands::create_note,
            commands::list_reminders,
            commands::get_reminder,
            commands::upsert_reminder,
            commands::delete_reminder,
            commands::list_postits,
            commands::get_postit,
            commands::upsert_postit,
            commands::delete_postit,
            windows::panel::panel_get_state,
            windows::panel::panel_set_open,
            windows::panel::panel_set_always_on_top,
            windows::panel::panel_redock,
            windows::capture::capture_show,
            windows::capture::capture_hide,
            windows::library::library_show,
            windows::gestion::gestion_show,
            windows::gestion::gestion_show_task,
            windows::gestion::gestion_get_config,
            windows::gestion::gestion_set_config,
            windows::gestion::gestion_login,
            windows::gestion::gestion_logout,
            windows::gestion::gestion_set_session,
            windows::gestion::gestion_migrate_local_tasks,
            windows::gestion::gestion_tasks_backend_active,
            windows::gestion::gestion_sync_now,
            windows::gestion::gestion_upload_file,
            windows::postit::create_scratch_postit,
            windows::postit::postit_open_for_note,
            windows::postit::postit_close,
            windows::postit::postit_update_geometry,
            windows::postit::postit_set_always_on_top,
            os::notifications::set_task_reminder,
            os::notifications::clear_task_reminder,
            os::notifications::snooze_reminder,
            os::notifications::dismiss_reminder,
            os::notifications::open_task_from_reminder,
            os::autostart::autostart_set_enabled,
            os::autostart::autostart_is_enabled,
            os::shortcuts::list_shortcuts,
            sync::sync_get_config,
            sync::sync_set_config,
            sync::sync_test,
            sync::sync_now,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(|app_handle, event| {
        os::tray::handle_run_event(app_handle, &event);
    });
}

fn resolve_db_path(app: &tauri::AppHandle) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let dir = app.path().app_data_dir()?;
    std::fs::create_dir_all(&dir)?;
    Ok(dir.join("ma-tete.db"))
}
