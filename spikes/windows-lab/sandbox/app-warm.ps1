# Dentro la Sandbox: copia l'app Tauri in C:\gl\app e scalda la build (dipendenze) in background
$ErrorActionPreference = 'Continue'
$env:RUSTUP_HOME = 'C:\tools\rustup'; $env:CARGO_HOME = 'C:\tools\cargo'
$env:Path += ';C:\tools\cargo\bin;C:\tools\w64devkit\bin'
# w64devkit non ha libgcc_eh (le funzioni stanno in libgcc) ma rustc la chiede al linker: archivio vuoto
$lib = (Get-ChildItem C:\tools\w64devkit\lib\gcc\x86_64-w64-mingw32 -Directory | Select-Object -First 1).FullName
if (-not (Test-Path "$lib\libgcc_eh.a")) { & ar.exe rcs "$lib\libgcc_eh.a" }
New-Item -ItemType Directory -Force C:\gl | Out-Null
Copy-Item C:\sbx\in\src\app C:\gl\ -Recurse -Force
Set-Location C:\gl\app\src-tauri
Remove-Item C:\sbx\out\app-warm.done -ErrorAction SilentlyContinue
Start-Process cmd.exe -ArgumentList '/c','cargo build --release > C:\sbx\out\app-warm.log 2>&1 & echo done > C:\sbx\out\app-warm.done' -WindowStyle Hidden
"warm build started"
