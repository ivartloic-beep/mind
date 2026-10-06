//! Tray, autostart, global shortcuts, notifications — étapes 9–12.

pub mod autostart;
pub mod notifications;
pub mod screen_analyze;
#[cfg_attr(not(windows), allow(dead_code))]
pub mod screen_crm;
pub mod shortcuts;
pub mod tray;
