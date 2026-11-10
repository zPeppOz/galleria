# Libreria dei criteri 1.6 e 1.7 (dentro la Sandbox). Dot-source: . C:\sbx\in\wgwin\msi-lib.ps1
$ErrorActionPreference = 'Continue'
$REL = 'C:\gl\rel'; $OUT = 'C:\sbx\out\msi'; $WWW = 'C:\gl\www'
$SLOG = 'C:\ProgramData\galleria-spike\galleriad.log'; $ALOG = 'C:\ProgramData\galleria-spike\app.log'
New-Item -ItemType Directory -Force $OUT, $WWW, C:\gl\log | Out-Null

function Hdr($t) { ""; "=== $t ==="; (Get-Date).ToUniversalTime().ToString('s') + 'Z'; "" }
function Run($cmd) { "PS> $cmd"; try { Invoke-Expression $cmd 2>&1 | Out-String -Width 220 } catch { "ERROR: $_" }; "" }
function Msi($v) { (Get-ChildItem "$REL\$v" -Filter *.msi | Select-Object -First 1).FullName }
function Msiexec($verb, $msi, $log) {
  "PS> msiexec $verb `"$msi`" /qn /l*v $log"
  $p = Start-Process msiexec.exe -ArgumentList $verb, "`"$msi`"", '/qn', '/l*v', $log -PassThru; $p.WaitForExit()
  "[exit $($p.ExitCode)]"; ""
}
function Product { Get-ItemProperty 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\*', 'HKLM:\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\*' -ErrorAction SilentlyContinue | Where-Object DisplayName -eq 'Galleria Spike' }
function InstallDir { $s = Get-CimInstance Win32_Service -Filter "Name='galleriad'"; if ($s) { Split-Path ($s.PathName -replace '^"([^"]+)".*$', '$1') } }
function Hello { $d = InstallDir; if ($d) { "PS> `"$d\galleriad.exe`" cli hello"; & "$d\galleriad.exe" cli hello 2>&1; "[exit $LASTEXITCODE]" } else { "hello: servizio galleriad assente" }; "" }
function State {
  $p = Product; "prodotto MSI: " + $(if ($p) { "$($p.DisplayName) $($p.DisplayVersion) ($($p.PSChildName))" } else { 'assente' })
  $s = Get-CimInstance Win32_Service -Filter "Name='galleriad'"
  "servizio galleriad: " + $(if ($s) { "$($s.State), $($s.StartMode), $($s.StartName), pid $($s.ProcessId), $($s.PathName)" } else { 'assente' })
}
function LogLines($path, $from) { if (Test-Path $path) { Get-Content $path | Select-Object -Skip $from | Where-Object { $_ -notmatch 'stats:' } } }
function LogCount($path) { if (Test-Path $path) { @(Get-Content $path).Count } else { 0 } }
