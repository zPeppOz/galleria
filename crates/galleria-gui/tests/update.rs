#![cfg(unix)]
//! The version check (plan B6, T4, G11): when it asks, what it takes from the answer, and what it sends. The server of these tests is a
//! `TcpListener` in a thread that counts the connections and keeps every request as it came; the request itself is made by libsoup
//! (`update_soup`), with the main loop of GLib running in a thread of the test, as the one of the app runs in its main thread.
#![allow(clippy::unwrap_used, clippy::expect_used)]

#[cfg(target_os = "linux")]
use galleria_gui::update_soup::SoupFetch;
use galleria_gui::{
    ui_state::{UiStateFile, UpdateState},
    update::{
        DAY, Endpoint, Fetch, FetchError, RELEASES_LATEST, UpdateView, Updater, due, endpoint,
        notice, parse,
    },
};
use rstest::rstest;
use semver::Version;
use std::{
    io::{Read, Write},
    net::{Shutdown, TcpListener, TcpStream},
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
use time::OffsetDateTime;

const TAG: &str = "https://github.com/zPeppOz/galleria/releases/tag/v";
const CURRENT: &str = "0.1.0";

fn version(text: &str) -> Version {
    Version::parse(text).unwrap()
}

fn at(seconds: i64) -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp(1_800_000_000 + seconds).unwrap()
}

// ---- the server that counts ---------------------------------------------------------------------------------------------------

enum Reply {
    /// `302` with this `Location`.
    Redirect(String),
    /// An answer that has no `Location`.
    Status(u16),
    /// Takes the connection and never answers.
    Silent,
}

struct Server {
    port: u16,
    connections: Arc<AtomicUsize>,
    requests: Arc<Mutex<Vec<String>>>,
}

impl Server {
    fn start(reply: Reply) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let connections = Arc::new(AtomicUsize::new(0));
        let requests = Arc::new(Mutex::new(Vec::new()));
        let reply = Arc::new(reply);
        let (count, kept) = (Arc::clone(&connections), Arc::clone(&requests));
        std::thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                count.fetch_add(1, Ordering::SeqCst);
                let (kept, reply) = (Arc::clone(&kept), Arc::clone(&reply));
                std::thread::spawn(move || answer(stream, &kept, &reply));
            }
        });
        Self {
            port,
            connections,
            requests,
        }
    }

    /// An address on this machine where nobody listens.
    fn closed_port() -> u16 {
        TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port()
    }

    fn url(&self) -> String {
        format!(
            "http://127.0.0.1:{}/zPeppOz/galleria/releases/latest",
            self.port
        )
    }

    fn connections(&self) -> usize {
        self.connections.load(Ordering::SeqCst)
    }

    fn requests(&self) -> Vec<String> {
        self.requests.lock().unwrap().clone()
    }
}

fn answer(mut stream: TcpStream, kept: &Mutex<Vec<String>>, reply: &Reply) {
    stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    let mut text = Vec::new();
    let mut chunk = [0u8; 1024];
    while !text.windows(4).any(|w| w == b"\r\n\r\n") {
        match stream.read(&mut chunk) {
            Ok(0) | Err(_) => break,
            Ok(n) => text.extend_from_slice(&chunk[..n]),
        }
    }
    kept.lock()
        .unwrap()
        .push(String::from_utf8_lossy(&text).into_owned());
    let head = match reply {
        Reply::Redirect(location) => format!("HTTP/1.1 302 Found\r\nLocation: {location}\r\n"),
        Reply::Status(code) => format!("HTTP/1.1 {code} Reply\r\n"),
        Reply::Silent => {
            std::thread::sleep(Duration::from_secs(6));
            return;
        }
    };
    let _ = stream
        .write_all(format!("{head}Content-Length: 0\r\nConnection: close\r\n\r\n").as_bytes());
    let _ = stream.shutdown(Shutdown::Both);
}

// ---- the main loop of GLib, as the app has it in its main thread -----------------------------------------------------------------

#[cfg(target_os = "linux")]
fn main_loop() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        std::thread::spawn(|| gtk::glib::MainLoop::new(None, false).run());
        // the thread owns the default context once the loop runs: the requests of the tests are handed to it
        std::thread::sleep(Duration::from_millis(200));
    });
}

#[cfg(target_os = "linux")]
fn soup(timeout: Duration) -> Arc<dyn Fetch> {
    main_loop();
    Arc::new(SoupFetch::with_timeout(timeout))
}

// ---- a fetch that answers what the test says -----------------------------------------------------------------------------------

struct Canned {
    asked: AtomicUsize,
    answer: Mutex<Result<(u16, Option<String>), FetchError>>,
}

impl Canned {
    fn new(answer: Result<(u16, Option<String>), FetchError>) -> Arc<Self> {
        Arc::new(Self {
            asked: AtomicUsize::new(0),
            answer: Mutex::new(answer),
        })
    }

    fn asked(&self) -> usize {
        self.asked.load(Ordering::SeqCst)
    }
}

impl Fetch for Canned {
    fn latest(&self, _url: &str) -> Result<(u16, Option<String>), FetchError> {
        self.asked.fetch_add(1, Ordering::SeqCst);
        self.answer.lock().unwrap().clone()
    }
}

struct Rig {
    rt: tokio::runtime::Runtime,
    state: Arc<UiStateFile>,
    updater: Updater,
    clock: Arc<Mutex<OffsetDateTime>>,
    _dir: tempfile::TempDir,
}

fn rig(fetch: Arc<dyn Fetch>, url: &str, check: bool) -> Rig {
    let dir = tempfile::tempdir().unwrap();
    let state = Arc::new(UiStateFile::new(dir.path().join("galleria/state.toml")));
    if !check {
        state.update(|s| s.update.check = false).unwrap();
    }
    let clock = Arc::new(Mutex::new(at(0)));
    let now = Arc::clone(&clock);
    let updater = Updater::new(
        Arc::clone(&state),
        fetch,
        url.to_owned(),
        version(CURRENT),
        Arc::new(move || *now.lock().unwrap()),
    );
    Rig {
        rt: tokio::runtime::Runtime::new().unwrap(),
        state,
        updater,
        clock,
        _dir: dir,
    }
}

impl Rig {
    fn tick(&self) -> Option<galleria_gui::update::Notice> {
        self.rt.block_on(self.updater.tick())
    }

    fn move_clock(&self, seconds: i64) {
        *self.clock.lock().unwrap() = at(seconds);
    }
}

fn stderr_during(work: impl FnOnce()) -> String {
    static ONE_AT_A_TIME: Mutex<()> = Mutex::new(());
    let _only = ONE_AT_A_TIME
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    use std::io::{Seek, SeekFrom};
    let mut capture = tempfile::tempfile().unwrap();
    let saved = nix::unistd::dup(std::io::stderr()).unwrap();
    nix::unistd::dup2_stderr(&capture).unwrap();
    work();
    nix::unistd::dup2_stderr(&saved).unwrap();
    let mut text = String::new();
    capture.seek(SeekFrom::Start(0)).unwrap();
    capture.read_to_string(&mut text).unwrap();
    text
}

// ---- G11: one request, no identifier -------------------------------------------------------------------------------------------

#[cfg(target_os = "linux")]
#[test]
fn update_asks_once_and_sends_no_identifier() {
    let server = Server::start(Reply::Redirect(format!("{TAG}9.9.9")));
    let rig = rig(soup(Duration::from_secs(10)), &server.url(), true);
    // the first tick asks and finds the version; nine more, one after the other, ask nothing
    let found = rig.tick().expect("the version was not found");
    assert_eq!(found.version, version("9.9.9"));
    for _ in 0..9 {
        assert!(rig.tick().is_none());
    }
    assert_eq!(server.connections(), 1, "more than one connection");
    let requests = server.requests();
    assert_eq!(requests.len(), 1, "{requests:?}");
    let text = &requests[0];
    let mut lines = text.split("\r\n");
    assert_eq!(
        lines.next().unwrap(),
        "GET /zPeppOz/galleria/releases/latest HTTP/1.1",
        "no parameter, no other target"
    );
    let names: Vec<String> = lines
        .take_while(|l| !l.is_empty())
        .map(|l| l.split(':').next().unwrap().to_ascii_lowercase())
        .collect();
    // what libsoup sends without being asked (S52, measured): exactly these three, and the README («What leaves your machine») lists them
    let mut sorted = names.clone();
    sorted.sort();
    assert_eq!(sorted, ["accept-encoding", "connection", "host"], "{text}");
    assert!(
        text.contains("Accept-Encoding: gzip, deflate\r\n"),
        "{text}"
    );
    assert!(text.contains("Connection: Keep-Alive\r\n"), "{text}");
    for forbidden in [
        "user-agent",
        "cookie",
        "referer",
        "authorization",
        "if-none-match",
        "if-modified-since",
        "accept-language",
    ] {
        assert!(!names.iter().any(|n| n == forbidden), "{forbidden}: {text}");
    }
    assert!(!names.iter().any(|n| n.starts_with("x-")), "{text}");
    assert!(
        !text.contains("0.1.0") && !text.contains("galleria-gui"),
        "the version installed is in the request: {text}"
    );
    assert!(
        text.ends_with("\r\n\r\n"),
        "a body follows the headers: {text}"
    );
}

#[cfg(target_os = "linux")]
#[test]
fn update_disabled_asks_nothing() {
    let server = Server::start(Reply::Redirect(format!("{TAG}9.9.9")));
    let rig = rig(soup(Duration::from_secs(10)), &server.url(), false);
    for _ in 0..10 {
        assert!(rig.tick().is_none());
    }
    // a day later and the view of the page: still nothing
    rig.move_clock(2 * 24 * 3600);
    assert!(rig.tick().is_none());
    let view = rig.updater.view();
    assert!(!view.enabled && view.latest.is_none());
    assert_eq!(server.connections(), 0);
}

#[test]
fn update_asks_at_most_once_a_day() {
    let fetch = Canned::new(Ok((302, Some(format!("{TAG}0.0.1")))));
    let rig = rig(fetch.clone(), "http://127.0.0.1:1/x", true);
    assert!(rig.tick().is_none());
    assert_eq!(fetch.asked(), 1);
    // 23 hours and 59 minutes later nothing is asked; at 24 hours one request
    rig.move_clock(23 * 3600 + 59 * 60);
    assert!(rig.tick().is_none());
    assert_eq!(fetch.asked(), 1);
    rig.move_clock(24 * 3600);
    assert!(rig.tick().is_none());
    assert_eq!(fetch.asked(), 2);
}

#[test]
fn a_failed_request_counts_for_the_day_too() {
    let fetch = Canned::new(Err(FetchError::Network));
    let rig = rig(fetch.clone(), "http://127.0.0.1:1/x", true);
    let said = stderr_during(|| {
        assert!(rig.tick().is_none());
        // the second tick, an hour later, does not try again
        rig.move_clock(3600);
        assert!(rig.tick().is_none());
    });
    assert_eq!(fetch.asked(), 1);
    // one line, with the kind of the error, no address
    assert!(said.contains("version check failed"), "{said}");
    assert!(!said.contains("127.0.0.1"), "{said}");
    assert_eq!(said.matches("version check failed").count(), 1, "{said}");
}

#[test]
fn a_state_that_cannot_be_written_asks_nothing() {
    // `last_check` is written before the request: without that write a failing disk would ask at every tick
    let fetch = Canned::new(Ok((302, Some(format!("{TAG}9.9.9")))));
    let dir = tempfile::tempdir().unwrap();
    let blocker = dir.path().join("blocker");
    std::fs::write(&blocker, "x").unwrap();
    let state = Arc::new(UiStateFile::new(blocker.join("state.toml")));
    let updater = Updater::new(
        state,
        fetch.clone(),
        "http://127.0.0.1:1/x".to_owned(),
        version(CURRENT),
        Arc::new(|| at(0)),
    );
    let rt = tokio::runtime::Runtime::new().unwrap();
    let _ = stderr_during(|| assert!(rt.block_on(updater.tick()).is_none()));
    assert_eq!(fetch.asked(), 0);
}

#[cfg(target_os = "linux")]
#[test]
fn update_never_follows_a_redirect() {
    let second = Server::start(Reply::Status(200));
    let first = Server::start(Reply::Redirect(format!(
        "http://127.0.0.1:{}/elsewhere",
        second.port
    )));
    let fetch = soup(Duration::from_secs(10));
    // the redirect is the answer: its status and its `Location`, and the second server never hears of it
    let (status, location) = fetch.latest(&first.url()).unwrap();
    assert_eq!(status, 302);
    assert_eq!(
        location.as_deref(),
        Some(format!("http://127.0.0.1:{}/elsewhere", second.port).as_str())
    );
    std::thread::sleep(Duration::from_millis(500));
    assert_eq!(first.connections(), 1);
    assert_eq!(second.connections(), 0, "the redirect was followed");
}

// ---- what counts as a version -------------------------------------------------------------------------------------------------

fn tag(rest: &str) -> Option<String> {
    Some(format!("{TAG}{rest}"))
}

#[rstest]
#[case::the_normal_case(302, tag("0.2.0"), Some("0.2.0"))]
#[case::another_redirect_status(307, tag("1.0.0"), Some("1.0.0"))]
#[case::the_answer_of_today_the_list(302, Some("https://github.com/zPeppOz/galleria/releases".to_owned()), None)]
#[case::another_owner(302, Some("https://github.com/someone/galleria/releases/tag/v0.2.0".to_owned()), None)]
#[case::another_repository(302, Some("https://github.com/zPeppOz/other/releases/tag/v0.2.0".to_owned()), None)]
#[case::another_host(302, Some("https://evil.example/zPeppOz/galleria/releases/tag/v0.2.0".to_owned()), None)]
#[case::a_host_that_only_starts_like_it(302, Some("https://github.com.evil.example/zPeppOz/galleria/releases/tag/v0.2.0".to_owned()), None)]
#[case::not_https(302, Some("http://github.com/zPeppOz/galleria/releases/tag/v0.2.0".to_owned()), None)]
#[case::a_path_only(302, Some("/zPeppOz/galleria/releases/tag/v0.2.0".to_owned()), None)]
#[case::a_prerelease(302, tag("0.2.0-rc.1"), None)]
#[case::build_metadata(302, tag("0.2.0+build"), None)]
#[case::a_path_after_it(302, tag("0.2.0/../../x"), None)]
#[case::a_query(302, tag("0.2.0?x=1"), None)]
#[case::a_fragment(302, tag("0.2.0#x"), None)]
#[case::a_line_break(302, tag("0.2.0\r\nX-Evil: 1"), None)]
#[case::no_v(302, tag("0.2.0").map(|t| t.replace("/v0.2.0", "/0.2.0")), None)]
#[case::two_numbers_only(302, tag("0.2"), None)]
#[case::leading_zero(302, tag("01.2.3"), None)]
#[case::a_thousand_characters(302, tag(&"9".repeat(1000)), None)]
#[case::an_answer_without_location(200, None, None)]
#[case::a_location_without_a_redirect(200, tag("0.2.0"), None)]
#[case::not_found(404, None, None)]
#[case::an_error_of_the_server(500, tag("0.2.0"), None)]
fn parse_takes_only_a_stable_version_of_this_repository(
    #[case] status: u16,
    #[case] location: Option<String>,
    #[case] expected: Option<&str>,
) {
    assert_eq!(parse(status, location.as_deref()), expected.map(version));
}

fn known(latest: Option<&str>, dismissed: Option<&str>, check: bool) -> UpdateState {
    UpdateState {
        check,
        last_check: None,
        latest: latest.map(str::to_owned),
        dismissed: dismissed.map(str::to_owned),
    }
}

#[rstest]
#[case::newer(Some("0.2.0"), true)]
#[case::the_same(Some("0.1.0"), false)]
#[case::older(Some("0.0.9"), false)]
#[case::nothing_known(None, false)]
#[case::garbage_on_the_disk(Some("not a version"), false)]
#[case::a_prerelease_on_the_disk(Some("0.2.0-rc.1"), false)]
fn only_a_newer_stable_version_of_this_repository_counts(
    #[case] latest: Option<&str>,
    #[case] shown: bool,
) {
    let found = notice(&version(CURRENT), &known(latest, None, true));
    assert_eq!(found.is_some(), shown);
    if let Some(found) = found {
        // the address is built from the number: nothing of what the answer said
        assert_eq!(found.version, version("0.2.0"));
        assert_eq!(found.url, format!("{TAG}0.2.0"));
    }
    // turned off, there is nothing to show whatever is known
    assert!(notice(&version(CURRENT), &known(latest, None, false)).is_none());
}

#[test]
fn a_dismissed_version_is_not_shown_again_and_a_newer_one_is() {
    let current = version(CURRENT);
    assert!(notice(&current, &known(Some("0.2.0"), Some("0.2.0"), true)).is_none());
    // a version that came after the one that was dismissed is shown
    assert_eq!(
        notice(&current, &known(Some("0.3.0"), Some("0.2.0"), true)).map(|n| n.version),
        Some(version("0.3.0"))
    );
    // a dismissal of something newer than what is known keeps it quiet too
    assert!(notice(&current, &known(Some("0.2.0"), Some("0.9.0"), true)).is_none());
}

#[test]
fn due_asks_only_when_it_is_on_and_a_day_has_passed() {
    let mut state = UpdateState::default();
    assert!(due(&state, at(0)), "never asked: ask");
    state.last_check = Some(
        at(0)
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap(),
    );
    assert!(!due(&state, at(DAY.as_secs() as i64 - 1)));
    assert!(due(&state, at(DAY.as_secs() as i64)));
    state.last_check = Some("not a date".to_owned());
    assert!(
        due(&state, at(0)),
        "a date that cannot be read is the same as no date"
    );
    state.check = false;
    assert!(!due(&state, at(10 * DAY.as_secs() as i64)));
}

// ---- nothing of the answer reaches the page ------------------------------------------------------------------------------------

#[rstest]
#[case::after_the_tag("9.9.9/MARKER-5521")]
#[case::as_build_metadata("9.9.9+MARKER-5521")]
#[case::as_a_prerelease("9.9.9-MARKER-5521")]
#[case::in_the_query("9.9.9?MARKER-5521")]
fn the_view_never_carries_a_string_of_the_answer(#[case] rest: &str) {
    let fetch = Canned::new(Ok((302, tag(rest))));
    let rig = rig(fetch, "http://127.0.0.1:1/x", true);
    let said = stderr_during(|| assert!(rig.tick().is_none()));
    let view = serde_json::to_string(&rig.updater.view()).unwrap();
    let disk = std::fs::read_to_string(rig.state.path()).unwrap_or_default();
    for (place, text) in [
        ("stderr", &said),
        ("the view", &view),
        ("state.toml", &disk),
    ] {
        assert!(!text.contains("MARKER"), "{place}: {text}");
    }
}

#[test]
fn a_version_found_is_kept_until_a_request_answers_otherwise() {
    let fetch = Canned::new(Ok((302, tag("9.9.9"))));
    let rig = rig(fetch.clone(), "http://127.0.0.1:1/x", true);
    assert!(rig.updater.view().latest.is_none());
    assert_eq!(rig.tick().unwrap().url, format!("{TAG}9.9.9"));
    // the view names both versions and the address, which Rust built from the number
    assert_eq!(
        rig.updater.view(),
        UpdateView {
            current: CURRENT.to_owned(),
            latest: Some("9.9.9".to_owned()),
            url: Some(format!("{TAG}9.9.9")),
            enabled: true,
        }
    );
    // a day later a request that fails keeps what was known
    *fetch.answer.lock().unwrap() = Err(FetchError::Timeout);
    rig.move_clock(i64::try_from(DAY.as_secs()).unwrap());
    let _ = stderr_during(|| assert!(rig.tick().is_none()));
    assert_eq!(rig.updater.view().latest.as_deref(), Some("9.9.9"));
    // and one that answers that no version is known clears it
    *fetch.answer.lock().unwrap() = Ok((
        302,
        Some("https://github.com/zPeppOz/galleria/releases".to_owned()),
    ));
    rig.move_clock(2 * i64::try_from(DAY.as_secs()).unwrap());
    assert!(rig.tick().is_none());
    assert!(rig.updater.view().latest.is_none());
    assert_eq!(fetch.asked(), 3);
}

// ---- nothing is said to the page when the request fails ---------------------------------------------------------------------------

#[cfg(target_os = "linux")]
#[test]
fn a_failed_request_says_nothing_to_the_page() {
    // the server is not there; and the server takes the connection and says nothing until the time is up
    let closed = format!(
        "http://127.0.0.1:{}/zPeppOz/galleria/releases/latest",
        Server::closed_port()
    );
    let silent = Server::start(Reply::Silent);
    for (url, timeout) in [
        (closed, Duration::from_secs(10)),
        (silent.url(), Duration::from_secs(1)),
    ] {
        let rig = rig(soup(timeout), &url, true);
        let started = std::time::Instant::now();
        let said = stderr_during(|| assert!(rig.tick().is_none()));
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "waited {:?}",
            started.elapsed()
        );
        let view = rig.updater.view();
        assert!(view.latest.is_none() && view.url.is_none());
        assert!(said.contains("version check failed"), "{said}");
        assert!(
            !said.contains("127.0.0.1") && !said.contains("zPeppOz"),
            "{said}"
        );
    }
}

// ---- the address, and the only way to change it -------------------------------------------------------------------------------------

#[rstest]
#[case::nothing(None, false)]
#[case::loopback(Some("http://127.0.0.1:8081/zPeppOz/galleria/releases/latest"), true)]
#[case::loopback_without_a_path(Some("http://127.0.0.1:8081/"), true)]
#[case::another_host(Some("https://example.org/zPeppOz/galleria/releases/latest"), false)]
#[case::plain_http_elsewhere(Some("http://example.org/x"), false)]
#[case::localhost_by_name(Some("http://localhost:8081/x"), false)]
#[case::a_lookalike(Some("http://127.0.0.1.example.org:8081/x"), false)]
#[case::userinfo(Some("http://127.0.0.1:8081@example.org/x"), false)]
#[case::no_port(Some("http://127.0.0.1/x"), false)]
#[case::https_on_loopback(Some("https://127.0.0.1:8081/x"), false)]
#[case::a_space(Some("http://127.0.0.1:8081/a b"), false)]
fn the_override_is_honoured_only_for_loopback(#[case] value: Option<&str>, #[case] honoured: bool) {
    let Endpoint { url, ignored } = endpoint(value);
    if honoured {
        assert_eq!(Some(url.as_str()), value);
        assert!(!ignored);
    } else {
        assert_eq!(url, RELEASES_LATEST, "the default address stays");
        // only a value that was given and refused is said on stderr
        assert_eq!(ignored, value.is_some());
    }
}

#[test]
fn the_only_remote_address_of_the_crate_is_in_update_rs() {
    // `grep -rn "https\?://" crates/galleria-gui/src` finds one address that is remote, and it is there (plan B6, T4, «Fatto quando»)
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut found = Vec::new();
    for entry in std::fs::read_dir(&src).unwrap().flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "rs") {
            let text = std::fs::read_to_string(&path).unwrap();
            for line in text.lines().filter(|l| !l.trim_start().starts_with("//")) {
                let code = line;
                for scheme in ["http://", "https://"] {
                    for (i, _) in code.match_indices(scheme) {
                        let host = &code[i + scheme.len()..];
                        if !host.starts_with("127.0.0.1") && !host.starts_with('<') {
                            found.push(format!(
                                "{}: {line}",
                                path.file_name().unwrap().to_string_lossy()
                            ));
                        }
                    }
                }
            }
        }
    }
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].starts_with("update.rs:"), "{found:?}");
}

#[test]
fn update_view_makes_no_request() {
    let fetch = Canned::new(Ok((302, tag("9.9.9"))));
    let rig = rig(fetch.clone(), "http://127.0.0.1:1/x", true);
    for _ in 0..5 {
        let _ = rig.updater.view();
    }
    assert_eq!(fetch.asked(), 0);
}
