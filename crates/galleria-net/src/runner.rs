use crate::{Cmd, NetError, Program};
use std::{
    collections::HashMap,
    io::Read,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    process::{Command, Stdio},
    time::{Duration, Instant},
};
pub trait CommandRunner: Send + Sync {
    fn run(&self, cmd: &Cmd) -> Result<(), NetError>;
}
pub struct SystemRunner {
    paths: HashMap<&'static str, PathBuf>,
}
impl SystemRunner {
    pub fn locate() -> Result<Self, NetError> {
        let mut paths = HashMap::new();
        for name in ["ip", "resolvectl", "sysctl"] {
            let path = ["/usr/sbin", "/usr/bin", "/sbin", "/bin"]
                .into_iter()
                .map(|dir| PathBuf::from(dir).join(name))
                .find(|p| {
                    p.metadata()
                        .is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
                });
            if let Some(path) = path {
                paths.insert(name, path);
            } // DNS absence only matters when DNS is requested.
        }
        if !paths.contains_key("ip") {
            return Err(NetError::ToolMissing("ip"));
        }
        Ok(Self { paths })
    }
}
impl SystemRunner {
    fn execute(&self, cmd: &Cmd) -> Result<Vec<u8>, NetError> {
        cmd.validate()?;
        let missing = if cmd.program == Program::Resolvectl {
            NetError::DnsUnavailable("resolvectl not installed".into())
        } else {
            NetError::ToolMissing(cmd.program.name())
        };
        let path = self.paths.get(cmd.program.name()).ok_or(missing)?;
        let failed = |stderr: &str| NetError::Command {
            cmd: cmd.to_string(),
            stderr: stderr.into(),
        };
        let mut child = Command::new(path)
            .args(&cmd.args)
            .env("LC_ALL", "C")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|_| failed("cannot start tool"))?;
        let stdout = child.stdout.take();
        let output_reader = std::thread::spawn(move || {
            let mut output = Vec::new();
            if let Some(mut stdout) = stdout {
                let mut chunk = [0u8; 1024];
                while let Ok(n) = stdout.read(&mut chunk) {
                    if n == 0 {
                        break;
                    }
                    let keep = n.min(4096usize.saturating_sub(output.len()));
                    output.extend_from_slice(&chunk[..keep]);
                }
            }
            output
        });
        let stderr = child.stderr.take();
        // Drain even beyond the retained limit so a noisy child cannot deadlock the runner.
        let reader = std::thread::spawn(move || {
            let mut output = Vec::new();
            if let Some(mut stderr) = stderr {
                let mut chunk = [0u8; 1024];
                while let Ok(n) = stderr.read(&mut chunk) {
                    if n == 0 {
                        break;
                    }
                    let keep = n.min(4096usize.saturating_sub(output.len()));
                    output.extend_from_slice(&chunk[..keep]);
                }
            }
            output
        });
        let deadline = Instant::now() + Duration::from_secs(2);
        let result = loop {
            match child.try_wait() {
                Ok(Some(status)) => break Ok(status.success()),
                Ok(None) if Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(10))
                }
                _ => {
                    let _ = child.kill();
                    let _ = child.wait();
                    break Err(failed("tool timed out or could not be waited"));
                }
            }
        };
        let stderr = reader.join().unwrap_or_default();
        let output = output_reader.join().unwrap_or_default();
        match result {
            Ok(true) => Ok(output),
            Ok(false) => Err(failed(&String::from_utf8_lossy(&stderr))),
            Err(e) => Err(e),
        }
    }
}

impl CommandRunner for SystemRunner {
    fn run(&self, cmd: &Cmd) -> Result<(), NetError> {
        let result = self.execute(cmd);
        let args = cmd.args.iter().map(String::as_str).collect::<Vec<_>>();
        if cmd.program == Program::Sysctl
            && args.as_slice() == ["-q", "-w", "net.ipv4.conf.all.src_valid_mark=1"]
        {
            // procps versions disagree about errors on a read-only /proc/sys.
            // The observable value, rather than the exit status alone, is authoritative.
            let query = Cmd {
                program: Program::Sysctl,
                args: ["-n", "net.ipv4.conf.all.src_valid_mark"]
                    .map(String::from)
                    .to_vec(),
            };
            if let Ok(value) = self.execute(&query)
                && String::from_utf8_lossy(&value).trim() == "1"
            {
                return Ok(());
            }
            return result.and_then(|_| {
                Err(NetError::Command {
                    cmd: cmd.to_string(),
                    stderr: "Required sysctl value was not applied".into(),
                })
            });
        }
        match result {
            Ok(_) => Ok(()),
            Err(error) => {
                let args = cmd.args.iter().map(String::as_str).collect::<Vec<_>>();
                if cmd.program == Program::Ip
                    && let [family, "route", "add", cidr, "dev", iface, tail @ ..] = args.as_slice()
                    && matches!(&error,NetError::Command {stderr,..} if stderr.contains("File exists"))
                {
                    let table = if tail.is_empty() { "main" } else { "51820" };
                    let query = Cmd {
                        program: Program::Ip,
                        args: [*family, "route", "show", "table", table, "exact", *cidr]
                            .map(String::from)
                            .to_vec(),
                    };
                    if let Ok(output) = self.execute(&query) {
                        let text = String::from_utf8_lossy(&output);
                        if text.lines().any(|line| {
                            let words = line.split_whitespace().collect::<Vec<_>>();
                            words.first().is_some_and(|s| {
                                *s == *cidr
                                    || (*s == "default" && matches!(*cidr, "0.0.0.0/0" | "::/0"))
                            }) && words.windows(2).any(|w| w == ["dev", *iface])
                        }) {
                            return Ok(());
                        }
                    }
                }
                Err(error)
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    #[test]
    fn existing_route_is_accepted_only_when_the_destination_uses_the_tunnel() {
        for (route, accepted) in [
            ("10.99.0.0/24 dev glr-wg0 proto kernel scope link", true),
            ("10.99.0.0/24 dev eth0 proto kernel scope link", false),
            ("", false),
        ] {
            let tmp = tempfile::tempdir().unwrap();
            let path = tmp.path().join("ip");
            let filtered = route.replace("dev glr-wg0 ", "").replace("dev eth0 ", "");
            std::fs::write(&path,format!("#!/bin/sh\nif [ \"$3\" = add ]; then echo 'RTNETLINK answers: File exists' >&2; exit 2; fi\nif [ \"$8\" = dev ]; then printf '%s\\n' '{filtered}'; else printf '%s\\n' '{route}'; fi\n")).unwrap();
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
            let runner = SystemRunner {
                paths: HashMap::from([("ip", path)]),
            };
            let cmd = Cmd {
                program: Program::Ip,
                args: ["-4", "route", "add", "10.99.0.0/24", "dev", "glr-wg0"]
                    .map(String::from)
                    .to_vec(),
            };
            assert_eq!(runner.run(&cmd).is_ok(), accepted, "{route}");
        }
    }
    #[test]
    fn sysctl_write_is_accepted_only_after_the_required_value_is_verified() {
        for (value, write_exit, accepted) in [
            ("1", 1, true),
            ("0", 1, false),
            ("", 1, false),
            ("1", 0, true),
            ("0", 0, false),
        ] {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("sysctl");
            std::fs::write(&path,format!("#!/bin/sh\nif [ \"$1\" = -n ]; then printf '%s\\n' '{value}'; exit 0; fi\necho 'permission denied' >&2\nexit {write_exit}\n")).unwrap();
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
            let runner = SystemRunner {
                paths: HashMap::from([("sysctl", path)]),
            };
            let cmd = Cmd {
                program: Program::Sysctl,
                args: ["-q", "-w", "net.ipv4.conf.all.src_valid_mark=1"]
                    .map(String::from)
                    .to_vec(),
            };
            assert_eq!(runner.run(&cmd).is_ok(), accepted, "value={value}");
        }
    }
}
