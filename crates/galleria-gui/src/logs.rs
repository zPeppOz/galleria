//! The log of the session and «Export log…» (plan B5, T7). No type of Tauri in here: the window that asks for the place is a
//! `SavePicker`, as for «Export» of a profile, and the text goes from the daemon to the file without passing through the webview.
//!
//! The lines are the daemon's: it redacts what it knows (spec §10) before it keeps a line, and this module neither translates,
//! filters nor writes them anywhere but the file the user chose. Nothing of the log goes to `stderr`.

use crate::{
    calls::{self, CallError},
    export::{ExportError, SavePicker, write_export},
};
use galleria_ipc::{LogEntry, LogsResult};
use std::{future::Future, path::Path};
use time::OffsetDateTime;

/// The most lines the daemon serves in one call, and the size of the pages `all` asks for.
pub const PAGE: usize = 1000;

/// Every line of the buffer of the daemon, oldest first: pages of [`PAGE`] lines, each after the last line of the one before,
/// until a page is short (a page that is full may be the last one, and then the next is empty).
///
/// # Errors
///
/// The error of the first call that fails.
pub async fn all(socket: &Path) -> Result<Vec<LogEntry>, CallError> {
    all_with(|after, limit| calls::logs(socket, after, limit)).await
}

/// [`all`] over any way of getting a page: the tests count the calls with it.
///
/// # Errors
///
/// The error of the first page that fails.
pub async fn all_with<F, Fut>(mut page: F) -> Result<Vec<LogEntry>, CallError>
where
    F: FnMut(Option<u64>, usize) -> Fut,
    Fut: Future<Output = Result<LogsResult, CallError>>,
{
    let mut lines = Vec::new();
    let mut after = None;
    loop {
        let LogsResult { entries, last } = page(after, PAGE).await?;
        let full = entries.len() >= PAGE;
        lines.extend(entries);
        if !full {
            return Ok(lines);
        }
        after = Some(last);
    }
}

/// The text of the file: a line for each entry in the format of `galleria logs` (`ts level source: message`), so that who attaches a
/// log can compare it with the one of the command line (P12).
#[must_use]
pub fn render(entries: &[LogEntry]) -> String {
    entries
        .iter()
        .map(|e| format!("{} {:?} {:?}: {}\n", e.ts, e.level, e.source, e.message))
        .collect()
}

/// The name offered in the window «Save»: `galleria-log-AAAAMMGG-HHMM.txt` (P12), in the time of the offset `now` carries.
#[must_use]
pub fn suggested_name(now: OffsetDateTime) -> String {
    format!(
        "galleria-log-{:04}{:02}{:02}-{:02}{:02}.txt",
        now.year(),
        u8::from(now.month()),
        now.day(),
        now.hour(),
        now.minute()
    )
}

/// Asks the window «Save» for a place and writes the lines there (`0600`). `Ok(false)` is a cancel, and a cancel writes nothing.
///
/// # Errors
///
/// [`ExportError`] with the place, when the file cannot be written.
pub fn save(
    entries: &[LogEntry],
    picker: &dyn SavePicker,
    now: OffsetDateTime,
) -> Result<bool, ExportError> {
    let Some(path) = picker.pick(&suggested_name(now)) else {
        return Ok(false);
    };
    write_export(&path, &render(entries))?;
    Ok(true)
}
