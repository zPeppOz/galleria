# Toolchain di build dentro la Sandbox (niente sull'host): Rust (host windows-gnu), w64devkit (gcc, windres), cargo-tauri
$ErrorActionPreference = 'Continue'
Start-Transcript C:\sbx\out\toolchain-install.txt | Out-Null
$dl = 'C:\sbx\in\dl'
New-Item -ItemType Directory -Force C:\tools | Out-Null
$env:RUSTUP_HOME = 'C:\tools\rustup'; $env:CARGO_HOME = 'C:\tools\cargo'
& "$dl\rustup-init.exe" -y --default-host x86_64-pc-windows-gnu --default-toolchain stable --profile minimal --no-modify-path 2>&1 | Select-Object -Last 5
& "$dl\w64devkit.exe" -oC:\tools -y | Select-Object -Last 2
Expand-Archive "$dl\cargo-tauri.zip" C:\tools\cargo\bin -Force
[Environment]::SetEnvironmentVariable('RUSTUP_HOME', 'C:\tools\rustup', 'Machine')
[Environment]::SetEnvironmentVariable('CARGO_HOME', 'C:\tools\cargo', 'Machine')
[Environment]::SetEnvironmentVariable('Path', [Environment]::GetEnvironmentVariable('Path','Machine') + ';C:\tools\cargo\bin;C:\tools\w64devkit\bin', 'Machine')
$env:Path += ';C:\tools\cargo\bin;C:\tools\w64devkit\bin'
rustc -vV; cargo -V; cargo tauri -V; gcc --version | Select-Object -First 1; windres --version | Select-Object -First 1
"cpu: $((Get-CimInstance Win32_ComputerSystem).NumberOfLogicalProcessors) ram: $([math]::Round((Get-CimInstance Win32_ComputerSystem).TotalPhysicalMemory/1GB,1)) GB"
"webview2: " + (Get-ItemProperty 'HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}' -ErrorAction SilentlyContinue).pv
Stop-Transcript | Out-Null
