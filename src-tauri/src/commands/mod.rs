//! API invoke exposée au frontend — commands métier aux étapes 2+.

#[tauri::command]
pub fn ping() -> String {
    "pong".into()
}
