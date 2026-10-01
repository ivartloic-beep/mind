mod commands;
mod domain;
mod os;
mod state;
mod storage;
mod windows;

use std::path::PathBuf;

use tauri::Manager;

use state::AppState;
use storage::local::LocalStorage;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            let db_path = resolve_db_path(app.handle())?;
            let storage = LocalStorage::open(&db_path).map_err(|e| e.to_string())?;
            app.manage(AppState::new(storage));
            if let Err(err) = windows::panel::init_panel(app.handle()) {
                eprintln!("panel init: {err}");
            }
            if let Err(err) = windows::capture::init_capture(app.handle()) {
                eprintln!("capture init: {err}");
            }
            if let Err(err) = windows::postit::restore_open_postits(app.handle()) {
                eprintln!("postit restore: {err}");
            }
            os::notifications::start_reminder_scheduler(app.handle().clone());
            os::shortcuts::register_shortcuts(app.handle());
            Ok(())
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
            windows::postit::postit_open_for_note,
            windows::postit::postit_close,
            windows::postit::postit_update_geometry,
            windows::postit::postit_set_always_on_top,
            os::notifications::set_task_reminder,
            os::notifications::clear_task_reminder,
            os::notifications::snooze_reminder,
            os::notifications::dismiss_reminder,
            os::notifications::open_task_from_reminder,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn resolve_db_path(app: &tauri::AppHandle) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let dir = app.path().app_data_dir()?;
    std::fs::create_dir_all(&dir)?;
    Ok(dir.join("ma-tete.db"))
}
