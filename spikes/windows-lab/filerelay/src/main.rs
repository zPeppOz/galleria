//! filerelay — throwaway lab tool for the Windows spikes.
//!
//! Windows Sandbox and WSL have no network path to each other and the host network must not be
//! touched. The only shared thing is the pair of mapped folders, so UDP datagrams travel through
//! two append-only files in the writable mapped folder:
//!
//!   sandbox:  filerelay listen  <ip:port> <tx-file> <rx-file>   (the VPN client talks to ip:port)
//!   wsl:      filerelay connect <ip:port> <tx-file> <rx-file>   (ip:port is the real VPN server)
//!
//! File format: 8 bytes of epoch (changes when the writer restarts), then records of
//! `u16 length + u16 flow (big endian) + datagram`. The flow is the client's UDP source port: the
//! WSL side keeps one socket per flow, so every new client process reaches the server from a new
//! source port, as on a real network (with a single socket an OpenVPN server took a new login for
//! a renegotiation of the previous one: "username attempted to change from 'alice' to 'bob'").
//! The reader reopens the file at every poll: inside the Sandbox an open handle (or a directory
//! listing) sees host-side changes seconds late, a fresh open sees them in a few milliseconds
//! (measured, see evidence/relay-latency.txt).
//! The process exits when a file named `<tx-file>.stop` appears.

use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

type Out = Arc<Mutex<File>>;

/// Pump one socket into the shared tx file. `flow`: fixed (connect side) or the sender's port (listen side).
fn pump(sock: UdpSocket, out: Out, flow: Option<u16>, last_ip: Arc<Mutex<IpAddr>>) {
    std::thread::spawn(move || {
        let mut buf = [0u8; 65540];
        loop {
            let Ok((n, from)) = sock.recv_from(&mut buf[4..]) else { continue };
            let f = flow.unwrap_or_else(|| {
                *last_ip.lock().unwrap() = from.ip();
                from.port()
            });
            buf[..2].copy_from_slice(&(n as u16).to_be_bytes());
            buf[2..4].copy_from_slice(&f.to_be_bytes());
            let mut o = out.lock().unwrap();
            let _ = o.write_all(&buf[..n + 4]);
            let _ = o.flush();
        }
    });
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    if a.len() != 5 || (a[1] != "listen" && a[1] != "connect") {
        eprintln!("usage: filerelay listen|connect <ip:port> <tx-file> <rx-file>");
        std::process::exit(64);
    }
    let addr: SocketAddr = a[2].parse().expect("ip:port");
    let (tx, rx) = (PathBuf::from(&a[3]), PathBuf::from(&a[4]));
    let stop = PathBuf::from(format!("{}.stop", a[3]));
    let _ = fs::remove_file(&stop);
    let listen = a[1] == "listen";

    let epoch = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos() as u64;
    let mut f = File::create(&tx).expect("tx file");
    f.write_all(&epoch.to_be_bytes()).unwrap();
    f.flush().unwrap();
    let out: Out = Arc::new(Mutex::new(f));
    let last_ip = Arc::new(Mutex::new(IpAddr::V4(Ipv4Addr::LOCALHOST)));

    // listen side: one socket, flows told apart by the client's source port
    let listener = if listen {
        let s = UdpSocket::bind(addr).expect("bind");
        pump(s.try_clone().unwrap(), out.clone(), None, last_ip.clone());
        Some(s)
    } else {
        None
    };
    // connect side: one socket per flow
    let mut flows: HashMap<u16, UdpSocket> = HashMap::new();

    let mut cur_epoch = [0u8; 8];
    let mut off: u64 = 8;
    let mut pending: Vec<u8> = Vec::new();
    let mut polls: u32 = 0;
    loop {
        polls = polls.wrapping_add(1);
        if polls % 256 == 0 && stop.exists() {
            eprintln!("filerelay: stop file found, exiting");
            return;
        }
        let mut got = false;
        if let Ok(mut f) = OpenOptions::new().read(true).open(&rx) {
            let mut e = [0u8; 8];
            if f.read_exact(&mut e).is_ok() {
                if e != cur_epoch {
                    cur_epoch = e;
                    off = 8;
                    pending.clear();
                }
                let len = f.metadata().map(|m| m.len()).unwrap_or(0);
                if len > off && f.seek(SeekFrom::Start(off)).is_ok() {
                    let mut chunk = vec![0u8; (len - off) as usize];
                    if let Ok(n) = f.read(&mut chunk) {
                        off += n as u64;
                        pending.extend_from_slice(&chunk[..n]);
                        got = n > 0;
                    }
                }
            }
        }
        while pending.len() >= 4 {
            let n = u16::from_be_bytes([pending[0], pending[1]]) as usize;
            let flow = u16::from_be_bytes([pending[2], pending[3]]);
            if pending.len() < 4 + n {
                break;
            }
            let data = &pending[4..4 + n];
            match &listener {
                Some(s) => {
                    let _ = s.send_to(data, SocketAddr::new(*last_ip.lock().unwrap(), flow));
                }
                None => {
                    let s = flows.entry(flow).or_insert_with(|| {
                        let s = UdpSocket::bind("0.0.0.0:0").expect("bind");
                        pump(s.try_clone().unwrap(), out.clone(), Some(flow), last_ip.clone());
                        s
                    });
                    let _ = s.send_to(data, addr);
                }
            }
            pending.drain(..4 + n);
        }
        if !got {
            std::thread::sleep(Duration::from_millis(1));
        }
    }
}
