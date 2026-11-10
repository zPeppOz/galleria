//! Spike 1 (throwaway): the smallest Tauri app that can test the MSI + service + updater chain.
//! No window: at startup it asks the service for its version over the named pipe (`hello`, D12),
//! then checks the update endpoint and installs what it finds. Everything is logged to a file.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::io::{BufRead, BufReader, Write};
use tauri_plugin_updater::UpdaterExt;

fn log(msg: &str) {
    let dir = std::env::var("GALLERIA_SPIKE_LOGDIR").unwrap_or_else(|_| r"C:\ProgramData\galleria-spike".into());
    let _ = std::fs::create_dir_all(&dir);
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(format!(r"{dir}\app.log")) {
        let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
        let _ = writeln!(f, "{t} app[{}] {msg}", std::process::id());
    }
}

fn service_hello() -> String {
    let pipe = std::fs::OpenOptions::new().read(true).write(true).open(r"\\.\pipe\galleria-spike");
    match pipe {
        Ok(mut p) => {
            if p.write_all(b"{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"hello\"}\n").is_err() {
                return "write failed".into();
            }
            let mut line = String::new();
            let _ = BufReader::new(p).read_line(&mut line);
            line.trim().to_string()
        }
        Err(e) => format!("pipe error: {e}"),
    }
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let h = app.handle().clone();
            log(&format!("app version {} ; service hello -> {}", app.package_info().version, service_hello()));
            tauri::async_runtime::spawn(async move {
                match h.updater() {
                    Ok(u) => match u.check().await {
                        Ok(Some(update)) => {
                            log(&format!("update found: {} -> {}", update.current_version, update.version));
                            match update.download_and_install(|_, _| {}, || log("update downloaded, signature ok")).await {
                                Ok(()) => log("update installer launched"),
                                Err(e) => log(&format!("update REJECTED: {e}")),
                            }
                        }
                        Ok(None) => log("no update available"),
                        Err(e) => log(&format!("update check failed: {e}")),
                    },
                    Err(e) => log(&format!("updater not available: {e}")),
                }
                h.exit(0);
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("tauri run");
}
