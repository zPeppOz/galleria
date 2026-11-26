//! The tray (plan B6, T1, T2): the menu out of the model, with the tables of the plan; the tray that keeps it and writes it again
//! only when it changed; the texts of the page; and, on the mock runtime, the click on an entry.
#![allow(clippy::unwrap_used, clippy::expect_used)]

#[path = "support/seed.rs"]
mod seed;
mod support;
#[path = "support/world.rs"]
mod world;

use galleria_gui::window::WindowControl;
use galleria_gui::{
    link::{DaemonView, LinkState},
    tray::{
        LABEL_MAX, MAX_FAVOURITES, Surface, Tray, TrayAction, TrayEntry, TrayIcon, TrayInput,
        TrayLabels, literal, model,
    },
    tray_tauri::{ActionPayload, EV_TRAY_ACTION, run_action},
};
use galleria_ipc::{ProfileRef, State, Status};
use rstest::rstest;
use serde_json::{Value, json};
use std::{
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tauri::Listener;
use uuid::Uuid;
use world::{Options, World, start};

fn labels() -> TrayLabels {
    TrayLabels {
        open: "Open Galleria".into(),
        quit: "Quit".into(),
        quit_connected: "Quit (the VPN stays on)".into(),
        connect: "Connect «{name}»".into(),
        disconnect: "Disconnect «{name}»".into(),
        connected: "Connected to «{name}»".into(),
        disconnected: "Disconnected".into(),
        connecting: "Connecting…".into(),
        unreachable: "Service not reachable".into(),
    }
}

fn id(n: u128) -> Uuid {
    Uuid::from_u128(n)
}

fn status(state: State, profile: Option<(Uuid, &str)>) -> Status {
    Status {
        state,
        profile: profile.map(|(id, name)| ProfileRef {
            id: id.to_string(),
            name: name.to_owned(),
        }),
        ..Status::default()
    }
}

fn build(
    link: LinkState,
    status: &Status,
    favourites: &[(Uuid, String)],
) -> (Vec<TrayEntry>, TrayIcon) {
    let labels = labels();
    model(&TrayInput {
        link,
        status,
        favourites,
        labels: &labels,
    })
}

fn favourites() -> Vec<(Uuid, String)> {
    vec![(id(1), "Ufficio".to_owned()), (id(2), "Casa".to_owned())]
}

fn texts(entries: &[TrayEntry]) -> Vec<String> {
    entries
        .iter()
        .map(|entry| match entry {
            TrayEntry::Label { text } => format!("[{text}]"),
            TrayEntry::Item {
                text,
                enabled: true,
                ..
            } => text.clone(),
            TrayEntry::Item { text, .. } => format!("({text})"),
            TrayEntry::Separator => "---".to_owned(),
        })
        .collect()
}

#[rstest]
#[case(
    State::Disconnected,
    None,
    "[Disconnected]",
    TrayIcon::Off,
    "Connect «Ufficio»",
    "Quit"
)]
#[case(State::Connecting, Some((1, "Ufficio")), "[Connecting…]", TrayIcon::Busy, "Disconnect «Ufficio»", "Quit (the VPN stays on)")]
#[case(State::NeedsInput, Some((1, "Ufficio")), "[Connecting…]", TrayIcon::Busy, "Disconnect «Ufficio»", "Quit (the VPN stays on)")]
#[case(State::Connected, Some((1, "Ufficio")), "[Connected to «Ufficio»]", TrayIcon::On, "Disconnect «Ufficio»", "Quit (the VPN stays on)")]
#[case(State::Reconnecting, Some((1, "Ufficio")), "[Connecting…]", TrayIcon::Busy, "Disconnect «Ufficio»", "Quit (the VPN stays on)")]
#[case(State::Disconnecting, Some((1, "Ufficio")), "[Connecting…]", TrayIcon::Busy, "(Disconnect «Ufficio»)", "Quit (the VPN stays on)")]
#[case(State::Error, Some((1, "Ufficio")), "[Disconnected]", TrayIcon::Busy, "Disconnect «Ufficio»", "Quit (the VPN stays on)")]
fn the_menu_for_each_state(
    #[case] state: State,
    #[case] profile: Option<(u128, &str)>,
    #[case] line: &str,
    #[case] icon: TrayIcon,
    #[case] first: &str,
    #[case] quit: &str,
) {
    let status = status(state, profile.map(|(n, name)| (id(n), name)));
    let (entries, shown) = build(LinkState::Connected, &status, &favourites());
    assert_eq!(
        texts(&entries),
        [
            line,
            "---",
            first,
            "Connect «Casa»",
            "---",
            "Open Galleria",
            quit,
        ]
    );
    assert_eq!(shown, icon);
}

#[rstest]
#[case(LinkState::Connecting)]
#[case(LinkState::DaemonNotRunning)]
#[case(LinkState::PermissionDenied)]
#[case(LinkState::IncompatibleProtocol)]
fn without_the_link_nothing_can_be_connected(#[case] link: LinkState) {
    // a status that is left over from before the link went: it is not shown
    let stale = status(State::Connected, Some((id(1), "Ufficio")));
    let (entries, icon) = build(link, &stale, &favourites());
    assert_eq!(
        texts(&entries),
        [
            "[Service not reachable]",
            "---",
            "(Connect «Ufficio»)",
            "(Connect «Casa»)",
            "---",
            "Open Galleria",
            "Quit"
        ]
    );
    assert_eq!(icon, TrayIcon::Off);
}

#[test]
fn favourites_keep_the_order_of_the_list_and_stop_at_eight() {
    let many: Vec<(Uuid, String)> = (1..=12).map(|n| (id(n), format!("P{n}"))).collect();
    let (entries, _) = build(LinkState::Connected, &Status::default(), &many);
    let connects: Vec<String> = texts(&entries)
        .into_iter()
        .filter(|t| t.starts_with("Connect"))
        .collect();
    assert_eq!(MAX_FAVOURITES, 8);
    assert_eq!(
        connects,
        (1..=8)
            .map(|n| format!("Connect «P{n}»"))
            .collect::<Vec<_>>()
    );
}

#[test]
fn the_active_favourite_offers_disconnect_and_the_others_connect() {
    let status = status(State::Connected, Some((id(2), "Casa")));
    let (entries, _) = build(LinkState::Connected, &status, &favourites());
    let actions: Vec<TrayAction> = entries
        .iter()
        .filter_map(|e| match e {
            TrayEntry::Item { action, .. } => Some(action.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(
        actions,
        [
            TrayAction::Connect(id(1)),
            TrayAction::Disconnect,
            TrayAction::Open,
            TrayAction::Quit
        ]
    );
}

#[rstest]
#[case::not_a_favourite(Some((id(9), "Fuori")))]
#[case::a_foreign_session_has_its_own_name(Some((id(77), "Sessione di altri")))]
fn an_active_profile_that_is_not_a_favourite_gets_its_own_row(
    #[case] profile: Option<(Uuid, &str)>,
) {
    let status = status(State::Connected, profile);
    let (entries, _) = build(LinkState::Connected, &status, &favourites());
    let name = profile.unwrap().1;
    assert_eq!(
        texts(&entries)[2..5],
        [
            "Connect «Ufficio»".to_owned(),
            "Connect «Casa»".to_owned(),
            format!("Disconnect «{name}»")
        ]
    );
}

#[test]
fn a_foreign_session_gets_a_disconnect_row_with_its_name() {
    // the name comes from the status, not from the rubrica: the rubrica does not have this profile
    let status = status(State::Connected, Some((id(500), "Rete di un collega")));
    let (entries, _) = build(LinkState::Connected, &status, &[]);
    assert_eq!(
        texts(&entries),
        [
            "[Connected to «Rete di un collega»]",
            "---",
            "Disconnect «Rete di un collega»",
            "---",
            "Open Galleria",
            "Quit (the VPN stays on)"
        ]
    );
}

#[rstest]
#[case(LinkState::Connected, State::Disconnected)]
#[case(LinkState::Connected, State::Connected)]
#[case(LinkState::Connecting, State::Disconnected)]
#[case(LinkState::DaemonNotRunning, State::Connected)]
fn open_and_quit_are_always_there_and_enabled(#[case] link: LinkState, #[case] state: State) {
    let (entries, _) = build(
        link,
        &status(state, Some((id(1), "Ufficio"))),
        &favourites(),
    );
    let last_two = &entries[entries.len() - 2..];
    assert!(matches!(
        &last_two[0],
        TrayEntry::Item {
            enabled: true,
            action: TrayAction::Open,
            ..
        }
    ));
    assert!(matches!(
        &last_two[1],
        TrayEntry::Item {
            enabled: true,
            action: TrayAction::Quit,
            ..
        }
    ));
}

#[test]
fn quit_says_the_vpn_stays_while_a_session_is_up() {
    let up = status(State::Connected, Some((id(1), "Ufficio")));
    let quit = |s: &Status| texts(&build(LinkState::Connected, s, &[]).0).pop().unwrap();
    assert_eq!(quit(&up), "Quit (the VPN stays on)");
    assert_eq!(quit(&Status::default()), "Quit");
}

#[test]
fn a_name_with_markup_or_newlines_is_shown_flat() {
    let names = vec![(id(1), "Ufficio\n& _casa_ <b>x</b>\r\nsecondo".to_owned())];
    let (entries, _) = build(LinkState::Connected, &Status::default(), &names);
    let TrayEntry::Item { text, .. } = &entries[2] else {
        panic!("{entries:?}")
    };
    assert_eq!(text, "Connect «Ufficio & _casa_ <b>x</b> secondo»");
    assert!(!text.contains(['\n', '\r']));
    // for the menu of Tauri `&` would be the mark of a shortcut: it is doubled, so it is shown as it is
    assert_eq!(
        literal(text),
        "Connect «Ufficio && _casa_ <b>x</b> secondo»"
    );
    assert_eq!(literal("Salva e chiudi"), "Salva e chiudi");
}

#[test]
fn labels_longer_than_two_hundred_characters_are_refused() {
    assert_eq!(LABEL_MAX, 200);
    assert!(labels().checked().is_ok());
    let mut long = labels();
    long.open = "x".repeat(201);
    assert!(long.checked().is_err());
    let mut limit = labels();
    limit.open = "x".repeat(200);
    assert!(limit.checked().is_ok());
    let mut empty = labels();
    empty.quit = " \n ".into();
    assert!(empty.checked().is_err());
    // a text that arrives with a line break is one line
    let mut broken = labels();
    broken.open = "Open\nGalleria".into();
    assert_eq!(broken.checked().unwrap().open, "Open Galleria");
}

// ---- the tray that keeps the model --------------------------------------------------------------------------------------

type Shown = (Vec<TrayEntry>, TrayIcon, u64);

#[derive(Clone, Default)]
struct Recorded(Arc<Mutex<Vec<Shown>>>);

struct Recording(Recorded);

impl Surface for Recording {
    fn show(&mut self, entries: &[TrayEntry], icon: TrayIcon, generation: u64) {
        self.0
            .0
            .lock()
            .unwrap()
            .push((entries.to_vec(), icon, generation));
    }
}

fn recording_tray() -> (Tray, Recorded, Arc<Mutex<u32>>) {
    let recorded = Recorded::default();
    let made = Arc::new(Mutex::new(0));
    let (shared, count) = (recorded.clone(), Arc::clone(&made));
    let tray = Tray::new(Box::new(move || {
        *count.lock().unwrap() += 1;
        Some(Box::new(Recording(shared.clone())))
    }));
    (tray, recorded, made)
}

fn linked(status: Status) -> DaemonView {
    DaemonView {
        link: LinkState::Connected,
        daemon_version: None,
        incompatible: None,
        backends: vec![],
        status: Some(status),
    }
}

#[test]
fn no_labels_no_tray() {
    let (tray, recorded, made) = recording_tray();
    tray.set_favourites(favourites());
    tray.set_link(&linked(Status::default()));
    tray.set_status(&status(State::Connected, Some((id(1), "Ufficio"))));
    assert_eq!(
        *made.lock().unwrap(),
        0,
        "the tray was made before the texts came"
    );
    assert!(!tray.available());
    assert!(recorded.0.lock().unwrap().is_empty());
    tray.set_labels(labels());
    assert_eq!(*made.lock().unwrap(), 1);
    assert!(tray.available());
    // what happened before the texts is in the first menu
    let first = recorded.0.lock().unwrap();
    assert_eq!(first.len(), 1);
    assert_eq!(texts(&first[0].0)[0], "[Connected to «Ufficio»]");
}

#[test]
fn the_surface_is_made_once_and_the_menu_is_written_again_only_when_it_changed() {
    let (tray, recorded, made) = recording_tray();
    tray.set_labels(labels());
    tray.set_link(&linked(Status::default()));
    let after_link = recorded.0.lock().unwrap().len();
    // the same things again: nothing is written
    tray.set_link(&linked(Status::default()));
    tray.set_status(&Status::default());
    tray.set_favourites(vec![]);
    tray.set_labels(labels());
    assert_eq!(recorded.0.lock().unwrap().len(), after_link);
    assert_eq!(*made.lock().unwrap(), 1);
    // a different state is written, with a number that is higher
    tray.set_status(&status(State::Connected, Some((id(1), "Ufficio"))));
    let written = recorded.0.lock().unwrap();
    assert_eq!(written.len(), after_link + 1);
    assert!(written[after_link].2 > written[after_link - 1].2);
}

#[test]
fn a_tray_that_cannot_be_made_is_not_tried_again_and_nothing_breaks() {
    let tries = Arc::new(Mutex::new(0));
    let count = Arc::clone(&tries);
    let tray = Tray::new(Box::new(move || {
        *count.lock().unwrap() += 1;
        None
    }));
    tray.set_labels(labels());
    tray.set_labels(labels());
    tray.set_status(&Status::default());
    assert_eq!(*tries.lock().unwrap(), 1);
    assert!(!tray.available());
    assert_eq!(tray.action(1, 0), None);
}

#[test]
fn a_click_asks_for_what_the_entry_of_that_menu_says() {
    let (tray, recorded, _) = recording_tray();
    tray.set_favourites(favourites());
    tray.set_link(&linked(Status::default()));
    tray.set_labels(labels());
    let (entries, generation) = {
        let written = recorded.0.lock().unwrap();
        let last = written.last().unwrap();
        (last.0.clone(), last.2)
    };
    let at = |action: &TrayAction| {
        entries
            .iter()
            .position(|e| matches!(e, TrayEntry::Item { action: a, .. } if a == action))
            .unwrap()
    };
    assert_eq!(
        tray.action(generation, at(&TrayAction::Connect(id(2)))),
        Some(TrayAction::Connect(id(2)))
    );
    assert_eq!(
        tray.action(generation, at(&TrayAction::Quit)),
        Some(TrayAction::Quit)
    );
    // the line of the state and a separator are not choices
    assert_eq!(tray.action(generation, 0), None);
    assert_eq!(tray.action(generation, 1), None);
    // a click on a menu that was written again since asks for nothing
    tray.set_status(&status(State::Connected, Some((id(1), "Ufficio"))));
    assert_eq!(tray.action(generation, at(&TrayAction::Quit)), None);
}

// ---- a click on the menu (plan B6, T2): `Connect` and `Disconnect` go to the page, `Open` and `Quit` are Rust's ---------------

/// What the page is asked to do, as it would hear it.
fn hear(world: &World) -> Arc<Mutex<Vec<Value>>> {
    let heard = Arc::new(Mutex::new(vec![]));
    let sink = Arc::clone(&heard);
    world.app().listen(EV_TRAY_ACTION, move |event| {
        sink.lock()
            .unwrap()
            .push(serde_json::from_str(event.payload()).unwrap());
    });
    heard
}

fn wait_for(what: &str, ready: impl Fn() -> bool) {
    let begun = Instant::now();
    while !ready() {
        assert!(
            begun.elapsed() < Duration::from_secs(8),
            "never happened: {what}"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn a_click_on_connect_reaches_the_page_as_an_action_with_the_id() {
    let world = start(Options::default());
    let heard = hear(&world);
    let homelab = world.id("Homelab");
    run_action(world.app(), &TrayAction::Connect(homelab));
    wait_for("the action", || heard.lock().unwrap().len() == 1);
    assert_eq!(
        heard.lock().unwrap()[0],
        json!({ "action": "connect", "id": homelab.to_string() })
    );
    run_action(world.app(), &TrayAction::Disconnect);
    wait_for("the second action", || heard.lock().unwrap().len() == 2);
    assert_eq!(heard.lock().unwrap()[1], json!({ "action": "disconnect" }));
    // Rust did not connect anything on its own: the session is the page's to ask for
    std::thread::sleep(Duration::from_millis(100));
    assert!(world.daemon().harness.connects.lock().unwrap().is_empty());
}

/// A window that says whether it is hidden and counts how many times it was asked to show and to quit.
struct TestWindow {
    hidden: AtomicBool,
    shown: AtomicUsize,
    quit: AtomicUsize,
}

impl TestWindow {
    fn new(hidden: bool) -> Arc<Self> {
        Arc::new(Self {
            hidden: AtomicBool::new(hidden),
            shown: AtomicUsize::new(0),
            quit: AtomicUsize::new(0),
        })
    }

    fn shown(&self) -> usize {
        self.shown.load(Ordering::SeqCst)
    }
}

impl WindowControl for TestWindow {
    fn show(&self) {
        self.shown.fetch_add(1, Ordering::SeqCst);
        self.hidden.store(false, Ordering::SeqCst);
    }

    fn is_hidden(&self) -> bool {
        self.hidden.load(Ordering::SeqCst)
    }

    fn quit(&self) {
        self.quit.fetch_add(1, Ordering::SeqCst);
    }
}

fn start_with(window: &Arc<TestWindow>, scenario: &'static str) -> World {
    let control: Arc<dyn WindowControl> = Arc::clone(window) as Arc<dyn WindowControl>;
    start(Options {
        scenario,
        window: Some(control),
        ..Options::default()
    })
}

#[test]
fn open_and_quit_never_reach_the_page() {
    let window = TestWindow::new(true);
    let world = start_with(&window, "plain");
    let heard = hear(&world);
    run_action(world.app(), &TrayAction::Open);
    assert_eq!(window.shown(), 1, "«Open» shows the window");
    run_action(world.app(), &TrayAction::Quit);
    assert_eq!(
        window.quit.load(Ordering::SeqCst),
        1,
        "«Quit» asks to end the app"
    );
    std::thread::sleep(Duration::from_millis(150));
    assert!(
        heard.lock().unwrap().is_empty(),
        "{:?}",
        heard.lock().unwrap()
    );
}

#[test]
fn no_tray_action_carries_more_than_an_action_and_an_id() {
    let id = Uuid::from_u128(7);
    assert_eq!(
        serde_json::to_value(ActionPayload::Connect { id }).unwrap(),
        json!({ "action": "connect", "id": id.to_string() })
    );
    assert_eq!(
        serde_json::to_value(ActionPayload::Disconnect).unwrap(),
        json!({ "action": "disconnect" })
    );
}

#[test]
fn needs_input_shows_the_hidden_window() {
    let window = TestWindow::new(true);
    let world = start_with(&window, "credentials");
    world.connect("Datacenter Milano", false).unwrap();
    world.until_state("needs_input", None);
    wait_for("the hidden window to show on the question", || {
        window.shown() == 1
    });
}

#[test]
fn a_question_leaves_a_window_that_is_shown_alone() {
    let window = TestWindow::new(false);
    let world = start_with(&window, "credentials");
    world.connect("Datacenter Milano", false).unwrap();
    world.until_state("needs_input", None);
    std::thread::sleep(Duration::from_millis(150));
    assert_eq!(
        window.shown(),
        0,
        "the focus of a window that is open is not taken"
    );
}

#[test]
fn window_show_is_a_command_with_no_parameter() {
    let window = TestWindow::new(true);
    let world = start_with(&window, "plain");
    assert_eq!(world.invoke("window_show", json!({})).unwrap(), Value::Null);
    assert_eq!(window.shown(), 1);
}
