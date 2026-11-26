//! What closing the window and quitting do (plan B6, T3, Q35, Q53, Q54): a decision with no type of Tauri, and the little state it
//! needs, which is one bit the page keeps up to date (are there changes that are not saved?).
//!
//! The decision is Rust's and it is made on the spot, on the bit the page last gave: it never waits for the page, so a page that is
//! stopped does not keep a window without changes from closing, and a page that is gone cannot make Rust throw changes away.

use crate::window::WindowControl;
use std::sync::atomic::{AtomicBool, Ordering};

/// The «X» of the window asks to hide it; «Quit» asks to end the app.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Intent {
    Hide,
    Quit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Decision {
    Hide,
    Quit,
    /// Nothing is done: the window is shown and the page asks what to do with the changes. The intent is the one that will be carried out
    /// afterwards (a hide that became a quit is a quit).
    Blocked(Intent),
}

/// `unsaved` is the bit of the page. `tray_shown` says the tray exists **and** someone shows it, now. A window that cannot come back is not
/// hidden: without a tray the «X» is a quit, as it was before B6 (Q53). Changes that are not saved block both, whatever the tray is.
#[must_use]
pub fn decide(intent: Intent, unsaved: bool, tray_shown: bool) -> Decision {
    let intent = match intent {
        Intent::Hide if !tray_shown => Intent::Quit,
        other => other,
    };
    if unsaved {
        Decision::Blocked(intent)
    } else {
        match intent {
            Intent::Hide => Decision::Hide,
            Intent::Quit => Decision::Quit,
        }
    }
}

/// The bit of the page and the way to ask whether the tray is shown. One for the app, in the state of Tauri.
pub struct Closing {
    unsaved: AtomicBool,
    tray_shown: Box<dyn Fn() -> bool + Send + Sync>,
}

impl Closing {
    pub fn new(tray_shown: impl Fn() -> bool + Send + Sync + 'static) -> Self {
        Self {
            unsaved: AtomicBool::new(false),
            tray_shown: Box::new(tray_shown),
        }
    }

    /// The page says whether it holds changes that are not saved: when that changes, and when the profile it edits changes.
    pub fn set_unsaved(&self, unsaved: bool) {
        self.unsaved.store(unsaved, Ordering::SeqCst);
    }

    #[must_use]
    pub fn unsaved(&self) -> bool {
        self.unsaved.load(Ordering::SeqCst)
    }

    /// Does what [`decide`] says, on `window`: the one place that hides the window or ends the app. `true` when it was done, `false` when
    /// the changes that are not saved stopped it (the window is shown and the page is told, `window.blocked`).
    pub fn request(&self, window: &dyn WindowControl, intent: Intent) -> bool {
        // the tray is asked about (a call on the bus) only when the answer can matter
        let shown = intent == Intent::Quit || (self.tray_shown)();
        match decide(intent, self.unsaved(), shown) {
            Decision::Hide => {
                window.hide();
                true
            }
            Decision::Quit => {
                window.quit();
                true
            }
            Decision::Blocked(intent) => {
                window.blocked(intent);
                false
            }
        }
    }
}
