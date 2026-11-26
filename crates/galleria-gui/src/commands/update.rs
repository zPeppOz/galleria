//! The command that tells the page what the version check knows (plan B6, T4, P1). It reads `state.toml`: it never makes a request, and
//! nothing the page sends can start one (the request is Rust's, on its own clock).

use crate::update::{UpdateView, Updater};
use std::sync::Arc;
use tauri::State;

/// `{ current, latest, url, enabled }`: `latest` and `url` are there only for a version newer than this one that the person has not
/// dismissed, and `url` was built by Rust from the number.
#[tauri::command]
pub fn update_view(updater: State<'_, Arc<Updater>>) -> UpdateView {
    updater.view()
}
