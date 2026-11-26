//! The request of the version check with libsoup (plan B6, T4, Q51 (a)): the one WebKitGTK has already loaded, so no crate is added. One
//! `GET`, no redirect followed, no `User-Agent`, no cookie, at most [`crate::update::REQUEST_TIMEOUT`]; only the status and the `Location` of
//! the answer are read and the body is thrown away. It uses the proxy and the certificates of the system, as libsoup does.
//!
//! libsoup works on the main context of GLib: the request is handed to it from any thread (`MainContext::invoke`), where the main loop of
//! GTK (the app) or of the test runs it, and the thread that asked waits for the answer.

use crate::update::{Fetch, FetchError, REQUEST_TIMEOUT};
use gtk::{gio, glib, prelude::CancellableExt};
use soup::{MessageFlags, prelude::*};
use std::{sync::mpsc, time::Duration};

pub struct SoupFetch {
    timeout: Duration,
}

impl SoupFetch {
    #[must_use]
    pub fn new() -> Self {
        Self::with_timeout(REQUEST_TIMEOUT)
    }

    /// The tests wait less than ten seconds for a server that does not answer.
    #[must_use]
    pub fn with_timeout(timeout: Duration) -> Self {
        Self { timeout }
    }
}

impl Default for SoupFetch {
    fn default() -> Self {
        Self::new()
    }
}

type Answer = Result<(u16, Option<String>), FetchError>;

impl Fetch for SoupFetch {
    fn latest(&self, url: &str) -> Answer {
        let (send, receive) = mpsc::channel::<Answer>();
        let cancel = gio::Cancellable::new();
        let (url, stop) = (url.to_owned(), cancel.clone());
        let seconds = u32::try_from(self.timeout.as_secs().max(1)).unwrap_or(10);
        glib::MainContext::default().invoke(move || ask(&url, seconds, &stop, send));
        // a little more than the timeout of the session: the request ends by itself, and if the loop is stuck the wait does not
        match receive.recv_timeout(self.timeout + Duration::from_secs(2)) {
            Ok(answer) => answer,
            Err(_) => {
                cancel.cancel();
                Err(FetchError::Timeout)
            }
        }
    }
}

/// On the main context: builds the session and the message, sends, and gives the answer to the thread that waits.
fn ask(url: &str, seconds: u32, cancel: &gio::Cancellable, send: mpsc::Sender<Answer>) {
    let Ok(message) = soup::Message::new("GET", url) else {
        let _ = send.send(Err(FetchError::Network));
        return;
    };
    // the redirect is the answer: its `Location` has the version, and it is never followed
    message.set_flags(MessageFlags::NO_REDIRECT);
    let session = soup::Session::new();
    session.set_timeout(seconds);
    session.set_idle_timeout(0);
    // a request that is the same for everyone: nothing that says who asks (the user agent is none, `soup::Session` keeps no cookies
    // unless a jar is added, and none is)
    session.set_accept_language_auto(false);
    let sent = message.clone();
    let keep = session.clone();
    session.send_and_read_async(
        &message,
        glib::Priority::DEFAULT,
        Some(cancel),
        move |result| {
            let _session_lives_until_the_answer = &keep;
            let answer = match result {
                Ok(_body) => {
                    let location = sent
                        .response_headers()
                        .and_then(|headers| headers.one("Location"))
                        .map(|value| value.to_string());
                    Ok((u16::try_from(sent.status_code()).unwrap_or(0), location))
                }
                Err(error)
                    if error.matches(gio::IOErrorEnum::TimedOut)
                        || error.matches(gio::IOErrorEnum::Cancelled) =>
                {
                    Err(FetchError::Timeout)
                }
                Err(_) => Err(FetchError::Network),
            };
            let _ = send.send(answer);
        },
    );
}
