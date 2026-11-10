//! Small test clients used by the spike harness (same binary on Linux and Windows).

use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::{Duration, Instant};

/// A client without Happy Eyeballs: tries the resolved addresses one after the other, like
/// `socket.create_connection` in Python (used for criteria 1.5 and 2.8).
///   seqprobe <host> <port> [timeout-secs]
pub fn seqprobe(args: &[String]) -> i32 {
    let (Some(host), Some(port)) = (args.first(), args.get(1).and_then(|p| p.parse::<u16>().ok())) else {
        eprintln!("usage: seqprobe <host> <port> [timeout-secs]");
        return 64;
    };
    let timeout = Duration::from_secs(args.get(2).and_then(|t| t.parse().ok()).unwrap_or(10));
    let t0 = Instant::now();
    let addrs: Vec<_> = match (host.as_str(), port).to_socket_addrs() {
        Ok(a) => a.collect(),
        Err(e) => {
            println!("seqprobe: resolve {host}: {e}");
            return 1;
        }
    };
    println!("seqprobe: {host} -> {addrs:?}");
    for a in &addrs {
        let t = Instant::now();
        match TcpStream::connect_timeout(a, timeout) {
            Ok(mut s) => {
                let _ = s.set_read_timeout(Some(timeout));
                let _ = s.write_all(format!("GET / HTTP/1.0\r\nHost: {host}\r\n\r\n").as_bytes());
                let mut body = String::new();
                let _ = s.read_to_string(&mut body);
                println!("seqprobe: connected to {a} after {:.3}s (this address {:.3}s); reply: {:?}",
                    t0.elapsed().as_secs_f64(), t.elapsed().as_secs_f64(), body.lines().last().unwrap_or(""));
                return 0;
            }
            Err(e) => println!("seqprobe: {a} failed after {:.3}s: {e}", t.elapsed().as_secs_f64()),
        }
    }
    println!("seqprobe: no address worked, total {:.3}s", t0.elapsed().as_secs_f64());
    1
}

/// Another local process trying the TCP management port (criterion 2.3).
///   mgmtprobe <port> [password]
pub fn mgmtprobe(args: &[String]) -> i32 {
    let Some(port) = args.first().and_then(|p| p.parse::<u16>().ok()) else {
        eprintln!("usage: mgmtprobe <port> [password]");
        return 64;
    };
    let t0 = Instant::now();
    let mut s = match TcpStream::connect_timeout(&([127, 0, 0, 1], port).into(), Duration::from_secs(3)) {
        Ok(s) => s,
        Err(e) => {
            println!("mgmtprobe: connect to 127.0.0.1:{port} failed: {e}");
            return 1;
        }
    };
    println!("mgmtprobe: tcp connect ok after {:.3}s", t0.elapsed().as_secs_f64());
    let _ = s.set_read_timeout(Some(Duration::from_secs(4)));
    let mut buf = [0u8; 512];
    let mut read = |s: &mut TcpStream, what: &str| match s.read(&mut buf) {
        Ok(0) => println!("mgmtprobe: {what}: connection closed by openvpn"),
        Ok(n) => println!("mgmtprobe: {what}: {:?}", String::from_utf8_lossy(&buf[..n])),
        Err(e) => println!("mgmtprobe: {what}: no data ({e})"),
    };
    read(&mut s, "greeting");
    let pw = args.get(1).cloned().unwrap_or_default();
    let _ = s.write_all(format!("{pw}\n").as_bytes());
    read(&mut s, "after password");
    let _ = s.write_all(b"state\n");
    read(&mut s, "after `state`");
    0
}
