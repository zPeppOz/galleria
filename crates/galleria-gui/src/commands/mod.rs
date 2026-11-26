//! Tauri commands: thin wrappers over the modules that do not import Tauri. The list is declared in `build.rs`.

pub mod announce;
pub mod app;
pub mod connection;
pub mod daemon;
pub mod groups;
pub mod import;
pub mod logs;
pub mod profiles;
pub mod secrets;
pub mod tray;
pub mod ui_state;
pub mod window;
