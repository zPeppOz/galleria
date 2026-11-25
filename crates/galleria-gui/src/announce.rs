//! What the screen reader is told (Q27, spec §12.1). No type of Tauri and none of GTK in here: the signal itself is
//! emitted by `announce_atk`, and the tests use an [`Announcer`] that records.
//!
//! The live regions of WebKitGTK stay silent under Orca, so an outcome that does not ask for an answer (saved,
//! duplicated, deleted) is also sent as an ATK signal on the accessible of the window; the atk-bridge turns it into
//! `object:announcement` on the AT-SPI bus, which Orca says.

/// *Polite* waits for what Orca is saying; *assertive* interrupts it. Only errors are assertive.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Urgency {
    Polite,
    Assertive,
}

pub trait Announcer: Send + Sync {
    fn announce(&self, text: &str, urgency: Urgency);
}

/// Where there is nothing to tell: not on Linux, or a system whose ATK has neither signal.
pub struct NoAnnouncer;

impl Announcer for NoAnnouncer {
    fn announce(&self, _text: &str, _urgency: Urgency) {}
}

/// The longest text sent, in characters: a screen reader reads all of it, and the webview is not trusted (P2).
pub const MAX_LEN: usize = 300;

/// Control characters go, white space (including line breaks) becomes one space, at most [`MAX_LEN`] characters;
/// `None` when nothing is left.
pub fn clean(text: &str) -> Option<String> {
    let mut out = String::new();
    let mut pending_space = false;
    for c in text.chars() {
        if c.is_whitespace() {
            pending_space = true;
        } else if !c.is_control() {
            if pending_space && !out.is_empty() {
                out.push(' ');
            }
            pending_space = false;
            out.push(c);
        }
    }
    let capped: String = out.chars().take(MAX_LEN).collect();
    let capped = capped.trim_end().to_owned();
    (!capped.is_empty()).then_some(capped)
}

/// The two ATK signals: `notification(text, urgency)` since ATK 2.50 and `announcement(text)` since 2.46.
#[derive(PartialEq, Eq, Debug)]
pub enum Signal {
    /// The number is `Atk.Live`: 1 polite, 2 assertive.
    Notification(i32),
    Announcement,
}

/// P3: `notification` when the object has it, `announcement` when only that is there, otherwise nothing.
pub fn pick_signal(
    has_notification: bool,
    has_announcement: bool,
    urgency: Urgency,
) -> Option<Signal> {
    if has_notification {
        Some(Signal::Notification(match urgency {
            Urgency::Polite => 1,
            Urgency::Assertive => 2,
        }))
    } else if has_announcement {
        Some(Signal::Announcement)
    } else {
        None
    }
}
