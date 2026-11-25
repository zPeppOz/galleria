//! The only part of the crate that touches GTK: emits the ATK signal that Orca says. Linux only.

use crate::announce::{Announcer, Signal, Urgency, pick_signal};
use gtk::{
    glib::{self, object::ObjectExt, subclass::SignalId},
    prelude::{Cast, WidgetExt},
};
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Manager, Runtime};

/// Tells it once only: a system with an older ATK has neither signal, and every outcome would say so again.
static WARNED: AtomicBool = AtomicBool::new(false);

/// Emits the signal `pick_signal` chooses on `accessible` and says which; `None` when the object has neither signal.
///
/// `emit_by_name` panics when the signal does not exist, and `announcement` and `notification` only exist from ATK 2.46
/// and 2.50: GLib is asked first, so an old system loses the announcement and not the application.
pub fn emit_on(accessible: &glib::Object, text: &str, urgency: Urgency) -> Option<Signal> {
    let kind = accessible.type_();
    let has = |name: &str| SignalId::lookup(name, kind).is_some();
    let Some(signal) = pick_signal(has("notification"), has("announcement"), urgency) else {
        if !WARNED.swap(true, Ordering::Relaxed) {
            eprintln!(
                "galleria: this ATK has no announcement signal; outcomes are not announced to the screen reader"
            );
        }
        return None;
    };
    match signal {
        Signal::Notification(live) => {
            accessible.emit_by_name::<()>("notification", &[&text, &live])
        }
        Signal::Announcement => accessible.emit_by_name::<()>("announcement", &[&text]),
    }
    Some(signal)
}

/// Announces on the accessible of the window `main`, from the main thread of GTK: GTK is not thread safe, and the
/// commands run on another one.
pub struct AtkAnnouncer<R: Runtime>(pub AppHandle<R>);

impl<R: Runtime> Announcer for AtkAnnouncer<R> {
    fn announce(&self, text: &str, urgency: Urgency) {
        let app = self.0.clone();
        let text = text.to_owned();
        // A window that is gone or an event loop that has stopped is not an error worth telling the webview about.
        let _ = self.0.run_on_main_thread(move || {
            let Some(window) = app.get_webview_window("main") else {
                return;
            };
            let Ok(gtk_window) = window.gtk_window() else {
                return;
            };
            if let Some(accessible) = gtk_window.accessible() {
                emit_on(accessible.upcast_ref::<glib::Object>(), &text, urgency);
            }
        });
    }
}
