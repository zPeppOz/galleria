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
impl CommandRunner for SystemRunner {
    fn run(&self, cmd: &Cmd) -> Result<(), NetError> {
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
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|_| failed("cannot start tool"))?;
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
        match result {
            Ok(true) => Ok(()),
            Ok(false) => Err(failed(&String::from_utf8_lossy(&stderr))),
            Err(e) => Err(e),
        }
    }
}
