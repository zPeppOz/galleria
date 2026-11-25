//! Forwarding of the daemon events to the webview (spec §4.3).

use crate::{link::DaemonView, supervisor::EventSink};
use galleria_ipc::{LogEntry, Stats, Status};
use tauri::{AppHandle, Emitter, Runtime};

pub const EV_LINK: &str = "galleria:link";
pub const EV_STATE: &str = "galleria:state";
pub const EV_STATS: &str = "galleria:stats";
pub const EV_LOG: &str = "galleria:log";

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
        self.emit(EV_LINK, view);
    }
    fn state(&self, status: &Status) {
        self.emit(EV_STATE, status);
    }
    fn stats(&self, stats: &Stats) {
        self.emit(EV_STATS, stats);
    }
    fn log(&self, entry: &LogEntry) {
        self.emit(EV_LOG, entry);
    }
}
