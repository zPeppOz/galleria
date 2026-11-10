//! galleriad-spike — spike 2 (throwaway). Drives openvpn 2.6/2.7 only via the
//! management interface, with no secrets on disk.
//!
//!   galleriad-spike connect   < request.json      (as root; events as JSON lines on stdout)
//!   galleriad-spike netcfg <socket>               (fixed --up/--down helper, run by openvpn)

mod netcfg;
mod sanitize;

use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
use std::os::unix::net::{UnixListener, UnixStream};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use base64::Engine as _;
use serde::Deserialize;
use serde_json::{json, Value};

static START: OnceLock<Instant> = OnceLock::new();

pub fn emit(event: &str, fields: Value) {
    let t = START.get_or_init(Instant::now).elapsed().as_secs_f64();
    let mut obj = json!({"t": (t * 1000.0).round() / 1000.0, "event": event});
    if let (Some(o), Value::Object(f)) = (obj.as_object_mut(), fields) {
        o.extend(f);
    }
    let mut out = std::io::stdout().lock();
    let _ = writeln!(out, "{obj}");
    let _ = out.flush();
}

#[derive(Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
#[serde(rename_all = "lowercase")]
pub enum Ipv6Mode {
    #[default]
    Off,
    Native,
    /// Native + a local ULA on the tun (`ifconfig-ipv6`), as in the openvpn man page example
    Native6,
    Common,
    /// variant of Common: local `unreachable` routes instead of the tunnel (immediate error, no packet sent)
    Reject,
}

#[derive(Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
#[serde(rename_all = "lowercase")]
enum ConfigVia {
    #[default]
    Pipe,
    Tempfile,
}

#[derive(Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
#[serde(rename_all = "lowercase")]
enum KeyMode {
    #[default]
    Inline,
    External,
}

/// What the GUI would send over IPC (D12) for a connect. Secrets live only in memory.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    profile: String,
    username: Option<String>,
    password: Option<String>,
    otp: Option<String>,
    key_passphrase: Option<String>,
    #[serde(default)]
    ipv6: Ipv6Mode,
    #[serde(default)]
    config_via: ConfigVia,
    #[serde(default)]
    key_mode: KeyMode,
    #[serde(default = "d_attempts")]
    max_attempts: u32,
    #[serde(default = "d_hold")]
    hold_secs: u64,
    /// 2.7 only: "default" keeps openvpn's own dns-updown, "disable" turns it off
    dns_updown: Option<String>,
    /// control run for 2.1: skip sanitization (proves the malicious profile works)
    #[serde(default)]
    unsafe_no_sanitize: bool,
    #[serde(default = "d_openvpn")]
    openvpn: String,
}
fn d_attempts() -> u32 { 3 }
fn d_hold() -> u64 { 20 }
fn d_openvpn() -> String { "/usr/sbin/openvpn".into() }

enum Outcome {
    Connected,
    AuthFailed(String),
    NeedInput(String),
    Failed(String),
}

fn main() {
    START.get_or_init(Instant::now);
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("netcfg") => std::process::exit(netcfg::helper_main(args.get(2).map(String::as_str).unwrap_or(""))),
        Some("connect") => std::process::exit(connect()),
        _ => {
            eprintln!("usage: galleriad-spike connect < request.json | netcfg <socket>");
            std::process::exit(64);
        }
    }
}

fn openvpn_version(bin: &str) -> (u32, u32, String) {
    let out = Command::new(bin).arg("--version").output().map(|o| String::from_utf8_lossy(&o.stdout).to_string()).unwrap_or_default();
    let first = out.lines().next().unwrap_or("").to_string();
    let v = first.split_whitespace().nth(1).unwrap_or("0.0");
    let mut it = v.split(|c: char| !c.is_ascii_digit()).filter_map(|x| x.parse().ok());
    (it.next().unwrap_or(0), it.next().unwrap_or(0), first)
}

fn connect() -> i32 {
    let mut raw = String::new();
    if std::io::stdin().read_to_string(&mut raw).is_err() {
        return 64;
    }
    let req: Request = match serde_json::from_str(&raw) {
        Ok(r) => r,
        Err(e) => {
            emit("error", json!({"message": format!("bad request: {e}")}));
            return 64;
        }
    };
    drop(raw);
    let (maj, min, vline) = openvpn_version(&req.openvpn);
    emit("openvpn_version", json!({"version": vline}));
    if (maj, min) < (2, 6) {
        emit("error", json!({"message": "OpenVPN 2.6 o successivo richiesto"}));
        return 69;
    }

    // session directory: /run/galleria is 0700 root (M5)
    let base = "/run/galleria";
    let _ = std::fs::DirBuilder::new().mode(0o700).create(base);
    let _ = std::fs::set_permissions(base, std::fs::Permissions::from_mode(0o700));
    let sess = format!("{base}/s{}", std::process::id());
    std::fs::DirBuilder::new().mode(0o700).create(&sess).expect("session dir");
    let mgmt = format!("{sess}/mgmt.sock");
    let netsock = format!("{sess}/netcfg.sock");
    let listener = UnixListener::bind(&netsock).expect("netcfg socket");
    let ipv6 = req.ipv6;
    std::thread::spawn(move || netcfg::serve(listener, ipv6));

    // generated configuration
    let san = sanitize::sanitize(&req.profile, req.key_mode == KeyMode::External);
    let mut cfg = if req.unsafe_no_sanitize { req.profile.clone() + "\n" } else { san.text.clone() };
    if !req.unsafe_no_sanitize {
        for r in &san.removed {
            emit("profile_directive_removed", json!({"line": r.line, "directive": r.directive, "reason": r.reason}));
        }
    } else {
        emit("warning", json!({"message": "UNSAFE control run: profile not sanitized"}));
    }
    let me = std::env::current_exe().unwrap().to_string_lossy().to_string();
    let mut ours = vec![
        format!("management {mgmt} unix"),
        "management-client-user root".into(),
        "management-hold".into(),
        "management-query-passwords".into(),
        "auth-retry none".into(),
        "connect-retry-max 1".into(),
        "server-poll-timeout 5".into(),
        "verb 3".into(),
    ];
    if !req.unsafe_no_sanitize {
        ours.push("script-security 2".into());
        ours.push(format!("up \"{me} netcfg {netsock}\""));
        ours.push(format!("down \"{me} netcfg {netsock}\""));
    }
    if (maj, min) >= (2, 7) {
        if let Some(d) = &req.dns_updown {
            if d == "disable" {
                ours.push("dns-updown disable".into());
            }
        }
    }
    if req.ipv6 == Ipv6Mode::Native || req.ipv6 == Ipv6Mode::Native6 {
        ours.push("redirect-gateway ipv6".into());
        ours.push("block-ipv6".into());
    }
    if req.ipv6 == Ipv6Mode::Native6 {
        // example from the openvpn man page (block-ipv6): local-only ULA on the tun, never sent to the server
        ours.push("ifconfig-ipv6 fd15:53b6:dead::2/64 fd15:53b6:dead::1".into());
    }
    if req.key_mode == KeyMode::External {
        ours.push("management-external-key".into());
    }
    emit("generated_config", json!({"daemon_directives": ours,
        "profile_directives": directive_lines(&cfg),
        "config_via": format!("{:?}", req.config_via), "key_mode": format!("{:?}", req.key_mode)}));
    cfg.push_str("\n# --- galleriad ---\n");
    for l in &ours {
        cfg.push_str(l);
        cfg.push('\n');
    }

    let ext_key = if req.key_mode == KeyMode::External {
        let pem = san.inline_key.clone().unwrap_or_default();
        match p256::SecretKey::from_sec1_pem(&pem).map(p256::ecdsa::SigningKey::from).or_else(|_| {
            use p256::pkcs8::DecodePrivateKey;
            p256::ecdsa::SigningKey::from_pkcs8_pem(&pem)
        }) {
            Ok(k) => Some(k),
            Err(e) => {
                emit("error", json!({"message": format!("external key: cannot parse P-256 key: {e}")}));
                return 65;
            }
        }
    } else {
        None
    };

    let mut code = 4;
    for attempt in 1..=req.max_attempts.max(1) {
        emit("attempt", json!({"n": attempt, "of": req.max_attempts}));
        match run_once(&req, &cfg, &sess, &mgmt, ext_key.as_ref()) {
            Outcome::Connected => {
                code = 0;
                break;
            }
            Outcome::AuthFailed(m) => {
                emit("error", json!({"kind": "auth_failed", "message": m, "retry": false}));
                code = 2;
                break;
            }
            Outcome::NeedInput(m) => {
                emit("error", json!({"kind": "need_input", "message": m, "retry": false}));
                code = 3;
                break;
            }
            Outcome::Failed(m) => {
                if attempt < req.max_attempts {
                    let backoff = 1u64 << (attempt - 1).min(5);
                    emit("retry", json!({"reason": m, "after_secs": backoff, "next_attempt": attempt + 1}));
                    std::thread::sleep(Duration::from_secs(backoff));
                } else {
                    emit("error", json!({"kind": "gave_up", "message": format!("Server non raggiungibile dopo {} tentativi: {m}", req.max_attempts), "retry": false}));
                }
            }
        }
    }
    let _ = std::fs::remove_dir_all(&sess);
    emit("exit", json!({"code": code}));
    code
}

/// Directive lines of a config, inline blocks shown only by tag (no key material in logs).
fn directive_lines(cfg: &str) -> Vec<String> {
    let mut v = Vec::new();
    let mut inside: Option<String> = None;
    for l in cfg.lines().map(str::trim) {
        if let Some(t) = &inside {
            if l == format!("</{t}>") {
                inside = None;
            }
            continue;
        }
        if let Some(tag) = l.strip_prefix('<').and_then(|x| x.strip_suffix('>')) {
            if !tag.starts_with('/') && tag != "connection" {
                v.push(format!("<{tag}> [inline, {} omitted]", "content"));
                inside = Some(tag.to_string());
                continue;
            }
        }
        if !l.is_empty() && !l.starts_with('#') && !l.starts_with(';') {
            v.push(l.to_string());
        }
    }
    v
}

fn quote(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

fn run_once(req: &Request, cfg: &str, sess: &str, mgmt: &str, ext_key: Option<&p256::ecdsa::SigningKey>) -> Outcome {
    let _ = std::fs::remove_file(mgmt);
    let tmpcfg = format!("{sess}/config.ovpn");
    let mut cmd = Command::new(&req.openvpn);
    match req.config_via {
        ConfigVia::Pipe => {
            cmd.args(["--config", "/dev/stdin"]).stdin(Stdio::piped());
        }
        ConfigVia::Tempfile => {
            let mut f = std::fs::OpenOptions::new();
            std::os::unix::fs::OpenOptionsExt::mode(&mut f, 0o600);
            f.write(true).create_new(true).open(&tmpcfg).unwrap().write_all(cfg.as_bytes()).unwrap();
            cmd.args(["--config", &tmpcfg]).stdin(Stdio::null());
        }
    }
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => return Outcome::Failed(format!("spawn: {e}")),
    };
    emit("openvpn_spawned", json!({"pid": child.id(), "argv": [&req.openvpn, "--config", if req.config_via == ConfigVia::Pipe { "/dev/stdin" } else { &tmpcfg }]}));
    if let Some(mut stdin) = child.stdin.take() {
        let c = cfg.to_string();
        std::thread::spawn(move || {
            let _ = stdin.write_all(c.as_bytes());
        });
    }
    for (pipe, name) in [(child.stdout.take().map(|p| Box::new(p) as Box<dyn Read + Send>), "stdout"), (child.stderr.take().map(|p| Box::new(p) as Box<dyn Read + Send>), "stderr")] {
        if let Some(p) = pipe {
            std::thread::spawn(move || {
                for l in BufReader::new(p).lines().map_while(Result::ok) {
                    eprintln!("openvpn[{name}]| {l}");
                }
            });
        }
    }

    // connect to the management socket
    let t0 = Instant::now();
    let stream = loop {
        if let Ok(s) = UnixStream::connect(mgmt) {
            break s;
        }
        if let Ok(Some(st)) = child.try_wait() {
            return Outcome::Failed(format!("openvpn exited before management was up ({st})"));
        }
        if t0.elapsed() > Duration::from_secs(10) {
            let _ = child.kill();
            return Outcome::Failed("management socket not available".into());
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    emit("mgmt_connected", json!({"socket": mgmt}));
    let mut w = stream.try_clone().unwrap();
    let (tx, rx) = mpsc::channel::<String>();
    std::thread::spawn(move || {
        for l in BufReader::new(stream).lines().map_while(Result::ok) {
            if tx.send(l).is_err() {
                break;
            }
        }
    });
    let mut send = |w: &mut UnixStream, c: &str| {
        let _ = w.write_all(format!("{c}\n").as_bytes());
    };

    let mut connected = false;
    let mut auth_failed: Option<String> = None;
    let mut need_input: Option<String> = None;
    let mut exit_reason = String::new();
    let mut deadline: Option<Instant> = None;
    let mut stop_sent = false;
    let mut last_bytes = Instant::now() - Duration::from_secs(10);
    let mut tempfile_removed = false;
    loop {
        if let Some(d) = deadline {
            if !stop_sent && Instant::now() >= d {
                emit("disconnect_requested", json!({}));
                send(&mut w, "signal SIGTERM");
                stop_sent = true;
            }
        }
        let line = match rx.recv_timeout(Duration::from_millis(200)) {
            Ok(l) => l,
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        };
        if let Some(rest) = line.strip_prefix(">INFO:") {
            emit("mgmt_info", json!({"info": rest}));
            send(&mut w, "version 3");
            send(&mut w, "state on");
            send(&mut w, "bytecount 1");
        } else if line.starts_with(">HOLD:") {
            emit("hold", json!({"line": line}));
            if req.config_via == ConfigVia::Tempfile && !tempfile_removed {
                // config already parsed: the fallback temp file lives only until here
                let _ = std::fs::remove_file(&tmpcfg);
                tempfile_removed = true;
                emit("tempfile_removed", json!({"path": tmpcfg}));
            }
            send(&mut w, "hold release");
        } else if let Some(rest) = line.strip_prefix(">STATE:") {
            let f: Vec<&str> = rest.split(',').collect();
            let state = f.get(1).copied().unwrap_or("");
            emit("state", json!({"state": state, "detail": f.get(2), "local_ip": f.get(3), "remote": f.get(4), "raw": rest}));
            if state == "CONNECTED" && !connected {
                connected = true;
                emit("connected", json!({"ip": f.get(3), "server": f.get(4)}));
                deadline = Some(Instant::now() + Duration::from_secs(req.hold_secs));
            }
            if state == "EXITING" {
                exit_reason = f.get(2).unwrap_or(&"").to_string();
            }
        } else if let Some(rest) = line.strip_prefix(">BYTECOUNT:") {
            if last_bytes.elapsed() >= Duration::from_millis(900) {
                let mut it = rest.split(',');
                emit("bytecount", json!({"in": it.next(), "out": it.next()}));
                last_bytes = Instant::now();
            }
        } else if let Some(rest) = line.strip_prefix(">PASSWORD:") {
            emit("mgmt_password", json!({"line": rest}));
            if rest.starts_with("Need 'Auth'") {
                match (&req.username, &req.password) {
                    (Some(u), Some(p)) => {
                        let sc = rest.find("SC:").map(|i| &rest[i + 3..]);
                        let pw = match (sc, &req.otp) {
                            (Some(scs), Some(otp)) => {
                                let flags: u32 = scs.split(',').next().and_then(|x| x.parse().ok()).unwrap_or(0);
                                if flags & 2 != 0 {
                                    // 2.7 "concat" format
                                    format!("{p}{otp}")
                                } else {
                                    let b = base64::engine::general_purpose::STANDARD;
                                    format!("SCRV1:{}:{}", b.encode(p), b.encode(otp))
                                }
                            }
                            (Some(_), None) => {
                                need_input = Some("Il server chiede un codice OTP".into());
                                send(&mut w, "signal SIGTERM");
                                continue;
                            }
                            _ => p.clone(),
                        };
                        send(&mut w, &format!("username \"Auth\" {}", quote(u)));
                        send(&mut w, &format!("password \"Auth\" {}", quote(&pw)));
                        emit("credentials_sent", json!({"type": "Auth", "static_challenge": sc.is_some()}));
                    }
                    _ => {
                        need_input = Some("Servono nome utente e password".into());
                        send(&mut w, "signal SIGTERM");
                    }
                }
            } else if rest.starts_with("Need 'Private Key'") {
                match &req.key_passphrase {
                    Some(pp) => {
                        send(&mut w, &format!("password \"Private Key\" {}", quote(pp)));
                        emit("credentials_sent", json!({"type": "Private Key"}));
                    }
                    None => {
                        need_input = Some("Serve la passphrase della chiave privata".into());
                        send(&mut w, "signal SIGTERM");
                    }
                }
            } else if rest.starts_with("Verification Failed: 'Auth'") {
                auth_failed = Some("Accesso negato dal server: nome utente, password o codice OTP non validi.".into());
            } else if rest.starts_with("Verification Failed: 'Private Key'") {
                auth_failed = Some("Passphrase della chiave privata non valida.".into());
            }
        } else if let Some(rest) = line.strip_prefix(">PK_SIGN:") {
            let mut parts = rest.splitn(2, ',');
            let data = parts.next().unwrap_or("");
            let algo = parts.next().unwrap_or("");
            let b = base64::engine::general_purpose::STANDARD;
            let digest = b.decode(data).unwrap_or_default();
            emit("pk_sign_request", json!({"algorithm": algo, "data_len": digest.len()}));
            match ext_key {
                Some(k) => {
                    use p256::ecdsa::signature::hazmat::PrehashSigner;
                    let sig: Result<p256::ecdsa::Signature, _> = k.sign_prehash(&digest);
                    match sig {
                        Ok(s) => {
                            send(&mut w, "pk-sig");
                            send(&mut w, &b.encode(s.to_der().as_bytes()));
                            send(&mut w, "END");
                            emit("pk_sign_done", json!({}));
                        }
                        Err(e) => emit("error", json!({"message": format!("sign: {e}")})),
                    }
                }
                None => emit("error", json!({"message": "PK_SIGN without external key"})),
            }
        } else if let Some(rest) = line.strip_prefix(">FATAL:") {
            emit("fatal", json!({"message": rest}));
        } else if line.starts_with("ERROR:") {
            emit("mgmt_error", json!({"line": line}));
        } else if line.starts_with(">LOG:") || line.starts_with("SUCCESS:") || line.starts_with(">NEED-OK") {
            // ignored in the spike
        } else if line.starts_with('>') {
            emit("mgmt_other", json!({"line": line}));
        }
    }
    let status = child.wait();
    emit("openvpn_exited", json!({"status": format!("{status:?}"), "exit_reason": exit_reason}));
    if let Some(m) = auth_failed {
        return Outcome::AuthFailed(m);
    }
    if exit_reason.contains("auth-failure") {
        return Outcome::AuthFailed("Accesso negato dal server (AUTH_FAILED).".into());
    }
    if let Some(m) = need_input {
        return Outcome::NeedInput(m);
    }
    if connected && !stop_sent {
        // the tunnel was up and openvpn died without us asking: relaunch (config is still in memory)
        return Outcome::Failed(format!("openvpn exited unexpectedly after connect ({exit_reason})"));
    }
    if connected {
        return Outcome::Connected;
    }
    Outcome::Failed(if exit_reason.is_empty() { "connection failed".into() } else { exit_reason })
}
