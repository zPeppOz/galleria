//! One window and the files that are handed to it (plan B4, T5, Q36 (b)): the arguments, the lock that dies with the process,
//! the private socket, and a channel that closes whatever is not one line of paths from the same user. No window, no
//! WebKit: the first process is a thread with its own runtime, and the second is `claim` in the test.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use galleria_gui::{
    import::{ImportOffer, Imports, Origin},
    instance::{
        Claim, MAX_FILES, MAX_MESSAGE_BYTES, PEER_TIMEOUT, Primary, claim, files_in_args,
        runtime_dir,
    },
    rubrica::Rubrica,
    ui_state::UiStateFile,
};
use rstest::rstest;
use std::{
    ffi::OsString,
    fs,
    io::{BufRead, BufReader, Read, Write},
    os::unix::{
        ffi::OsStringExt,
        fs::{FileTypeExt, PermissionsExt},
        net::UnixStream,
    },
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

fn euid() -> u32 {
    nix::unistd::geteuid().as_raw()
}

fn mode(path: &Path) -> u32 {
    fs::symlink_metadata(path).unwrap().permissions().mode() & 0o777
}

/// A runtime folder of the test: `$XDG_RUNTIME_DIR/galleria`, which is not made yet.
fn channel(root: &tempfile::TempDir) -> PathBuf {
    root.path().join("run").join("galleria")
}

fn primary_of(claimed: Claim) -> Primary {
    match claimed {
        Claim::Primary(primary) => primary,
        Claim::Handed => panic!("expected the first process, the files were handed over"),
        Claim::Alone(why) => panic!("expected the first process, it is alone: {why}"),
        Claim::Failed(why) => panic!("expected the first process, it failed: {why}"),
    }
}

/// The first process of a test: it serves in a thread of its own and records every list of files it is given. Dropping
/// it stops the service and lets go of the lock and the socket.
struct Running {
    received: Arc<Mutex<Vec<Vec<PathBuf>>>>,
    stop: Option<tokio::sync::oneshot::Sender<()>>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl Running {
    /// `uid`: serve the connections of that user, instead of the user of the test.
    fn start(primary: Primary, uid: Option<u32>) -> Self {
        let received: Arc<Mutex<Vec<Vec<PathBuf>>>> = Arc::default();
        let sink = Arc::clone(&received);
        let (stop, stopped) = tokio::sync::oneshot::channel();
        let thread = std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            runtime.block_on(async {
                let on_files = move |files| sink.lock().unwrap().push(files);
                tokio::select! {
                    () = primary.serve_for(uid.unwrap_or_else(euid), on_files) => {}
                    _ = stopped => {}
                }
            });
        });
        Self {
            received,
            stop: Some(stop),
            thread: Some(thread),
        }
    }

    fn files(&self) -> Vec<Vec<PathBuf>> {
        self.received.lock().unwrap().clone()
    }
}

impl Drop for Running {
    fn drop(&mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn paths(list: &[&str]) -> Vec<PathBuf> {
    list.iter().map(PathBuf::from).collect()
}

/// What the answer to a message is: `Some(line)` or `None` when the connection was closed (or reset) without a line.
fn answer_of(stream: &mut UnixStream) -> Option<String> {
    let mut line = String::new();
    match BufReader::new(stream).read_line(&mut line) {
        Ok(n) if n > 0 => Some(line),
        _ => None,
    }
}

fn say(socket: &Path, message: &[u8]) -> Option<String> {
    let mut stream = UnixStream::connect(socket).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    // the server may close before everything is written: that is the point of some of the messages
    let _ = stream.write_all(message);
    answer_of(&mut stream)
}

// ---- the arguments --------------------------------------------------------------------------------------------------

#[rstest]
#[case::a_flag(&["--flag"], &[])]
#[case::a_single_dash(&["-", ""], &[])]
#[case::a_relative_path(&["a.ovpn"], &["/home/u/a.ovpn"])]
#[case::an_absolute_path(&["/x/b.ovpn"], &["/x/b.ovpn"])]
#[case::a_file_url(&["file:///x/c%20d.ovpn"], &["/x/c d.ovpn"])]
#[case::a_file_url_of_this_host(&["file://localhost/x/e.ovpn"], &["/x/e.ovpn"])]
#[case::a_file_url_of_another_host(&["file://elsewhere/x/e.ovpn"], &[])]
#[case::a_url_that_is_not_a_file(&["https://example.org/x.ovpn", "ftp://h/x.ovpn"], &[])]
#[case::a_bad_escape(&["file:///x/%zz.ovpn", "file:///x/%2"], &[])]
#[case::a_nul_in_a_path(&["file:///x/a%00b.ovpn"], &[])]
#[case::a_query_is_not_a_path(&["file:///x/f.ovpn?x=1#top"], &["/x/f.ovpn"])]
#[case::a_colon_is_not_a_scheme(&["a:b.ovpn"], &["/home/u/a:b.ovpn"])]
#[case::the_order_stays(&["--x", "b.conf", "/y/a.ovpn", "https://z/q"], &["/home/u/b.conf", "/y/a.ovpn"])]
fn files_in_args_skips_flags_and_takes_paths_and_file_urls(
    #[case] args: &[&str],
    #[case] expected: &[&str],
) {
    let got = files_in_args(args.iter().map(OsString::from), Path::new("/home/u"));
    assert_eq!(got, paths(expected));
}

#[test]
fn an_argument_that_is_not_utf8_is_skipped() {
    let args = vec![
        OsString::from_vec(vec![b'/', b'x', b'/', 0xff, b'.', b'o']),
        OsString::from("/x/ok.ovpn"),
    ];
    assert_eq!(
        files_in_args(args, Path::new("/home/u")),
        paths(&["/x/ok.ovpn"])
    );
}

#[test]
fn the_runtime_folder_is_galleria_inside_an_absolute_runtime_dir() {
    assert_eq!(
        runtime_dir(Some(OsString::from("/run/user/1000"))),
        Some(PathBuf::from("/run/user/1000/galleria"))
    );
    assert_eq!(runtime_dir(None), None);
    assert_eq!(runtime_dir(Some(OsString::new())), None);
    assert_eq!(runtime_dir(Some(OsString::from("run/user"))), None);
}

// ---- who is the first -----------------------------------------------------------------------------------------------

#[test]
fn the_first_claim_is_primary_and_the_second_hands_its_files() {
    let root = tempfile::tempdir().unwrap();
    let dir = channel(&root);
    let first = Running::start(primary_of(claim(Some(&dir), &[])), None);
    let files = paths(&["/home/u/a.ovpn", "/home/u/b conf.ovpn"]);
    assert!(matches!(claim(Some(&dir), &files), Claim::Handed));
    // the first received them, absolute, before the second was told «ok»
    assert_eq!(first.files(), vec![files]);
}

#[test]
fn a_second_launch_without_files_only_raises_the_window() {
    let root = tempfile::tempdir().unwrap();
    let dir = channel(&root);
    let first = Running::start(primary_of(claim(Some(&dir), &[])), None);
    assert!(matches!(claim(Some(&dir), &[]), Claim::Handed));
    assert_eq!(first.files(), vec![Vec::<PathBuf>::new()]);
}

#[test]
fn without_a_runtime_dir_the_app_starts_alone() {
    // S11: no runtime folder, no channel, and the app works anyway
    assert!(matches!(
        claim(None, &paths(&["/x/a.ovpn"])),
        Claim::Alone(_)
    ));
    // a folder that cannot be made (a file is in the way) is the same
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("run"), b"not a folder").unwrap();
    let dir = channel(&root);
    assert!(matches!(claim(Some(&dir), &[]), Claim::Alone(_)));
}

#[test]
fn the_lock_dies_with_the_process() {
    let root = tempfile::tempdir().unwrap();
    let dir = channel(&root);
    // a process of its own takes the lock (this same test binary, one test, told by the environment)
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "lock_holder_child",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("GALLERIA_TEST_LOCK_DIR", &dir)
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut lines = BufReader::new(child.stdout.take().unwrap()).lines();
    loop {
        let line = lines
            .next()
            .expect("the child ended before it held the lock")
            .unwrap();
        // libtest writes «test name ... » before the output of the test, on the same line
        if line.trim().ends_with("held") {
            break;
        }
    }
    // while it lives, nobody else is the first, and a second launch finds the socket of the child
    assert!(
        !matches!(claim(Some(&dir), &[]), Claim::Primary(_)),
        "the lock was not held by the child"
    );
    child.kill().unwrap();
    child.wait().unwrap();
    // killed, not stopped: the socket is still on the disk, and the lock is free
    assert!(
        fs::symlink_metadata(dir.join("gui.sock"))
            .unwrap()
            .file_type()
            .is_socket()
    );
    let first = Running::start(primary_of(claim(Some(&dir), &[])), None);
    // and the socket left behind was replaced by one that answers
    assert!(matches!(
        claim(Some(&dir), &paths(&["/x/a.ovpn"])),
        Claim::Handed
    ));
    assert_eq!(first.files(), vec![paths(&["/x/a.ovpn"])]);
}

/// The process that holds the lock for `the_lock_dies_with_the_process`: a test that does nothing unless it is told to.
#[test]
fn lock_holder_child() {
    let Some(dir) = std::env::var_os("GALLERIA_TEST_LOCK_DIR") else {
        return;
    };
    let _first = primary_of(claim(Some(Path::new(&dir)), &[]));
    println!("held");
    std::io::stdout().flush().unwrap();
    std::thread::sleep(Duration::from_secs(120));
}

// ---- private to the user (Q36) ---------------------------------------------------------------------------------------

#[test]
fn the_socket_is_private() {
    let root = tempfile::tempdir().unwrap();
    let dir = channel(&root);
    let _first = primary_of(claim(Some(&dir), &[]));
    assert_eq!(mode(&dir), 0o700);
    assert_eq!(mode(&dir.join("gui.sock")), 0o600);
    assert_eq!(mode(&dir.join("gui.lock")), 0o600);
    // a folder that was there with wider permissions is closed before the socket is made in it
    let wide = root.path().join("wide").join("galleria");
    fs::create_dir_all(&wide).unwrap();
    fs::set_permissions(&wide, fs::Permissions::from_mode(0o755)).unwrap();
    let _second = primary_of(claim(Some(&wide), &[]));
    assert_eq!(mode(&wide), 0o700);
    assert_eq!(mode(&wide.join("gui.sock")), 0o600);
}

#[test]
fn a_connection_of_another_user_is_closed_without_being_read() {
    let root = tempfile::tempdir().unwrap();
    let dir = channel(&root);
    // this first process serves the connections of the user after us: ours is «another user» for it, whose uid the kernel
    // gives it with `SO_PEERCRED`
    let first = Running::start(primary_of(claim(Some(&dir), &[])), Some(euid() + 1));
    let socket = dir.join("gui.sock");
    let answer = say(&socket, b"{\"v\":1,\"files\":[\"/x/a.ovpn\"]}\n");
    assert_eq!(answer, None, "a user that is not the one got an answer");
    assert!(
        first.files().is_empty(),
        "the files of another user were taken"
    );
    // and the second launch of ours says so loudly instead of opening another window
    assert!(matches!(
        claim(Some(&dir), &paths(&["/x/a.ovpn"])),
        Claim::Failed(_)
    ));
    assert!(first.files().is_empty());
}

#[test]
fn a_socket_with_wider_permissions_is_not_trusted() {
    let root = tempfile::tempdir().unwrap();
    let dir = channel(&root);
    // a first process that is not the real one: it holds the lock and listens, but its socket is open to the group and others
    let first = primary_of(claim(Some(&dir), &[]));
    let socket = dir.join("gui.sock");
    fs::set_permissions(&socket, fs::Permissions::from_mode(0o666)).unwrap();
    let started = Instant::now();
    assert!(
        matches!(claim(Some(&dir), &paths(&["/x/a.ovpn"])), Claim::Failed(_)),
        "the files were handed to a socket that is open to everybody"
    );
    // it refused at once: it did not wait for an answer from the socket
    assert!(started.elapsed() < Duration::from_secs(1));
    // the socket that is private again is trusted
    fs::set_permissions(&socket, fs::Permissions::from_mode(0o600)).unwrap();
    let running = Running::start(first, None);
    assert!(matches!(
        claim(Some(&dir), &paths(&["/x/a.ovpn"])),
        Claim::Handed
    ));
    assert_eq!(running.files(), vec![paths(&["/x/a.ovpn"])]);
}

// ---- the messages ---------------------------------------------------------------------------------------------------

#[test]
fn oversized_slow_or_garbled_messages_are_dropped() {
    let root = tempfile::tempdir().unwrap();
    let dir = channel(&root);
    let first = Running::start(primary_of(claim(Some(&dir), &[])), None);
    let socket = dir.join("gui.sock");

    // 1 MiB with no end of line
    let mut flood = UnixStream::connect(&socket).unwrap();
    flood
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    let chunk = vec![b'a'; 64 * 1024];
    for _ in 0..16 {
        if flood.write_all(&chunk).is_err() {
            break;
        }
    }
    assert_eq!(
        answer_of(&mut flood),
        None,
        "a message of 1 MiB got an answer"
    );

    // a line just over the limit, with its end
    let mut long = vec![b'x'; usize::try_from(MAX_MESSAGE_BYTES).unwrap() + 10];
    long.push(b'\n');
    assert_eq!(say(&socket, &long), None);

    // not JSON, another version, a path that is not absolute, too many paths, a field that is not in the message
    let too_many = format!(
        "{{\"v\":1,\"files\":[{}]}}\n",
        (0..=MAX_FILES)
            .map(|n| format!("\"/x/{n}.ovpn\""))
            .collect::<Vec<_>>()
            .join(",")
    );
    for message in [
        "this is not json\n",
        "{\"v\":2,\"files\":[]}\n",
        "{\"v\":1,\"files\":[\"rel/a.ovpn\"]}\n",
        "{\"v\":1,\"files\":[\"\"]}\n",
        "{\"v\":1,\"files\":[1]}\n",
        "{\"v\":1,\"files\":[],\"more\":true}\n",
        "{\"files\":[]}\n",
        &too_many,
    ] {
        assert_eq!(say(&socket, message.as_bytes()), None, "{message}");
    }
    // a connection that says nothing is let go after the time it has
    let started = Instant::now();
    let mut silent = UnixStream::connect(&socket).unwrap();
    silent
        .set_read_timeout(Some(PEER_TIMEOUT + Duration::from_secs(3)))
        .unwrap();
    assert_eq!(answer_of(&mut silent), None);
    assert!(
        started.elapsed() < PEER_TIMEOUT + Duration::from_secs(2),
        "the silent connection was held for {:?}",
        started.elapsed()
    );
    assert!(
        first.files().is_empty(),
        "something was taken: {:?}",
        first.files()
    );

    // the first process is alive, and a good message still goes through, at the limit of the paths too
    let exact: Vec<PathBuf> = (0..MAX_FILES)
        .map(|n| PathBuf::from(format!("/x/{n}.ovpn")))
        .collect();
    assert!(matches!(claim(Some(&dir), &exact), Claim::Handed));
    assert_eq!(first.files(), vec![exact]);
}

#[test]
fn a_primary_that_does_not_answer_makes_the_second_fail_loudly() {
    let root = tempfile::tempdir().unwrap();
    let dir = channel(&root);
    // it holds the lock and the socket (the kernel takes the connection) but it never serves
    let _mute = primary_of(claim(Some(&dir), &[]));
    let started = Instant::now();
    let outcome = claim(Some(&dir), &paths(&["/x/a.ovpn"]));
    assert!(
        started.elapsed() < Duration::from_secs(3),
        "the second launch waited {:?}",
        started.elapsed()
    );
    let Claim::Failed(why) = outcome else {
        panic!("the second launch did not fail");
    };
    assert!(!why.is_empty());
}

#[test]
fn a_lock_without_a_socket_is_waited_for_a_moment_and_then_it_fails() {
    // the first process is between taking the lock and making the socket for a moment: not forever
    let root = tempfile::tempdir().unwrap();
    let dir = channel(&root);
    let first = primary_of(claim(Some(&dir), &[]));
    fs::remove_file(dir.join("gui.sock")).unwrap();
    let started = Instant::now();
    assert!(matches!(
        claim(Some(&dir), &paths(&["/x/a.ovpn"])),
        Claim::Failed(_)
    ));
    assert!(started.elapsed() < Duration::from_secs(4));
    drop(first);
}

// ---- what the first process does with the files ---------------------------------------------------------------------

#[test]
fn handed_files_get_the_same_checks_as_dropped_ones() {
    let root = tempfile::tempdir().unwrap();
    let rubrica = Arc::new(Rubrica::new(
        root.path().join("galleria"),
        Arc::new(UiStateFile::new(
            root.path().join("state/galleria/state.toml"),
        )),
    ));
    let imports = Arc::new(Imports::new(rubrica));
    let fifo = root.path().join("fifo.ovpn");
    assert!(
        Command::new("mkfifo")
            .arg(&fifo)
            .status()
            .unwrap()
            .success()
    );
    let dir = channel(&root);
    let primary = primary_of(claim(Some(&dir), &[]));
    let (tx, rx) = std::sync::mpsc::channel();
    let handler = Arc::clone(&imports);
    let thread = std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async {
            let _ = tx.send(());
            primary
                .serve(move |files| {
                    handler.offer_from_outside(&files, Origin::Open);
                })
                .await;
        });
    });
    rx.recv().unwrap();
    // a FIFO as an argument: refused like a dropped one, and the second launch is not left waiting for a writer
    let started = Instant::now();
    assert!(matches!(
        claim(Some(&dir), std::slice::from_ref(&fifo)),
        Claim::Handed
    ));
    assert!(started.elapsed() < Duration::from_secs(5));
    let Some(ImportOffer::Failed { error, file_name }) = imports.take_pending() else {
        panic!("the FIFO was not refused");
    };
    assert_eq!(error.code, "gui_import_unsupported_file");
    assert_eq!(file_name.as_deref(), Some("fifo.ovpn"));
    // a thread that serves for ever is left to the end of the process: the lock and the socket go with it
    drop(thread);
}

#[test]
fn a_channel_that_reads_no_more_than_the_limit_does_not_grow_with_the_input() {
    // a stream that never ends is read for 64 KiB and no further
    let root = tempfile::tempdir().unwrap();
    let dir = channel(&root);
    let first = Running::start(primary_of(claim(Some(&dir), &[])), None);
    let mut stream = UnixStream::connect(dir.join("gui.sock")).unwrap();
    stream
        .set_write_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    let chunk = vec![b'z'; 8 * 1024];
    let mut written = 0usize;
    while stream.write_all(&chunk).is_ok() && written < 16 * 1024 * 1024 {
        written += chunk.len();
    }
    // the other side stopped reading and closed: we could not push 16 MiB into it
    assert!(written < 16 * 1024 * 1024, "{written} bytes went in");
    let mut sink = Vec::new();
    let _ = stream.read_to_end(&mut sink);
    assert!(first.files().is_empty());
}
