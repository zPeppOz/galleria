#![cfg(unix)]
//! The session of someone else (plan B5, T8, D21): a session whose profile the rubrica does not have is foreign, and the window is told
//! so by Rust, which looks at the rubrica when the daemon answers. Nothing here sends the replacement on its own.
#![allow(clippy::unwrap_used, clippy::expect_used)]
#[path = "support/seed.rs"]
mod seed;
mod support;
#[path = "support/world.rs"]
mod world;

use galleria_profile::ProfileStore;
use serde_json::{Value, json};
use std::time::Duration;
use support::*;
use world::*;

#[test]
fn a_session_of_a_deleted_profile_becomes_foreign() {
    let world = start(Options::default());
    let homelab = world.id("Homelab");
    world.connect("Homelab", false).unwrap();
    world.until_state("connected", Some(homelab));
    // while it is its own, a second connection is told so
    let own = world.connect("VPS Hetzner", false).unwrap_err();
    assert_eq!(own["data"]["foreign"], false);

    // the profile is deleted from outside (the CLI), with the session up: the rubrica no longer has it
    ProfileStore::open(&world.secrets_dir())
        .unwrap()
        .delete(&homelab)
        .unwrap();
    let foreign = world.connect("VPS Hetzner", false).unwrap_err();
    assert_eq!(code(&foreign), "already_active");
    assert_eq!(foreign["data"]["foreign"], true);
    assert_eq!(foreign["data"]["profile"]["name"], "Homelab");
    assert_eq!(
        foreign["data"]["profile"]["id"],
        homelab.to_string().as_str()
    );
    // and Rust still replaces nothing by itself: the session is up
    std::thread::sleep(Duration::from_millis(200));
    assert_eq!(world.states().last().unwrap(), "connected");
    let log = world.daemon().harness.net.log.lock().unwrap().clone();
    assert!(!log.iter().any(|line| line == "stop"), "{log:?}");
    world.stop();
}

#[test]
fn a_foreign_session_is_replaced_only_when_the_page_asks_again_with_replace() {
    let world = start(Options::default());
    world.rt.block_on(async {
        let mut other = world.daemon().client().await;
        let _: Value = other.call("connect", &params(false)).await.unwrap();
    });
    world.until_state("connected", None);
    let first = world.connect("Homelab", false).unwrap_err();
    assert_eq!(first["data"]["foreign"], true);
    assert_eq!(
        world
            .daemon()
            .harness
            .spawns
            .load(std::sync::atomic::Ordering::SeqCst),
        1
    );
    // the page asked the person and the person said yes: one more call, with `replace`
    world.connect("Homelab", true).unwrap();
    world.until_state("connected", Some(world.id("Homelab")));
    assert_eq!(
        world
            .daemon()
            .harness
            .spawns
            .load(std::sync::atomic::Ordering::SeqCst),
        2
    );
    world.stop();
}

#[test]
fn a_foreign_session_can_be_closed_from_the_window() {
    let world = start(Options::default());
    world.rt.block_on(async {
        let mut other = world.daemon().client().await;
        let _: Value = other.call("connect", &params(false)).await.unwrap();
    });
    world.until_state("connected", None);
    // the question is the page's to ask; once it was answered, `disconnect` ends whatever session there is
    assert_eq!(world.invoke("disconnect", json!({})).unwrap(), Value::Null);
    world.until_state("disconnected", None);
    world.stop();
}

#[test]
fn deleting_the_active_profile_from_the_window_is_still_refused() {
    let world = start(Options::default());
    let homelab = world.id("Homelab");
    world.connect("Homelab", false).unwrap();
    world.until_state("connected", Some(homelab));
    let error = world
        .invoke("profile_delete", json!({ "id": homelab.to_string() }))
        .unwrap_err();
    assert_eq!(code(&error), "gui_profile_in_use");
    assert!(
        world
            .secrets_dir()
            .join(format!("profiles/{homelab}.toml"))
            .exists()
    );
    world.stop();
}
