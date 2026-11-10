//! Platform glue for the spike: where the management interface and the netcfg helper channel live.
//!
//! Linux: Unix sockets in a 0700 root directory (criterion 2.2).
//! Windows: OpenVPN has no Unix-socket management there (M5), so TCP on 127.0.0.1 with a random
//! port and a random per-session password (criterion 2.3); the helper channel and the
//! configuration travel through named pipes with an ACL for SYSTEM and Administrators only.

use std::io::{Read, Write};
use std::process::Child;
use std::time::{Duration, Instant};

use crate::Ipv6Mode;

pub type Rw = (Box<dyn Read + Send>, Box<dyn Write + Send>);

pub fn random_hex(bytes: usize) -> String {
    let mut b = vec![0u8; bytes];
    getrandom::getrandom(&mut b).expect("os random");
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// Command line of the fixed `--up`/`--down` helper as written in the generated config.
/// OpenVPN parses the string twice (config line, then command), and backslash is an escape
/// character both times: on Windows the path is written with forward slashes and the pipe is
/// passed by its short name (found the hard way: `C:\\gl\\x.exe` arrives as `C:glx.exe`).
pub fn helper_command(exe: &str, endpoint: &str) -> String {
    format!("{} netcfg {}", exe.replace('\\', "/"), endpoint)
}

pub enum Mgmt {
    #[cfg(unix)]
    Unix { path: String },
    #[cfg(windows)]
    Tcp { port: u16, password: String, listener: Option<std::net::TcpListener> },
}

// ---------------------------------------------------------------- unix
#[cfg(unix)]
mod imp {
    use super::*;
    use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
    use std::os::unix::net::{UnixListener, UnixStream};

    pub const DEFAULT_OPENVPN: &str = "/usr/sbin/openvpn";

    pub fn session_dir() -> String {
        // /run/galleria is 0700 root (M5)
        let base = "/run/galleria";
        let _ = std::fs::DirBuilder::new().mode(0o700).create(base);
        let _ = std::fs::set_permissions(base, std::fs::Permissions::from_mode(0o700));
        let sess = format!("{base}/s{}", std::process::id());
        std::fs::DirBuilder::new().mode(0o700).create(&sess).expect("session dir");
        sess
    }

    pub fn new_mgmt(sess: &str, _client_mode: bool) -> Mgmt {
        Mgmt::Unix { path: format!("{sess}/mgmt.sock") }
    }

    pub fn mgmt_directives(m: &Mgmt) -> Vec<String> {
        let Mgmt::Unix { path } = m;
        vec![format!("management {path} unix"), "management-client-user root".into()]
    }

    pub fn mgmt_describe(m: &Mgmt) -> String {
        let Mgmt::Unix { path } = m;
        path.clone()
    }

    pub fn mgmt_prepare(m: &Mgmt) {
        let Mgmt::Unix { path } = m;
        let _ = std::fs::remove_file(path);
    }

    /// Nothing to send on the child's stdin on Linux.
    pub fn stdin_secret(_m: &Mgmt) -> Option<String> {
        None
    }

    pub fn mgmt_connect(m: &Mgmt, child: &mut Child) -> Result<Rw, String> {
        let Mgmt::Unix { path } = m;
        let t0 = Instant::now();
        loop {
            if let Ok(s) = UnixStream::connect(path) {
                let w = s.try_clone().map_err(|e| e.to_string())?;
                return Ok((Box::new(s), Box::new(w)));
            }
            if let Ok(Some(st)) = child.try_wait() {
                return Err(format!("openvpn exited before management was up ({st})"));
            }
            if t0.elapsed() > Duration::from_secs(10) {
                let _ = child.kill();
                return Err("management socket not available".into());
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    pub fn netcfg_endpoint(sess: &str) -> String {
        format!("{sess}/netcfg.sock")
    }

    pub fn spawn_netcfg_server(endpoint: &str, ipv6: Ipv6Mode) {
        let listener = UnixListener::bind(endpoint).expect("netcfg socket");
        std::thread::spawn(move || {
            for conn in listener.incoming() {
                let Ok(conn) = conn else { continue };
                let Ok(w) = conn.try_clone() else { continue };
                crate::netcfg::handle(conn, w, ipv6);
            }
        });
    }

    pub fn netcfg_client(endpoint: &str) -> std::io::Result<Rw> {
        let s = UnixStream::connect(endpoint)?;
        let w = s.try_clone()?;
        Ok((Box::new(s), Box::new(w)))
    }

    /// Configuration through a pipe: on Linux the child's stdin.
    pub fn config_pipe(_cfg: &str) -> (String, bool) {
        ("/dev/stdin".into(), true)
    }

    pub fn write_private_file(path: &str, data: &[u8]) -> std::io::Result<()> {
        let mut f = std::fs::OpenOptions::new();
        std::os::unix::fs::OpenOptionsExt::mode(&mut f, 0o600);
        f.write(true).create_new(true).open(path)?.write_all(data)
    }

    pub fn tempfile_path(sess: &str) -> String {
        format!("{sess}/config.ovpn")
    }

    pub fn describe_access(path: &str) -> String {
        std::fs::metadata(path).map(|m| format!("mode {:o}", m.permissions().mode() & 0o7777)).unwrap_or_default()
    }

    pub fn cleanup(sess: &str) {
        let _ = std::fs::remove_dir_all(sess);
    }
}

// ---------------------------------------------------------------- windows
#[cfg(windows)]
mod imp {
    use super::*;
    use std::fs::File;
    use std::net::{TcpListener, TcpStream};
    use std::os::windows::io::FromRawHandle;
    use windows_sys::Win32::Foundation::{CloseHandle, LocalFree, HANDLE, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::Security::Authorization::ConvertStringSecurityDescriptorToSecurityDescriptorW;
    use windows_sys::Win32::Security::SECURITY_ATTRIBUTES;
    use windows_sys::Win32::Storage::FileSystem::{FlushFileBuffers, PIPE_ACCESS_DUPLEX, PIPE_ACCESS_OUTBOUND};
    use windows_sys::Win32::System::Pipes::{ConnectNamedPipe, CreateNamedPipeW, DisconnectNamedPipe};

    pub const DEFAULT_OPENVPN: &str = r"C:\Program Files\OpenVPN\bin\openvpn.exe";
    /// SYSTEM and Administrators only; no access for ordinary users.
    const SDDL: &str = "D:P(A;;GA;;;SY)(A;;GA;;;BA)";
    const FILE_FLAG_FIRST_PIPE_INSTANCE: u32 = 0x0008_0000;
    const PIPE_REJECT_REMOTE_CLIENTS: u32 = 0x0000_0008;

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    /// Create one instance of a named pipe with our ACL. `FILE_FLAG_FIRST_PIPE_INSTANCE` makes the
    /// call fail if somebody else already created a pipe with this name (pipe squatting).
    fn create_pipe(name: &str, access: u32) -> std::io::Result<HANDLE> {
        unsafe {
            let mut sd = std::ptr::null_mut();
            if ConvertStringSecurityDescriptorToSecurityDescriptorW(wide(SDDL).as_ptr(), 1, &mut sd, std::ptr::null_mut()) == 0 {
                return Err(std::io::Error::last_os_error());
            }
            let sa = SECURITY_ATTRIBUTES { nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32, lpSecurityDescriptor: sd, bInheritHandle: 0 };
            let h = CreateNamedPipeW(wide(name).as_ptr(), access | FILE_FLAG_FIRST_PIPE_INSTANCE, PIPE_REJECT_REMOTE_CLIENTS, 1, 65536, 65536, 0, &sa);
            let err = std::io::Error::last_os_error();
            LocalFree(sd as _);
            if h == INVALID_HANDLE_VALUE { Err(err) } else { Ok(h) }
        }
    }

    pub fn session_dir() -> String {
        // only used by the temp-file fallback; created on demand
        format!(r"C:\ProgramData\galleria-spike\s{}", std::process::id())
    }

    pub fn new_mgmt(_sess: &str, client_mode: bool) -> Mgmt {
        // random free port on loopback; in client mode we keep listening and openvpn connects to us
        let l = TcpListener::bind("127.0.0.1:0").expect("loopback port");
        let port = l.local_addr().unwrap().port();
        Mgmt::Tcp { port, password: random_hex(32), listener: if client_mode { Some(l) } else { None } }
    }

    pub fn mgmt_directives(m: &Mgmt) -> Vec<String> {
        let Mgmt::Tcp { port, listener, .. } = m;
        // "stdin": openvpn asks for the management password on its standard input
        let mut v = vec![format!("management 127.0.0.1 {port} stdin")];
        if listener.is_some() {
            v.push("management-client".into());
        }
        v
    }

    pub fn mgmt_describe(m: &Mgmt) -> String {
        let Mgmt::Tcp { port, listener, .. } = m;
        format!("tcp 127.0.0.1:{port} ({})", if listener.is_some() { "management-client: openvpn connects to the daemon" } else { "openvpn listens" })
    }

    pub fn mgmt_prepare(_m: &Mgmt) {}

    pub fn stdin_secret(m: &Mgmt) -> Option<String> {
        let Mgmt::Tcp { password, .. } = m;
        Some(password.clone())
    }

    pub fn mgmt_connect(m: &Mgmt, child: &mut Child) -> Result<Rw, String> {
        let Mgmt::Tcp { port, password, listener } = m;
        let t0 = Instant::now();
        if let Some(l) = listener {
            l.set_nonblocking(true).map_err(|e| e.to_string())?;
        }
        let mut s = loop {
            let got = match listener {
                Some(l) => l.accept().map(|(s, _)| s),
                None => TcpStream::connect(("127.0.0.1", *port)),
            };
            if let Ok(s) = got {
                break s;
            }
            if let Ok(Some(st)) = child.try_wait() {
                return Err(format!("openvpn exited before management was up ({st})"));
            }
            if t0.elapsed() > Duration::from_secs(15) {
                let _ = child.kill();
                return Err("management port not available".into());
            }
            std::thread::sleep(Duration::from_millis(50));
        };
        s.set_nonblocking(false).map_err(|e| e.to_string())?;
        // openvpn sends "ENTER PASSWORD:" without a newline: answer straight away
        s.write_all(format!("{password}\n").as_bytes()).map_err(|e| e.to_string())?;
        let w = s.try_clone().map_err(|e| e.to_string())?;
        Ok((Box::new(s), Box::new(w)))
    }

    pub fn netcfg_endpoint(_sess: &str) -> String {
        // short name: the helper and the server add the `\\.\pipe\` prefix themselves
        format!("galleria-netcfg-{}-{}", std::process::id(), random_hex(8))
    }

    pub fn spawn_netcfg_server(endpoint: &str, ipv6: Ipv6Mode) {
        let h = create_pipe(&format!(r"\\.\pipe\{endpoint}"), PIPE_ACCESS_DUPLEX).expect("netcfg pipe") as usize;
        std::thread::spawn(move || loop {
            let h = h as HANDLE;
            unsafe {
                // ERROR_PIPE_CONNECTED (535) means the client was faster than us: fine
                if ConnectNamedPipe(h, std::ptr::null_mut()) == 0 && std::io::Error::last_os_error().raw_os_error() != Some(535) {
                    std::thread::sleep(Duration::from_millis(100));
                    continue;
                }
                let f = std::mem::ManuallyDrop::new(File::from_raw_handle(h as _));
                let (r, w) = (f.try_clone(), f.try_clone());
                if let (Ok(r), Ok(w)) = (r, w) {
                    crate::netcfg::handle(r, w, ipv6);
                }
                FlushFileBuffers(h);
                DisconnectNamedPipe(h);
            }
        });
    }

    pub fn netcfg_client(endpoint: &str) -> std::io::Result<Rw> {
        let f = std::fs::OpenOptions::new().read(true).write(true).open(format!(r"\\.\pipe\{endpoint}"))?;
        let w = f.try_clone()?;
        Ok((Box::new(f), Box::new(w)))
    }

    /// Configuration through a pipe: on Windows the child's stdin is taken by the management
    /// password, so the configuration goes through a one-shot named pipe that openvpn opens as
    /// if it were the config file.
    pub fn config_pipe(cfg: &str) -> (String, bool) {
        let name = format!(r"\\.\pipe\galleria-cfg-{}-{}", std::process::id(), random_hex(8));
        let h = create_pipe(&name, PIPE_ACCESS_OUTBOUND).expect("config pipe") as usize;
        let data = cfg.as_bytes().to_vec();
        std::thread::spawn(move || unsafe {
            let h = h as HANDLE;
            if ConnectNamedPipe(h, std::ptr::null_mut()) != 0 || std::io::Error::last_os_error().raw_os_error() == Some(535) {
                let mut f = std::mem::ManuallyDrop::new(File::from_raw_handle(h as _));
                let _ = f.write_all(&data);
                FlushFileBuffers(h);
            }
            DisconnectNamedPipe(h);
            CloseHandle(h);
        });
        (name, false)
    }

    /// Temp-file fallback: directory and file readable by SYSTEM and Administrators only.
    pub fn write_private_file(path: &str, data: &[u8]) -> std::io::Result<()> {
        let dir = std::path::Path::new(path).parent().unwrap();
        std::fs::create_dir_all(dir)?;
        let acl = |p: &str| {
            std::process::Command::new("icacls").args([p, "/inheritance:r", "/grant:r", "*S-1-5-18:(F)", "/grant:r", "*S-1-5-32-544:(F)"]).output().map(|o| o.status.success()).unwrap_or(false)
        };
        if !acl(&dir.to_string_lossy()) {
            return Err(std::io::Error::other("icacls failed on the session directory"));
        }
        std::fs::OpenOptions::new().write(true).create_new(true).open(path)?.write_all(data)?;
        if !acl(path) {
            return Err(std::io::Error::other("icacls failed on the config file"));
        }
        Ok(())
    }

    pub fn tempfile_path(sess: &str) -> String {
        format!(r"{sess}\config.ovpn")
    }

    pub fn describe_access(path: &str) -> String {
        std::process::Command::new("icacls").arg(path).output().map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string()).unwrap_or_default()
    }

    pub fn cleanup(sess: &str) {
        let _ = std::fs::remove_dir_all(sess);
    }
}

pub use imp::*;
