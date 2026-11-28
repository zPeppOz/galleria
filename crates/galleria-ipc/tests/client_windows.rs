#![cfg(all(feature = "client", windows))]
#![allow(clippy::unwrap_used, clippy::expect_used)]
//! P1: Windows has no transport yet, so the client says so instead of pretending. P2 replaces this file.
use galleria_ipc::client::{Client, ClientError};
use std::{io::ErrorKind, path::Path};

#[tokio::test]
async fn the_client_says_unsupported_without_a_transport() {
    let Err(error) = Client::connect(Path::new(r"\\.\pipe\galleria")).await else {
        panic!("connect must fail on Windows until the transport exists");
    };
    assert!(
        matches!(&error, ClientError::Io(e) if e.kind() == ErrorKind::Unsupported),
        "{error:?}"
    );
}
