use std::collections::HashSet;
use std::sync::Mutex;

use crate::storage::local::LocalStorage;

pub struct AppState {
    pub storage: LocalStorage,
    /// Rappels déjà notifiés (en attente snooze/dismiss).
    pub fired_reminders: Mutex<HashSet<String>>,
}

impl AppState {
    pub fn new(storage: LocalStorage) -> Self {
        Self {
            storage,
            fired_reminders: Mutex::new(HashSet::new()),
        }
    }
}
