//! The menu of the tray (plan B6, T1). No type of Tauri in here: the state, the favourites and the texts go into a pure
//! function ([`model`]) and a list of entries comes out; [`Tray`] keeps what the model needs, decides when the menu has to be
//! written again and hands the entries to a [`Surface`], which `tray_tauri` makes out of the real menu.
//!
//! The tray is an accessory (analysis §8): everything the menu does is done from the window too, and without a tray (no
//! library of the indicator, no one that shows it) the app is whole and does not panic.

use crate::{announce::clean, link::DaemonView, link::LinkState};
use galleria_ipc::{State, Status};
use serde::Deserialize;
use std::sync::{Mutex, MutexGuard};
use uuid::Uuid;

/// The favourites the menu lists, at most (P2).
pub const MAX_FAVOURITES: usize = 8;
/// The longest text the page may give (P3).
pub const LABEL_MAX: usize = 200;

/// What a click on an entry asks for. The page does `Connect` and `Disconnect` (Q52); `Open` and `Quit` are Rust's.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TrayAction {
    Connect(Uuid),
    Disconnect,
    Open,
    Quit,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TrayEntry {
    /// The line of the state: it is text, not a choice.
    Label {
        text: String,
    },
    Item {
        text: String,
        enabled: bool,
        action: TrayAction,
    },
    Separator,
}

/// Which of the three icons the tray shows (P5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrayIcon {
    Off,
    On,
    Busy,
}

/// The texts of the menu in the language of the window (P3). `{name}` is the only placeholder.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
pub struct TrayLabels {
    pub open: String,
    pub quit: String,
    /// «Quit (the VPN stays on)»: the text of «Quit» while a session is up (Q54).
    pub quit_connected: String,
    pub connect: String,
    pub disconnect: String,
    pub connected: String,
    pub disconnected: String,
    pub connecting: String,
    pub unreachable: String,
}

/// A text of the page that is not accepted.
#[derive(Debug, PartialEq, Eq)]
pub struct InvalidLabels;

impl TrayLabels {
    /// Every text cleaned like an announcement (no control character, one line) and refused when it is empty or longer than
    /// [`LABEL_MAX`] characters: the page is not trusted with what the desktop draws.
    ///
    /// # Errors
    ///
    /// [`InvalidLabels`] for a text that is empty after the cleaning or too long.
    pub fn checked(mut self) -> Result<Self, InvalidLabels> {
        for text in [
            &mut self.open,
            &mut self.quit,
            &mut self.quit_connected,
            &mut self.connect,
            &mut self.disconnect,
            &mut self.connected,
            &mut self.disconnected,
            &mut self.connecting,
            &mut self.unreachable,
        ] {
            if text.chars().count() > LABEL_MAX {
                return Err(InvalidLabels);
            }
            *text = clean(text).ok_or(InvalidLabels)?;
        }
        Ok(self)
    }
}

pub struct TrayInput<'a> {
    pub link: LinkState,
    pub status: &'a Status,
    pub favourites: &'a [(Uuid, String)],
    pub labels: &'a TrayLabels,
}

/// A name of a profile as one line of text: the line breaks and the control characters go (the rest is the name).
fn flat(name: &str) -> String {
    clean(name).unwrap_or_default()
}

/// The text of an entry for the menu of Tauri, where `&` marks the letter of a shortcut: a name with one shows it as it is, and
/// no entry has a shortcut. (`_` needs nothing: the menu of Tauri doubles it for GTK.)
#[must_use]
pub fn literal(text: &str) -> String {
    text.replace('&', "&&")
}

fn with_name(template: &str, name: &str) -> String {
    template.replace("{name}", &flat(name))
}

/// P2, P4, P5: the menu for what the window knows, from the top: the line of the state, a separator, the favourites in the
/// order of the list (at most eight; the one of the session offers «Disconnect», the others «Connect»), a row of its own for a
/// session that is not one of them, a separator, «Open» and «Quit». Without a link to the daemon nothing can be connected.
#[must_use]
pub fn model(input: &TrayInput<'_>) -> (Vec<TrayEntry>, TrayIcon) {
    let labels = input.labels;
    let linked = input.link == LinkState::Connected;
    let state = if linked {
        input.status.state
    } else {
        State::Disconnected
    };
    let active = input
        .status
        .profile
        .as_ref()
        .filter(|_| linked && state != State::Disconnected);
    let name = active.map_or("", |p| p.name.as_str());
    let (line, icon) = if !linked {
        (labels.unreachable.clone(), TrayIcon::Off)
    } else {
        match state {
            State::Disconnected => (labels.disconnected.clone(), TrayIcon::Off),
            State::Connected => (with_name(&labels.connected, name), TrayIcon::On),
            State::Connecting | State::NeedsInput | State::Reconnecting | State::Disconnecting => {
                (labels.connecting.clone(), TrayIcon::Busy)
            }
            // A session that failed: the line of «Disconnected» (there is no connection), the icon says it is not quiet.
            State::Error => (labels.disconnected.clone(), TrayIcon::Busy),
        }
    };
    let mut entries = vec![TrayEntry::Label { text: line }, TrayEntry::Separator];
    let mut active_shown = false;
    for (id, favourite) in input.favourites.iter().take(MAX_FAVOURITES) {
        let is_active = active.is_some_and(|p| p.id == id.to_string());
        active_shown |= is_active;
        entries.push(if is_active {
            TrayEntry::Item {
                text: with_name(&labels.disconnect, favourite),
                enabled: state != State::Disconnecting,
                action: TrayAction::Disconnect,
            }
        } else {
            TrayEntry::Item {
                text: with_name(&labels.connect, favourite),
                enabled: linked,
                action: TrayAction::Connect(*id),
            }
        });
    }
    if let Some(profile) = active
        && !active_shown
    {
        entries.push(TrayEntry::Item {
            text: with_name(&labels.disconnect, &profile.name),
            enabled: state != State::Disconnecting,
            action: TrayAction::Disconnect,
        });
    }
    entries.push(TrayEntry::Separator);
    entries.push(TrayEntry::Item {
        text: labels.open.clone(),
        enabled: true,
        action: TrayAction::Open,
    });
    entries.push(TrayEntry::Item {
        text: if state == State::Disconnected {
            labels.quit.clone()
        } else {
            labels.quit_connected.clone()
        },
        enabled: true,
        action: TrayAction::Quit,
    });
    (entries, icon)
}

/// Where the entries are drawn. `show` must not wait for the thread of the window: it is called with the lock of the tray held
/// and from the thread that delivers the events, so it only posts the work (`tray_tauri`).
pub trait Surface: Send {
    fn show(&mut self, entries: &[TrayEntry], icon: TrayIcon, generation: u64);
}

/// Makes the surface, once, when the texts arrive: `None` is a tray that cannot be made (the library of the indicator is not
/// there), and it is not tried again.
pub type Factory = Box<dyn Fn() -> Option<Box<dyn Surface>> + Send + Sync>;

struct Inner {
    link: LinkState,
    status: Status,
    favourites: Vec<(Uuid, String)>,
    labels: Option<TrayLabels>,
    surface: Option<Box<dyn Surface>>,
    tried: bool,
    shown: Option<(Vec<TrayEntry>, TrayIcon)>,
    generation: u64,
}

/// What the tray knows and what it shows. One for the app, in the state of Tauri; the supervisor, the commands and the rubrica
/// tell it what changed and it writes the menu again only when the model came out different.
pub struct Tray {
    factory: Factory,
    inner: Mutex<Inner>,
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

impl Tray {
    #[must_use]
    pub fn new(factory: Factory) -> Self {
        Self {
            factory,
            inner: Mutex::new(Inner {
                link: LinkState::Connecting,
                status: Status::default(),
                favourites: vec![],
                labels: None,
                surface: None,
                tried: false,
                shown: None,
                generation: 0,
            }),
        }
    }

    /// The texts of the page. The first delivery makes the tray (P3): before it there is none, and a menu in the wrong language
    /// for a moment is worse than one that appears half a second later.
    pub fn set_labels(&self, labels: TrayLabels) {
        let mut inner = lock(&self.inner);
        inner.labels = Some(labels);
        if !inner.tried {
            inner.tried = true;
            inner.surface = (self.factory)();
        }
        Self::refresh(&mut inner);
    }

    pub fn set_link(&self, view: &DaemonView) {
        let mut inner = lock(&self.inner);
        inner.link = view.link;
        if let Some(status) = &view.status {
            inner.status = status.clone();
        }
        Self::refresh(&mut inner);
    }

    pub fn set_status(&self, status: &Status) {
        let mut inner = lock(&self.inner);
        inner.status = status.clone();
        Self::refresh(&mut inner);
    }

    pub fn set_favourites(&self, favourites: Vec<(Uuid, String)>) {
        let mut inner = lock(&self.inner);
        if inner.favourites == favourites {
            return;
        }
        inner.favourites = favourites;
        Self::refresh(&mut inner);
    }

    /// Whether there is a tray of ours: the texts came, and the surface could be made.
    #[must_use]
    pub fn available(&self) -> bool {
        lock(&self.inner).surface.is_some()
    }

    /// What the click on the entry `index` of the menu of `generation` asks for. A click on a menu that was written again
    /// since (an older generation) asks for nothing.
    #[must_use]
    pub fn action(&self, generation: u64, index: usize) -> Option<TrayAction> {
        let inner = lock(&self.inner);
        if inner.generation != generation {
            return None;
        }
        match inner.shown.as_ref()?.0.get(index)? {
            TrayEntry::Item {
                action,
                enabled: true,
                ..
            } => Some(action.clone()),
            _ => None,
        }
    }

    fn refresh(inner: &mut Inner) {
        let (Some(labels), true) = (inner.labels.as_ref(), inner.surface.is_some()) else {
            return;
        };
        let now = model(&TrayInput {
            link: inner.link,
            status: &inner.status,
            favourites: &inner.favourites,
            labels,
        });
        if inner.shown.as_ref() == Some(&now) {
            return;
        }
        inner.generation += 1;
        let generation = inner.generation;
        if let Some(surface) = inner.surface.as_mut() {
            surface.show(&now.0, now.1, generation);
        }
        inner.shown = Some(now);
    }
}
