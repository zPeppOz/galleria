//! Named pipe server with an ACL (M1): SYSTEM and Administrators only in this spike.
//! One client at a time, one request line -> one reply line.

use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::os::windows::io::FromRawHandle;
use std::time::Duration;

use windows_sys::Win32::Foundation::{LocalFree, HANDLE, INVALID_HANDLE_VALUE};
use windows_sys::Win32::Security::Authorization::ConvertStringSecurityDescriptorToSecurityDescriptorW;
use windows_sys::Win32::Security::SECURITY_ATTRIBUTES;
use windows_sys::Win32::Storage::FileSystem::{FlushFileBuffers, PIPE_ACCESS_DUPLEX};
use windows_sys::Win32::System::Pipes::{ConnectNamedPipe, CreateNamedPipeW, DisconnectNamedPipe, GetNamedPipeClientProcessId};

/// D: protected DACL, generic-all to SYSTEM (SY) and to the Administrators group (BA). Nobody else.
pub const SDDL: &str = "D:P(A;;GA;;;SY)(A;;GA;;;BA)";
const FILE_FLAG_FIRST_PIPE_INSTANCE: u32 = 0x0008_0000;
const PIPE_REJECT_REMOTE_CLIENTS: u32 = 0x0000_0008;
const MAX_REQUEST: u64 = 256 * 1024;

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

pub fn serve(name: &str, handler: fn(&str, &str) -> String) -> std::io::Result<()> {
    let h = unsafe {
        let mut sd = std::ptr::null_mut();
        if ConvertStringSecurityDescriptorToSecurityDescriptorW(wide(SDDL).as_ptr(), 1, &mut sd, std::ptr::null_mut()) == 0 {
            return Err(std::io::Error::last_os_error());
        }
        let sa = SECURITY_ATTRIBUTES { nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32, lpSecurityDescriptor: sd, bInheritHandle: 0 };
        // FIRST_PIPE_INSTANCE: fail if another process already owns a pipe with this name (squatting)
        let h = CreateNamedPipeW(wide(name).as_ptr(), PIPE_ACCESS_DUPLEX | FILE_FLAG_FIRST_PIPE_INSTANCE, PIPE_REJECT_REMOTE_CLIENTS, 1, 65536, 65536, 0, &sa);
        let err = std::io::Error::last_os_error();
        LocalFree(sd as _);
        if h == INVALID_HANDLE_VALUE {
            return Err(err);
        }
        h as usize
    };
    std::thread::spawn(move || loop {
        let h = h as HANDLE;
        unsafe {
            if ConnectNamedPipe(h, std::ptr::null_mut()) == 0 && std::io::Error::last_os_error().raw_os_error() != Some(535) {
                std::thread::sleep(Duration::from_millis(100));
                continue;
            }
            let mut pid = 0u32;
            GetNamedPipeClientProcessId(h, &mut pid);
            let f = std::mem::ManuallyDrop::new(File::from_raw_handle(h as _));
            if let (Ok(r), Ok(mut w)) = (f.try_clone(), f.try_clone()) {
                let mut line = String::new();
                use std::io::Read;
                if BufReader::new(r.take(MAX_REQUEST)).read_line(&mut line).is_ok() && !line.trim().is_empty() {
                    let reply = handler(line.trim(), &format!("pid {pid}"));
                    let _ = w.write_all(format!("{reply}\n").as_bytes());
                }
            }
            FlushFileBuffers(h);
            DisconnectNamedPipe(h);
        }
    });
    Ok(())
}
