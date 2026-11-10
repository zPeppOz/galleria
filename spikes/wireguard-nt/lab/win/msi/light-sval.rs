// Spike 1 (throwaway): stands in for WiX light.exe and adds -sval (skip ICE validation).
// Needed only where Windows has no VBScript engine (Windows Sandbox 26100): the ICE checks are VBScript custom
// actions and fail with error 2738. Tauri has no option to pass -sval to light.
fn main() {
    let real = std::env::current_exe().unwrap().with_file_name("light-real.exe");
    let status = std::process::Command::new(real).arg("-sval").args(std::env::args_os().skip(1)).status().unwrap();
    std::process::exit(status.code().unwrap_or(1));
}
