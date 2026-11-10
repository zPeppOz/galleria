# Libreria dei criteri dello spike 2 su Windows (dentro la Sandbox). Dot-source: . C:\sbx\in\win\lib.ps1
$ErrorActionPreference = 'Continue'
$GL = 'C:\gl'; $SRC = Get-Item C:\sbx\in\bin\galleriad-spike.exe; $DD = "$GL\d$($SRC.LastWriteTimeUtc.Ticks)"; $D = "$DD\galleriad-spike.exe"; $REQ = 'C:\sbx\in\req'; $EV = 'C:\sbx\out\ev'; $LOG = "$GL\log"
New-Item -ItemType Directory -Force $EV, $LOG, "$GL\req" | Out-Null
# un binario per versione in una cartella sua: una prova rimasta appesa non blocca la copia del binario nuovo
if (-not (Test-Path $D)) { New-Item -ItemType Directory -Force $DD | Out-Null; Copy-Item $SRC.FullName $D }

function Hdr($t) { ""; "=== $t ==="; (Get-Date).ToUniversalTime().ToString('s') + 'Z'; (& 'C:\Program Files\OpenVPN\bin\openvpn.exe' --version | Select-Object -First 1); "" }
function Run($cmd) { "PS> $cmd"; try { Invoke-Expression $cmd 2>&1 | Out-String -Width 220 } catch { "ERROR: $_" }; "" }

# Lancia il demone come SYSTEM in sessione 0 (attivita' pianificata), come farebbe il servizio: la richiesta
# (profilo + segreti, quello che la GUI manderebbe via IPC) arriva su stdin, gli eventi vanno in C:\gl\log\<nome>-<n>.log.
# Torna quando compare "connected". Richieste e log sono attrezzi della prova, non file del demone.
$script:LOGS = @{}
function LogOf($name) { $script:LOGS[$name] }
function Start-Conn($name, [int]$waitSecs = 40, [switch]$NoWait) {
  $log = "$LOG\$name-$([DateTime]::UtcNow.Ticks).log"; $script:LOGS[$name] = $log
  Copy-Item "$REQ\$name.json" "$GL\req\$name.json" -Force
  $a = New-ScheduledTaskAction -Execute 'cmd.exe' -Argument "/c `"$D connect < $GL\req\$name.json > $log 2>&1`""
  $p = New-ScheduledTaskPrincipal -UserId 'SYSTEM' -RunLevel Highest
  Register-ScheduledTask -TaskName "gl-$name" -Action $a -Principal $p -Force | Out-Null
  Start-ScheduledTask -TaskName "gl-$name"
  if ($NoWait) { return $true }
  $t = [Diagnostics.Stopwatch]::StartNew()
  while ($t.Elapsed.TotalSeconds -lt $waitSecs) {
    if ((Test-Path $log) -and (Select-String -Path $log -Pattern '"event":"connected"' -Quiet)) { return $true }
    if ((Test-Path $log) -and (Select-String -Path $log -Pattern '"event":"exit"' -Quiet)) { return $false }
    Start-Sleep -Milliseconds 200
  }
  return $false
}
function Wait-Event($name, $event, [int]$waitSecs = 60) {
  $t = [Diagnostics.Stopwatch]::StartNew()
  while ($t.Elapsed.TotalSeconds -lt $waitSecs) {
    if ((Test-Path (LogOf $name)) -and (Select-String -Path (LogOf $name) -Pattern "`"event`":`"$event`"" -Quiet)) { return $true }
    Start-Sleep -Milliseconds 200
  }
  return $false
}
function Wait-Conn($name, [int]$waitSecs = 200) {
  [void](Wait-Event $name 'exit' $waitSecs)
  $log = LogOf $name
  $e = if (Test-Path $log) { Select-String -Path $log -Pattern '"event":"exit"' | Select-Object -Last 1 }
  "[daemon $(if ($e) { $e.Line } else { 'NOT EXITED' })]"
  Unregister-ScheduledTask -TaskName "gl-$name" -Confirm:$false -ErrorAction SilentlyContinue
  Remove-Item "$GL\req\$name.json" -ErrorAction SilentlyContinue
  if (Test-Path $log) { Copy-Item $log "$EV\$name.log" -Force }
}
function Grep-Log($name, $pattern, [int]$width = 420) { if (Test-Path (LogOf $name)) { Select-String -Path (LogOf $name) -Pattern $pattern | ForEach-Object { $_.Line.Substring(0, [Math]::Min($width, $_.Line.Length)) } } }
function Ev($name, $event) { Get-Content (LogOf $name) | Where-Object { $_ -like "*`"event`":`"$event`"*" } | ForEach-Object { $_ | ConvertFrom-Json } }
function Mgmt-Port($name) { ((Get-Content (LogOf $name) | Select-String 'management 127.0.0.1 (\d+)' | Select-Object -First 1).Matches[0].Groups[1].Value) }
function Procs { Get-CimInstance Win32_Process | Where-Object { $_.Name -in 'openvpn.exe','galleriad-spike.exe' } | ForEach-Object { "pid=$($_.ProcessId) ppid=$($_.ParentProcessId) session=$($_.SessionId) owner=$((Invoke-CimMethod -InputObject $_ -MethodName GetOwner).User) cmdline=$($_.CommandLine)" } }
# Cerca una stringa nei file delle cartelle dove un segreto potrebbe finire. Richieste e log della prova stanno in
# C:\gl (fuori da queste cartelle): sono attrezzi della prova, non file del demone.
function Find-OnDisk($needle, $what) {
  $roots = 'C:\ProgramData','C:\Windows\Temp','C:\Users','C:\Program Files\OpenVPN','C:\Windows\System32\config\systemprofile'
  $hits = @()
  foreach ($r in $roots) {
    $hits += Get-ChildItem $r -Recurse -File -Force -ErrorAction SilentlyContinue | Where-Object { $_.Length -lt 20MB } |
      Select-String -SimpleMatch -Pattern $needle -List -ErrorAction SilentlyContinue | ForEach-Object { $_.Path }
  }
  "ricerca di [$what] in $($roots -join ', '): $($hits.Count) file"
  $hits | ForEach-Object { "  TROVATO: $_" }
}
# Esegue il binario di prova come utente locale senza privilegi (creato qui, solo nella Sandbox)
function As-User($argline) {
  $pw = ConvertTo-SecureString 'Gl-spike-2026!x' -AsPlainText -Force
  if (-not (Get-LocalUser gltest -ErrorAction SilentlyContinue)) { New-LocalUser gltest -Password $pw -PasswordNeverExpires | Out-Null; Add-LocalGroupMember -Group 'Users' -Member gltest -ErrorAction SilentlyContinue }
  $cred = New-Object Management.Automation.PSCredential('gltest', $pw)
  $out = "C:\Users\Public\gltest-$([DateTime]::UtcNow.Ticks).txt"
  try {
    $p = Start-Process $D -ArgumentList $argline -Credential $cred -WorkingDirectory C:\Users\Public -RedirectStandardOutput $out -RedirectStandardError "$out.err" -WindowStyle Hidden -PassThru -ErrorAction Stop
    $p.WaitForExit(20000) | Out-Null
    "[come utente gltest, exit $($p.ExitCode)]"
    Get-Content $out, "$out.err" -ErrorAction SilentlyContinue
  } catch { "As-User: non riuscito a lanciare il processo come gltest: $_" }
}
