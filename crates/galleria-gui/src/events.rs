//! Forwarding of the daemon events to the webview (spec §4.3).

use crate::{
    announce::Announcer,
    import::{Imports, Origin},
    link::DaemonView,
    rubrica::Rubrica,
    session::Sessions,
    supervisor::EventSink,
    tray::Tray,
    tray_tauri::favourites_of,
};
use galleria_ipc::{LogEntry, Stats, Status};
use std::{path::PathBuf, sync::Arc};
use tauri::{AppHandle, DragDropEvent, Emitter, Manager, Runtime, Window, WindowEvent};

pub const EV_LINK: &str = "galleria:link";
pub const EV_STATE: &str = "galleria:state";
pub const EV_STATS: &str = "galleria:stats";
pub const EV_LOG: &str = "galleria:log";
/// A file came from outside (dropped on the window, or given to the app): the payload is an `ImportOffer`, with no path.
pub const EV_OPEN_FILE: &str = "galleria:open-file";
/// A file is over the window or has left it: `{ active }`, no paths. Only for drawing (P9).
pub const EV_DROP_HOVER: &str = "galleria:drop-hover";

/// Emits one Tauri event per call. Emitting queues the payload for the webview and returns: it does not wait for
/// the page, so the thread that delivers events is never held up by it.
pub struct TauriSink<R: Runtime>(pub AppHandle<R>);

impl<R: Runtime> TauriSink<R> {
    fn emit<S: serde::Serialize + Clone>(&self, event: &str, payload: &S) {
        if let Err(e) = self.0.emit(event, payload) {
            eprintln!("galleria-gui: cannot emit {event}: {e}");
        }
    }
}

impl<R: Runtime> EventSink for TauriSink<R> {
    fn link(&self, view: &DaemonView) {
        // The menu of the tray follows the link (nothing can be connected without it) and reads the favourites again: a profile
        // that the CLI marked since is in the menu from the next link on.
        if let Some(tray) = self.0.try_state::<Arc<Tray>>() {
            if let Some(rubrica) = self.0.try_state::<Arc<Rubrica>>() {
                tray.set_favourites(favourites_of(&rubrica));
            }
            tray.set_link(view);
        }
        self.emit(EV_LINK, view);
    }
    fn state(&self, status: &Status) {
        // What follows the state is Rust's own (the last use of the profile, the sentence for the screen reader) and comes
        // before the page hears of it, so a page that reads the rubrica on `connected` already sees the last use.
        if let Some(sessions) = self.0.try_state::<Arc<Sessions>>()
            && let Some((text, urgency)) = sessions.on_state(status)
            && let Some(announcer) = self.0.try_state::<Arc<dyn Announcer>>()
        {
            announcer.announce(&text, urgency);
        }
        if let Some(tray) = self.0.try_state::<Arc<Tray>>() {
            tray.set_status(status);
        }
        self.emit(EV_STATE, status);
    }
    fn stats(&self, stats: &Stats) {
        self.emit(EV_STATS, stats);
    }
    fn log(&self, entry: &LogEntry) {
        self.emit(EV_LOG, entry);
    }
}

#[derive(serde::Serialize, Clone)]
struct DropHover {
    active: bool,
}

/// Hands the paths that came from outside to the imports and tells the page what it can show: the preview, or the error.
/// Reading the files is not quick: the drop handler calls it off the main thread.
pub fn offer_from_outside<R: Runtime>(app: &AppHandle<R>, paths: &[PathBuf], origin: Origin) {
    let Some(imports) = app.try_state::<Arc<Imports>>() else {
        return;
    };
    let Some(offer) = imports.offer_from_outside(paths, origin) else {
        return;
    };
    if let Err(e) = app.emit(EV_OPEN_FILE, &offer) {
        eprintln!("galleria-gui: cannot emit {EV_OPEN_FILE}: {e}");
    }
}

/// The drag of a file over the main window (plan B4, T4). Tauri gives the paths to Rust here and Rust is the only one that
/// keeps them: the page gets a yes or a no for the hint, and then the offer, which has no paths.
pub fn on_drag_drop<R: Runtime>(window: &Window<R>, event: &WindowEvent) {
    let WindowEvent::DragDrop(drag) = event else {
        return;
    };
    if window.label() != "main" {
        return;
    }
    let app = window.app_handle().clone();
    let hover = |active| {
        let _ = app.emit(EV_DROP_HOVER, DropHover { active });
    };
    match drag {
        DragDropEvent::Enter { .. } => hover(true),
        DragDropEvent::Leave => hover(false),
        DragDropEvent::Drop { paths, .. } => {
            hover(false);
            let (app, paths) = (app.clone(), paths.clone());
            tauri::async_runtime::spawn_blocking(move || {
                offer_from_outside(&app, &paths, Origin::Drop);
            });
        }
        _ => {}
    }
}
