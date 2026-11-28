//! One window, and the file the file manager gives to it (plan B4, T5; spec §7, D31, Q36 (b)). No type of Tauri in here.
//!
//! The first process takes an exclusive lock on `gui.lock` in `$XDG_RUNTIME_DIR/galleria` (`flock`: it falls by itself when
//! the process dies), removes a `gui.sock` that was left behind and listens on a new one. A later process that finds the
//! lock taken connects to that socket, sends **one line** of JSON with the files it was given, waits for `ok` and ends.
//! No other crate, no bus of the session: a file in a private folder is all it takes.
//!
//! The channel is one more way into the app, so it is narrow (Q36):
//!
//! * the folder is `0700` and the socket `0600`; a process that finds the socket with wider permissions, or not its own,
//!   does not send anything to it;
//! * the first process accepts only connections of **its own user**, read with `SO_PEERCRED` (`peer_cred`) on the accepted
//!   connection, before it reads a byte;
//! * a message is one line of at most 64 KiB with at most 16 paths, version 1, and it has two seconds to arrive; anything
//!   else closes the connection;
//! * what comes through is **paths**, and a path is not trusted: it goes to `Imports::offer_from_outside` like a dropped one.
//!
//! On Windows there is no single instance yet (P6): every launch is the first, and works alone.

use std::{ffi::OsString, path::PathBuf, time::Duration};
#[cfg(unix)]
mod unix;
#[cfg(unix)]
pub use unix::{Primary, claim, files_in_args};
#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use windows::{Primary, claim, files_in_args};

/// The longest message, the most paths in one, and how long a connection has to say what it has to say.
pub const MAX_MESSAGE_BYTES: u64 = 64 * 1024;
pub const MAX_FILES: usize = 16;
pub const PEER_TIMEOUT: Duration = Duration::from_secs(2);

/// The folder of the channel: `galleria` inside the runtime folder of the session, which must be a path (an empty or
/// relative value is as good as none).
pub fn runtime_dir(xdg_runtime_dir: Option<OsString>) -> Option<PathBuf> {
    let base = PathBuf::from(xdg_runtime_dir?);
    base.is_absolute().then(|| base.join("galleria"))
}

/// What a launch of the app is: the first window (`Primary`), a process that gave its files to the first window and has
/// nothing left to do (`Handed`), a process that works on its own because there is no channel (`Alone`, with the reason)
/// or one that could not reach the first window (`Failed`, with the reason: it ends with an error, it does not open a
/// second window).
pub enum Claim {
    Primary(Primary),
    Handed,
    Alone(String),
    Failed(String),
}
