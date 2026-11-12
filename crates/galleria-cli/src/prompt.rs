use crate::commands::{CliError, failure};
use galleria_ipc::*;
use nix::sys::termios::{self, LocalFlags, SetArg, Termios};
use std::{
    fs::File,
    io::{self, BufRead, BufReader, Write},
    os::fd::AsFd,
};
use zeroize::Zeroizing;
fn line(reader: &mut impl BufRead) -> Result<Secret, CliError> {
    let mut value = Zeroizing::new(String::new());
    if reader
        .read_line(&mut value)
        .map_err(|_| failure("needs_input"))?
        == 0
    {
        return Err(failure("needs_input"));
    }
    if value.ends_with('\n') {
        value.pop();
        if value.ends_with('\r') {
            value.pop();
        }
    }
    Ok(Secret::new(value.as_str()))
}
pub fn stdin_secrets(
    reader: &mut impl BufRead,
    password: bool,
    otp: bool,
) -> Result<InputValues, CliError> {
    Ok(InputValues {
        password: if password { Some(line(reader)?) } else { None },
        otp: if otp { Some(line(reader)?) } else { None },
        ..Default::default()
    })
}
struct Restore<'a, F: AsFd> {
    fd: &'a F,
    attributes: Termios,
}
impl<F: AsFd> Drop for Restore<'_, F> {
    fn drop(&mut self) {
        let _ = termios::tcsetattr(self.fd, SetArg::TCSANOW, &self.attributes);
    }
}
pub fn read_hidden_from(tty: &File) -> Result<Secret, CliError> {
    hidden_from(tty, || Ok(()))
}
fn hidden_from(
    tty: &File,
    before_read: impl FnOnce() -> Result<(), CliError>,
) -> Result<Secret, CliError> {
    let attributes = termios::tcgetattr(tty).map_err(|_| failure("needs_input"))?;
    let mut hidden = attributes.clone();
    hidden
        .local_flags
        .remove(LocalFlags::ECHO | LocalFlags::ECHONL);
    termios::tcsetattr(tty, SetArg::TCSANOW, &hidden).map_err(|_| failure("needs_input"))?;
    let _restore = Restore {
        fd: tty,
        attributes,
    };
    before_read()?;
    line(&mut BufReader::new(tty))
}
pub fn require_terminal(_request: &InputRequest, terminal: bool) -> Result<(), CliError> {
    if terminal {
        Ok(())
    } else {
        Err(failure("needs_input"))
    }
}
fn ask(prompt: &str) -> Result<Secret, CliError> {
    let tty = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/tty")
        .map_err(|_| failure("needs_input"))?;
    let value = prompt_hidden_from(&tty, prompt, &mut io::stderr());
    eprintln!();
    value
}
pub fn request_input(
    request: &InputRequest,
    username: &mut Option<String>,
) -> Result<InputValues, CliError> {
    use std::io::IsTerminal;
    require_terminal(request, io::stdin().is_terminal())?;
    Ok(match request.kind {
        InputKind::Credentials => {
            if username.is_none() {
                *username = Some(ask("Username")?.expose().into());
            }
            InputValues {
                username: username.clone(),
                password: Some(ask("Password")?),
                ..Default::default()
            }
        }
        InputKind::Otp => InputValues {
            otp: Some(ask(&request.prompt)?),
            ..Default::default()
        },
        InputKind::KeyPassphrase => InputValues {
            key_passphrase: Some(ask("Private key passphrase")?),
            ..Default::default()
        },
    })
}

pub fn prompt_hidden_from(
    tty: &File,
    prompt: &str,
    output: &mut impl Write,
) -> Result<Secret, CliError> {
    hidden_from(tty, || {
        write!(output, "{prompt}: ").map_err(|_| failure("internal"))?;
        output.flush().map_err(|_| failure("internal"))
    })
}
