#![cfg(unix)]
//! The log of the session and «Export log…» (plan B5, T7): the whole buffer of the daemon is read in pages, written where the window
//! «Save» says and nowhere else, and neither a path nor a known secret reaches the page. The window «Save» is a `SavePicker` that
//! the tests replace.
#![allow(clippy::unwrap_used, clippy::expect_used)]
#[path = "support/seed.rs"]
mod seed;
mod support;
#[path = "support/world.rs"]
mod world;

use galleria_gui::{
    export::SavePicker,
    logs::{PAGE, all, all_with, render, suggested_name},
};
use galleria_ipc::{LogEntry, LogLevel, LogSource, LogsResult};
use serde_json::{Value, json};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    sync::{Arc, Mutex},
};
use time::{Date, Month, OffsetDateTime, Time};
use world::*;

/// A window «Save» that answers what the test says and remembers the names it was offered.
struct FakePicker {
    answer: Option<PathBuf>,
    offered: Mutex<Vec<String>>,
}

impl FakePicker {
    fn new(answer: Option<PathBuf>) -> Arc<Self> {
        Arc::new(Self {
            answer,
            offered: Mutex::new(vec![]),
        })
    }
}

impl SavePicker for FakePicker {
    fn pick(&self, suggested_name: &str) -> Option<PathBuf> {
        self.offered.lock().unwrap().push(suggested_name.to_owned());
        self.answer.clone()
    }
}

fn entry(seq: u64) -> LogEntry {
    LogEntry {
        seq,
        ts: format!("{}", 1_795_000_000_000 + seq),
        level: LogLevel::Info,
        source: LogSource::Daemon,
        message: format!("line {seq}"),
    }
}

/// A buffer of `total` lines, as the daemon serves it: oldest first, at most 1000 for a call, `last` the number of the last one.
fn page_of(total: u64, after: Option<u64>, limit: usize) -> LogsResult {
    let entries: Vec<LogEntry> = (1..=total)
        .filter(|seq| after.is_none_or(|a| *seq > a))
        .take(limit.min(1000))
        .map(entry)
        .collect();
    let last = entries.last().map_or(after.unwrap_or(total), |e| e.seq);
    LogsResult { entries, last }
}

#[test]
fn all_pages_until_a_short_page() {
    // 2000 lines: two full pages and one that is empty, and every line once
    let calls = Mutex::new(vec![]);
    let rt = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    let got = rt
        .block_on(all_with(|after, limit| {
            calls.lock().unwrap().push((after, limit));
            let page = page_of(2000, after, limit);
            async move { Ok(page) }
        }))
        .unwrap();
    assert_eq!(
        *calls.lock().unwrap(),
        [(None, PAGE), (Some(1000), PAGE), (Some(2000), PAGE)]
    );
    assert_eq!(got.len(), 2000);
    assert!(got.iter().zip(1..).all(|(e, n)| e.seq == n));

    // 1500 lines: one full page and one short
    let calls = Mutex::new(vec![]);
    let got = rt
        .block_on(all_with(|after, limit| {
            calls.lock().unwrap().push(after);
            let page = page_of(1500, after, limit);
            async move { Ok(page) }
        }))
        .unwrap();
    assert_eq!(*calls.lock().unwrap(), [None, Some(1000)]);
    assert_eq!(got.len(), 1500);

    // nothing in the buffer: one call, no line
    let got = rt
        .block_on(all_with(|after, limit| {
            let page = page_of(0, after, limit);
            async move { Ok(page) }
        }))
        .unwrap();
    assert!(got.is_empty());
}

#[test]
fn a_full_buffer_of_the_real_daemon_comes_whole_with_no_line_twice_or_lost() {
    let world = start(Options {
        scenario: "chatty",
        ..Options::default()
    });
    world.connect("Homelab", false).unwrap();
    world.until_state("connected", None);
    let lines = world.rt.block_on(all(&world.daemon().socket)).unwrap();
    assert!(
        lines.len() >= 2000,
        "the buffer holds 2000 lines, saw {}",
        lines.len()
    );
    // one run of numbers, each once: what the daemon evicted is at the start, and nothing is missing in the middle
    for pair in lines.windows(2) {
        assert_eq!(
            pair[1].seq,
            pair[0].seq + 1,
            "a line was lost or came twice"
        );
    }
    assert!(lines.iter().any(|e| e.message == "line 2100"));
    world.stop();
}

#[test]
fn logs_get_answers_a_page_of_the_daemon() {
    let world = start(Options::default());
    world.connect("Homelab", false).unwrap();
    world.until_state("connected", None);
    let page = world
        .invoke("logs_get", json!({ "after": null, "limit": 1000 }))
        .unwrap();
    let entries = page["entries"].as_array().unwrap();
    assert!(!entries.is_empty());
    assert_eq!(page["last"], entries.last().unwrap()["seq"]);
    let first = entries[0]["seq"].as_u64().unwrap();
    let rest = world
        .invoke("logs_get", json!({ "after": first, "limit": 1000 }))
        .unwrap();
    assert!(
        rest["entries"]
            .as_array()
            .unwrap()
            .iter()
            .all(|e| e["seq"].as_u64().unwrap() > first)
    );
    world.stop();
}

#[test]
fn logs_get_refuses_more_than_a_thousand() {
    // no daemon at all: a refusal that names the field cannot have come from a call
    let world = start(Options {
        daemon: false,
        ..Options::default()
    });
    for limit in [1001, 5000, 0] {
        let error = world
            .invoke("logs_get", json!({ "after": null, "limit": limit }))
            .unwrap_err();
        assert_eq!(code(&error), "gui_invalid_field", "limit {limit}");
        assert_eq!(error["data"]["fields"][0]["field"], "limit");
    }
    let unreachable = world
        .invoke("logs_get", json!({ "after": null, "limit": 1000 }))
        .unwrap_err();
    assert_eq!(code(&unreachable), "gui_daemon_unreachable");
}

#[test]
fn export_writes_what_logs_returned() {
    let out = tempfile::tempdir().unwrap();
    let target = out.path().join("saved.txt");
    let picker = FakePicker::new(Some(target.clone()));
    let world = start(Options {
        picker: Some(picker.clone()),
        ..Options::default()
    });
    world.connect("Homelab", false).unwrap();
    world.until_state("connected", None);
    let before = world.rt.block_on(all(&world.daemon().socket)).unwrap();

    let answer = world.invoke("logs_export", json!({})).unwrap();
    assert_eq!(answer, json!({ "saved": true }));

    let written = fs::read_to_string(&target).unwrap();
    // the buffer only grows (the calls of this test are lines of it too): what was read before is the start of the file
    assert!(
        written.starts_with(&render(&before)),
        "the file is not what the daemon had"
    );
    assert!(written.ends_with('\n'));
    assert_eq!(
        fs::metadata(&target).unwrap().permissions().mode() & 0o777,
        0o600
    );
    // the name offered is the one of P12
    let offered = picker.offered.lock().unwrap();
    assert_eq!(offered.len(), 1);
    assert!(
        offered[0].starts_with("galleria-log-") && offered[0].ends_with(".txt"),
        "{}",
        offered[0]
    );
    world.stop();
}

#[test]
fn export_cancelled_writes_nothing() {
    let out = tempfile::tempdir().unwrap();
    let picker = FakePicker::new(None);
    let world = start(Options {
        picker: Some(picker.clone()),
        ..Options::default()
    });
    world.connect("Homelab", false).unwrap();
    world.until_state("connected", None);
    let answer = world.invoke("logs_export", json!({})).unwrap();
    assert_eq!(answer, json!({ "saved": false }));
    assert_eq!(
        picker.offered.lock().unwrap().len(),
        1,
        "the window was asked"
    );
    assert_eq!(fs::read_dir(out.path()).unwrap().count(), 0);
    world.stop();
}

#[test]
fn the_export_never_carries_a_known_secret() {
    let out = tempfile::tempdir().unwrap();
    let target = out.path().join("leak.txt");
    let world = start(Options {
        scenario: "leaky",
        picker: Some(FakePicker::new(Some(target.clone()))),
        ..Options::default()
    });
    world.connect("Homelab", false).unwrap();
    world.until_state("connected", None);
    world.invoke("logs_export", json!({})).unwrap();
    let written = fs::read_to_string(&target).unwrap();
    assert!(
        written.contains("handshake with key [redatto] done"),
        "the lines of the backend are in the file, with the keys redacted: {written}"
    );
    assert!(
        !written.contains(seed::SECRET_MARK),
        "a secret is in the file: {written}"
    );
    // nor in what the page was given as a page of the log, or as an event
    let page = world
        .invoke("logs_get", json!({ "after": null, "limit": 1000 }))
        .unwrap();
    assert!(!page.to_string().contains(seed::SECRET_MARK));
    let events = world.events.lock().unwrap();
    assert!(
        events
            .iter()
            .all(|(_, value)| !value.to_string().contains(seed::SECRET_MARK))
    );
    drop(events);
    world.stop();
}

#[test]
fn the_answer_holds_no_path() {
    let out = tempfile::tempdir().unwrap();
    let target = out.path().join("where-it-went.txt");
    let world = start(Options {
        picker: Some(FakePicker::new(Some(target.clone()))),
        ..Options::default()
    });
    let answer: Value = world.invoke("logs_export", json!({})).unwrap();
    let text = answer.to_string();
    assert_eq!(answer, json!({ "saved": true }));
    assert!(!text.contains(out.path().to_str().unwrap()));
    assert!(!text.contains("where-it-went"));
    world.stop();
}

#[test]
fn a_place_that_cannot_be_written_is_an_error_with_the_place_and_no_text() {
    let out = tempfile::tempdir().unwrap();
    let target = out.path().join("no-such-folder/log.txt");
    let world = start(Options {
        picker: Some(FakePicker::new(Some(target))),
        ..Options::default()
    });
    let error = world.invoke("logs_export", json!({})).unwrap_err();
    assert_eq!(code(&error), "gui_export_failed");
    world.stop();
}

#[test]
fn render_is_the_line_of_galleria_logs() {
    let lines = [
        entry(1),
        LogEntry {
            level: LogLevel::Warn,
            source: LogSource::Wireguard,
            message: "handshake did not complete".into(),
            ..entry(2)
        },
    ];
    assert_eq!(
        render(&lines),
        "1795000000001 Info Daemon: line 1\n1795000000002 Warn Wireguard: handshake did not complete\n"
    );
    assert_eq!(render(&[]), "");
}

#[test]
fn the_name_offered_has_the_day_and_the_minute() {
    let now = OffsetDateTime::new_utc(
        Date::from_calendar_date(2026, Month::November, 26).unwrap(),
        Time::from_hms(7, 5, 59).unwrap(),
    );
    assert_eq!(suggested_name(now), "galleria-log-20261126-0705.txt");
}
