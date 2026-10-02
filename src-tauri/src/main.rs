// Always hide console window on Windows (dev + release).
#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

fn main() {
    ma_tete_lib::run()
}
