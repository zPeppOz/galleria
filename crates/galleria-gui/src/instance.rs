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

use serde::Deserialize;
use std::{
    ffi::{OsStr, OsString},
    fs::{self, DirBuilder, File, OpenOptions, TryLockError},
    io::{self, BufRead, BufReader, Read, Write},
    os::unix::{
        ffi::{OsStrExt, OsStringExt},
        fs::{DirBuilderExt, FileTypeExt, MetadataExt, OpenOptionsExt, PermissionsExt},
        net::{UnixListener as StdListener, UnixStream as StdStream},
    },
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt};

/// The longest message, the most paths in one, and how long a connection has to say what it has to say.
pub const MAX_MESSAGE_BYTES: u64 = 64 * 1024;
pub const MAX_FILES: usize = 16;
pub const PEER_TIMEOUT: Duration = Duration::from_secs(2);

const LOCK: &str = "gui.lock";
const SOCKET: &str = "gui.sock";
const VERSION: u32 = 1;

/// The folder of the channel: `galleria` inside the runtime folder of the session, which must be a path (an empty or
/// relative value is as good as none).
pub fn runtime_dir(xdg_runtime_dir: Option<OsString>) -> Option<PathBuf> {
    let base = PathBuf::from(xdg_runtime_dir?);
    base.is_absolute().then(|| base.join("galleria"))
}

/// The arguments that are files: what starts with `-` is an option and is skipped, `file://…` is read as a path (percent
/// escapes included; a host that is not empty or `localhost` is not a file of ours), any other URL is skipped, and a
/// relative path is made absolute from `cwd`. Nothing here looks at the disk. An argument that is not UTF-8 is skipped: it
/// could not be told to the first process, and the file managers give percent-escaped URLs.
pub fn files_in_args(args: impl IntoIterator<Item = OsString>, cwd: &Path) -> Vec<PathBuf> {
    args.into_iter()
        .filter_map(|arg| file_of(&arg, cwd))
        .collect()
}

fn file_of(arg: &OsStr, cwd: &Path) -> Option<PathBuf> {
    if arg.is_empty() || arg.as_bytes().first() == Some(&b'-') {
        return None;
    }
    let text = arg.to_str()?;
    let path = match scheme_of(text) {
        Some(scheme) if scheme.eq_ignore_ascii_case("file") => path_of_file_url(text)?,
        Some(_) => return None,
        None => PathBuf::from(text),
    };
    Some(if path.is_absolute() {
        path
    } else {
        cwd.join(path)
    })
}

/// `https` in `https://…`: letters, digits, `+`, `-` and `.` after a letter, then `://`.
fn scheme_of(text: &str) -> Option<&str> {
    let (scheme, _) = text.split_once("://")?;
    let mut chars = scheme.chars();
    let valid = chars.next().is_some_and(|c| c.is_ascii_alphabetic())
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'));
    valid.then_some(scheme)
}

fn path_of_file_url(url: &str) -> Option<PathBuf> {
    let rest = url.get("file://".len()..)?;
    // a query or a fragment is not part of a path
    let rest = rest.split(['?', '#']).next()?;
    let (authority, path) = rest.split_at(rest.find('/')?);
    if !authority.is_empty() && !authority.eq_ignore_ascii_case("localhost") {
        return None;
    }
    let decoded = percent_decode(path)?;
    Some(PathBuf::from(OsString::from_vec(decoded)))
}

fn percent_decode(text: &str) -> Option<Vec<u8>> {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut at = 0;
    while at < bytes.len() {
        if bytes[at] == b'%' {
            let pair = bytes.get(at + 1..at + 3)?;
            let value = u8::from_str_radix(std::str::from_utf8(pair).ok()?, 16).ok()?;
            // a NUL cannot be in a path
            if value == 0 {
                return None;
            }
            out.push(value);
            at += 3;
        } else {
            out.push(bytes[at]);
            at += 1;
        }
    }
    Some(out)
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

/// The first process: the lock and the socket it listens on. The lock is held, and the socket left on the disk is taken
/// away, when this is dropped.
pub struct Primary {
    listener: StdListener,
    guard: Guard,
}

struct Guard {
    /// Holding the file holds the lock.
    _lock: File,
    socket: PathBuf,
}

impl Drop for Guard {
    fn drop(&mut self) {
        // The lock is released with the file; the socket of a process that is gone would only be taken away by the next one.
        let _ = fs::remove_file(&self.socket);
    }
}

fn euid() -> u32 {
    nix::unistd::geteuid().as_raw()
}

/// Makes the folder (or takes the one that is there) private: it must be ours, and it is `0700`.
fn private_dir(dir: &Path) -> io::Result<()> {
    DirBuilder::new().recursive(true).mode(0o700).create(dir)?;
    let meta = fs::symlink_metadata(dir)?;
    if !meta.is_dir() || meta.uid() != euid() {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "the folder is not a folder of this user",
        ));
    }
    if meta.mode() & 0o777 != 0o700 {
        fs::set_permissions(dir, fs::Permissions::from_mode(0o700))?;
    }
    Ok(())
}

/// A socket that this user made and nobody else can use: that is what a file path is told to.
fn trusted_socket(socket: &Path) -> io::Result<()> {
    let meta = fs::symlink_metadata(socket)?;
    if !meta.file_type().is_socket() || meta.uid() != euid() || meta.mode() & 0o077 != 0 {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "the socket is not private to this user",
        ));
    }
    Ok(())
}

fn listen(dir: &Path) -> io::Result<StdListener> {
    let socket = dir.join(SOCKET);
    match fs::remove_file(&socket) {
        Ok(()) => {}
        Err(e) if e.kind() == io::ErrorKind::NotFound => {}
        Err(e) => return Err(e),
    }
    // The folder is `0700`, so nobody gets to the socket in the moment between `bind` and the change of its mode.
    let listener = StdListener::bind(&socket)?;
    fs::set_permissions(&socket, fs::Permissions::from_mode(0o600))?;
    Ok(listener)
}

/// Decides what this launch is. `runtime_dir` is `$XDG_RUNTIME_DIR/galleria` ([`runtime_dir`]); without it there is no
/// channel and the app works alone. `files` are the files of the arguments, absolute ([`files_in_args`]): a second launch
/// gives them to the first (an empty list only brings its window forward).
pub fn claim(runtime_dir: Option<&Path>, files: &[PathBuf]) -> Claim {
    let Some(dir) = runtime_dir else {
        return Claim::Alone("there is no runtime folder (XDG_RUNTIME_DIR)".to_owned());
    };
    if let Err(e) = private_dir(dir) {
        return Claim::Alone(format!("the runtime folder cannot be used: {e}"));
    }
    let lock = OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .mode(0o600)
        .open(dir.join(LOCK));
    let lock = match lock {
        Ok(lock) => lock,
        Err(e) => return Claim::Alone(format!("the lock cannot be opened: {e}")),
    };
    match lock.try_lock() {
        Ok(()) => match listen(dir) {
            Ok(listener) => Claim::Primary(Primary {
                listener,
                guard: Guard {
                    _lock: lock,
                    socket: dir.join(SOCKET),
                },
            }),
            Err(e) => Claim::Alone(format!("the socket cannot be made: {e}")),
        },
        Err(TryLockError::WouldBlock) => hand_over(dir, files),
        Err(TryLockError::Error(e)) => Claim::Alone(format!("the lock cannot be taken: {e}")),
    }
}

/// The second process: the first window has the lock, so the files go to it. It has [`PEER_TIMEOUT`] in all, the time the
/// first one may need to start listening included. Nothing is sent to a socket that is not private to this user.
fn hand_over(dir: &Path, files: &[PathBuf]) -> Claim {
    let deadline = Instant::now() + PEER_TIMEOUT;
    let socket = dir.join(SOCKET);
    let stream = loop {
        match trusted_socket(&socket).and_then(|()| StdStream::connect(&socket)) {
            Ok(stream) => break stream,
            // not there yet, or not listening yet: the first process is between the lock and the socket
            Err(e)
                if matches!(
                    e.kind(),
                    io::ErrorKind::NotFound | io::ErrorKind::ConnectionRefused
                ) && Instant::now() < deadline =>
            {
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(e) => return Claim::Failed(format!("the first window cannot be reached: {e}")),
        }
    };
    let left = deadline
        .saturating_duration_since(Instant::now())
        .max(Duration::from_millis(100));
    match talk(stream, files, left) {
        Ok(()) => Claim::Handed,
        Err(e) => Claim::Failed(format!("the first window did not take the files: {e}")),
    }
}

#[derive(serde::Serialize)]
struct Outgoing<'a> {
    v: u32,
    files: Vec<&'a str>,
}

fn talk(mut stream: StdStream, files: &[PathBuf], within: Duration) -> io::Result<()> {
    stream.set_read_timeout(Some(within))?;
    stream.set_write_timeout(Some(within))?;
    let files: Vec<&str> = files
        .iter()
        .filter_map(|p| p.to_str())
        .take(MAX_FILES)
        .collect();
    let mut line = serde_json::to_vec(&Outgoing { v: VERSION, files }).map_err(io::Error::other)?;
    line.push(b'\n');
    stream.write_all(&line)?;
    let mut answer = String::new();
    BufReader::new(stream.take(16)).read_line(&mut answer)?;
    if answer == "ok\n" {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "no «ok» came back",
        ))
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Incoming {
    v: u32,
    files: Vec<String>,
}

/// The paths of a message, or `None` for anything that is not one: another version, too many paths, a path that is empty,
/// relative or has a NUL in it.
fn paths_of(line: &[u8]) -> Option<Vec<PathBuf>> {
    let message: Incoming = serde_json::from_slice(line).ok()?;
    if message.v != VERSION || message.files.len() > MAX_FILES {
        return None;
    }
    message
        .files
        .into_iter()
        .map(|file| {
            let path = PathBuf::from(file);
            (path.is_absolute() && !path.as_os_str().as_bytes().contains(&0)).then_some(path)
        })
        .collect()
}

/// One connection: of the same user (as the kernel says, before a byte is read), one line within the size, a message that
/// says what it should. `None` closes it without an answer.
async fn receive_from(stream: &mut tokio::net::UnixStream, uid: u32) -> Option<Vec<PathBuf>> {
    if stream.peer_cred().ok()?.uid() != uid {
        return None;
    }
    let mut reader = tokio::io::BufReader::new(stream.take(MAX_MESSAGE_BYTES));
    let mut line = Vec::new();
    reader.read_until(b'\n', &mut line).await.ok()?;
    // a line without its end is one that was too long, or cut short
    if line.last() != Some(&b'\n') {
        return None;
    }
    paths_of(&line)
}

impl Primary {
    /// Takes the files that other launches hand over, one at a time, until the future is dropped. `on_files` gets a list of
    /// absolute paths, empty when the second launch had none (it only brings the window forward); it is called before
    /// the sender gets its `ok`.
    pub async fn serve(self, on_files: impl Fn(Vec<PathBuf>) + Send + 'static) {
        self.serve_for(euid(), on_files).await;
    }

    /// [`Primary::serve`] for the connections of the user `uid`. The app serves its own user; a test serves another, to
    /// see that a connection of a different user is closed without being read.
    pub async fn serve_for(self, uid: u32, on_files: impl Fn(Vec<PathBuf>) + Send + 'static) {
        let Self { listener, guard } = self;
        let _guard = guard;
        if listener.set_nonblocking(true).is_err() {
            return;
        }
        let Ok(listener) = tokio::net::UnixListener::from_std(listener) else {
            return;
        };
        loop {
            let Ok((stream, _)) = listener.accept().await else {
                // a failure of `accept` (no descriptors left, say) is not the end of the service: wait a little
                tokio::time::sleep(Duration::from_millis(100)).await;
                continue;
            };
            handle(stream, uid, &on_files).await;
        }
    }
}

async fn handle(stream: tokio::net::UnixStream, uid: u32, on_files: &impl Fn(Vec<PathBuf>)) {
    let work = async {
        let mut stream = stream;
        let paths = receive_from(&mut stream, uid).await?;
        on_files(paths);
        stream.write_all(b"ok\n").await.ok()
    };
    // a connection that is slow is closed, and the next one is taken
    let _ = tokio::time::timeout(PEER_TIMEOUT, work).await;
}
