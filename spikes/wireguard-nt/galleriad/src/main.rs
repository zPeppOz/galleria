//! galleriad — spike 1 (throwaway). A Windows service (LocalSystem) that brings up a WireGuard
//! tunnel with WireGuardNT (`wireguard.dll`, API of `wireguard.h` through the `wireguard-nt` crate),
//! reads handshake and byte counters every second and talks JSON-RPC 2.0, one message per line,
//! on a named pipe with an ACL (D12, M1).
//!
//!   galleriad service              run under the Service Control Manager
//!   galleriad cli hello            | status | disconnect
//!   galleriad cli connect <file.conf> [none|route|firewall]   (third argument: IPv6 block, criterion 1.5)
//!
//! The client sends the *content* of the .conf, never a path: the daemon reads no file on behalf
//! of a caller (M1).

#[cfg(not(windows))]
fn main() {
    eprintln!("galleriad (spike 1) is Windows only");
}

#[cfg(windows)]
mod pipe;
#[cfg(windows)]
mod tunnel;

#[cfg(windows)]
fn main() {
    win::main()
}

#[cfg(windows)]
mod win {
    use std::ffi::OsString;
    use std::io::{BufRead, BufReader, Write};
    use std::sync::mpsc;
    use std::time::Duration;

    use serde_json::{json, Value};
    use windows_service::service::{ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus, ServiceType};
    use windows_service::service_control_handler::{self, ServiceControlHandlerResult};
    use windows_service::{define_windows_service, service_dispatcher};

    use crate::{pipe, tunnel};

    pub const PIPE: &str = r"\\.\pipe\galleria-spike";
    const SERVICE: &str = "galleriad";

    pub fn log(msg: &str) {
        let dir = r"C:\ProgramData\galleria-spike";
        let _ = std::fs::create_dir_all(dir);
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(format!(r"{dir}\galleriad.log")) {
            let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs_f64()).unwrap_or(0.0);
            let _ = writeln!(f, "{t:.3} galleriad[{}] {msg}", std::process::id());
        }
    }

    pub fn main() {
        let args: Vec<String> = std::env::args().collect();
        match args.get(1).map(String::as_str) {
            Some("service") => {
                if let Err(e) = service_dispatcher::start(SERVICE, ffi_service_main) {
                    log(&format!("service dispatcher: {e}"));
                    std::process::exit(1);
                }
            }
            Some("cli") => std::process::exit(cli(&args[2..])),
            _ => {
                eprintln!("usage: galleriad service | cli hello|status|disconnect | cli connect <file.conf> [none|route|firewall]");
                std::process::exit(64);
            }
        }
    }

    define_windows_service!(ffi_service_main, service_main);

    fn service_main(_args: Vec<OsString>) {
        let (tx, rx) = mpsc::channel::<()>();
        let handler = move |control| match control {
            ServiceControl::Stop | ServiceControl::Shutdown => {
                let _ = tx.send(());
                ServiceControlHandlerResult::NoError
            }
            ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
            _ => ServiceControlHandlerResult::NotImplemented,
        };
        let Ok(status) = service_control_handler::register(SERVICE, handler) else { return };
        let set = |state, accept| {
            let _ = status.set_service_status(ServiceStatus {
                service_type: ServiceType::OWN_PROCESS,
                current_state: state,
                controls_accepted: accept,
                exit_code: ServiceExitCode::Win32(0),
                checkpoint: 0,
                wait_hint: Duration::from_secs(10),
                process_id: None,
            });
        };
        log(&format!("service starting, version {}", env!("CARGO_PKG_VERSION")));
        match pipe::serve(PIPE, handle_request) {
            Ok(()) => log(&format!("ipc: listening on {PIPE}, acl {}", pipe::SDDL)),
            Err(e) => log(&format!("ipc: cannot create pipe: {e}")),
        }
        tunnel::start_stats_thread();
        set(ServiceState::Running, ServiceControlAccept::STOP | ServiceControlAccept::SHUTDOWN);
        let _ = rx.recv();
        set(ServiceState::StopPending, ServiceControlAccept::empty());
        log("service stopping: closing the tunnel if any");
        tunnel::disconnect();
        set(ServiceState::Stopped, ServiceControlAccept::empty());
        log("service stopped");
    }

    /// One JSON-RPC request -> one reply. Unknown methods and unknown fields are refused.
    fn handle_request(line: &str, client: &str) -> String {
        let req: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(e) => return json!({"jsonrpc": "2.0", "id": null, "error": {"code": -32700, "message": e.to_string()}}).to_string(),
        };
        let id = req.get("id").cloned().unwrap_or(Value::Null);
        let method = req.get("method").and_then(Value::as_str).unwrap_or("");
        log(&format!("ipc: request `{method}` from {client}"));
        let result: Result<Value, String> = match method {
            "hello" => Ok(json!({"daemon": "galleriad", "version": env!("CARGO_PKG_VERSION"), "protocol": 1})),
            "connect" => {
                #[derive(serde::Deserialize)]
                #[serde(deny_unknown_fields)]
                struct P {
                    config: String,
                    #[serde(default)]
                    ipv6_block: tunnel::Ipv6Block,
                }
                serde_json::from_value::<P>(req.get("params").cloned().unwrap_or(Value::Null))
                    .map_err(|e| format!("bad params: {e}"))
                    .and_then(|p| tunnel::connect(&p.config, p.ipv6_block))
            }
            "status" => Ok(tunnel::status()),
            "disconnect" => Ok(json!({"was_connected": tunnel::disconnect()})),
            _ => Err(format!("unknown method `{method}`")),
        };
        match result {
            Ok(r) => json!({"jsonrpc": "2.0", "id": id, "result": r}),
            Err(m) => {
                log(&format!("ipc: `{method}` failed: {m}"));
                json!({"jsonrpc": "2.0", "id": id, "error": {"code": -32000, "message": m}})
            }
        }
        .to_string()
    }

    fn cli(args: &[String]) -> i32 {
        let req = match args.first().map(String::as_str) {
            Some("connect") => {
                let Some(conf) = args.get(1).and_then(|p| std::fs::read_to_string(p).ok()) else {
                    eprintln!("cannot read the .conf");
                    return 66;
                };
                json!({"jsonrpc": "2.0", "id": 1, "method": "connect", "params": {"config": conf, "ipv6_block": args.get(2).map(String::as_str).unwrap_or("none")}})
            }
            Some(m) => json!({"jsonrpc": "2.0", "id": 1, "method": m}),
            None => return 64,
        };
        let mut p = match std::fs::OpenOptions::new().read(true).write(true).open(PIPE) {
            Ok(p) => p,
            Err(e) => {
                println!("cannot open {PIPE}: {e} (os error {:?})", e.raw_os_error());
                return 5;
            }
        };
        if p.write_all(format!("{req}\n").as_bytes()).is_err() {
            return 1;
        }
        let mut line = String::new();
        let _ = BufReader::new(p).read_line(&mut line);
        println!("{}", line.trim());
        if line.contains("\"error\"") { 2 } else { 0 }
    }
}
