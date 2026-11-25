//! The size of the window (Q34): saved when it closes and half a second after the last resize, applied before the
//! window is shown. The position is not saved: a Wayland client cannot choose it.

use crate::ui_state::{UiStateFile, WindowSize};
use std::{
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};
use tauri::{LogicalSize, Runtime, Window, WindowEvent};

/// How long the size must stay the same before it is written.
pub const SAVE_DELAY: Duration = Duration::from_millis(500);

pub struct WindowSaver {
    file: Arc<UiStateFile>,
    /// Each resize takes a number; a pending save only writes if no later resize took the next one.
    ticket: AtomicU64,
}

impl WindowSaver {
    pub fn new(file: Arc<UiStateFile>) -> Self {
        Self {
            file,
            ticket: AtomicU64::new(0),
        }
    }

    /// Writes the size of `window` as it is now. A window that is minimised or has no size yet is left alone.
    pub fn save_now<R: Runtime>(&self, window: &Window<R>) {
        let (Ok(physical), Ok(scale), Ok(maximized)) = (
            window.inner_size(),
            window.scale_factor(),
            window.is_maximized(),
        ) else {
            return;
        };
        if window.is_minimized().unwrap_or(false) {
            return;
        }
        let logical = physical.to_logical::<f64>(scale);
        // Rounded and clamped to a size a window can have, then to what `usable` accepts.
        let side = |v: f64| v.round().clamp(0.0, 1_000_000.0) as u32;
        let (width, height) = (side(logical.width), side(logical.height));
        let previous = self.file.load().window;
        let now = WindowSize::after(previous, width, height, maximized);
        if now.usable().is_none() || Some(now) == previous {
            return;
        }
        if let Err(e) = self.file.update(|state| state.window = Some(now)) {
            eprintln!("galleria: {e}");
        }
    }

    pub fn save_later<R: Runtime>(self: &Arc<Self>, window: &Window<R>) {
        let mine = self.ticket.fetch_add(1, Ordering::SeqCst) + 1;
        let (saver, window) = (Arc::clone(self), window.clone());
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(SAVE_DELAY).await;
            if saver.ticket.load(Ordering::SeqCst) == mine {
                saver.save_now(&window);
            }
        });
    }
}

/// Window events of the main window: a resize starts the timer, closing writes at once.
pub fn on_event<R: Runtime>(saver: &Arc<WindowSaver>, window: &Window<R>, event: &WindowEvent) {
    if window.label() != "main" {
        return;
    }
    match event {
        WindowEvent::Resized(_) => saver.save_later(window),
        WindowEvent::CloseRequested { .. } | WindowEvent::Destroyed => saver.save_now(window),
        _ => {}
    }
}

/// Applies the saved size and shows the window: `tauri.conf.json` creates it hidden, so it opens once, at its size.
pub fn restore<R: Runtime>(window: &tauri::WebviewWindow<R>, file: &UiStateFile) {
    if let Some(size) = file.load().window.and_then(WindowSize::usable) {
        let _ = window.set_size(LogicalSize::new(
            f64::from(size.width),
            f64::from(size.height),
        ));
        if size.maximized {
            let _ = window.maximize();
        }
    }
    let _ = window.show();
}
