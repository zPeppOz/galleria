. C:\sbx\in\wgwin\msi-lib.ps1
function Latest($sigFile, $file) {
  $j = @{ version = '0.0.2'; notes = 'spike'; pub_date = '2026-11-10T19:00:00Z'
          platforms = @{ 'windows-x86_64' = @{ signature = (Get-Content $sigFile -Raw).Trim(); url = "http://127.0.0.1:8765/$file" } } } | ConvertTo-Json -Depth 5
  [IO.File]::WriteAllText("$WWW\latest.json", $j, (New-Object Text.UTF8Encoding($false)))
  "latest.json: version 0.0.2, url .../$file, signature da $sigFile (" + (Get-FileHash $sigFile -Algorithm SHA256).Hash.Substring(0, 16) + "...)"
}
function RunApp($wait) {
  $exe = Join-Path (InstallDir) 'galleria-spike-app.exe'; $n = LogCount $ALOG; $h = LogCount C:\gl\log\httpd.log
  "PS> `"$exe`""
  $p = Start-Process $exe -PassThru; $ok = $p.WaitForExit(120000); "[uscita entro 120 s: $ok, exit $($p.ExitCode)]"
  if ($wait) { 1..90 | ForEach-Object { if ((Product).DisplayVersion -ne '0.0.2' -or (Get-Service galleriad -ErrorAction SilentlyContinue).Status -ne 'Running' -or (Get-Process msiexec -ErrorAction SilentlyContinue | Where-Object SessionId -ne 0)) { Start-Sleep 2 } }; Start-Sleep 5 }
  "--- registro dell'app ---"; LogLines $ALOG $n
  "--- richieste al server locale ---"; LogLines C:\gl\log\httpd.log $h
  ""
}

Hdr "1.7 preparazione: 0.0.1 installata dall'MSI, server locale con latest.json"
if (-not (Product)) { Msiexec '/i' (Msi 0.0.1) "$OUT\1.7-install-0.0.1.log" }
State; Hello
Copy-Item (Msi 0.0.2) "$WWW\app-0.0.2.msi" -Force
$b = [IO.File]::ReadAllBytes("$WWW\app-0.0.2.msi"); $b[$b.Length - 1] = $b[$b.Length - 1] -bxor 1; [IO.File]::WriteAllBytes("$WWW\app-tampered.msi", $b)
Get-ChildItem $WWW -Filter *.msi | ForEach-Object { "{0,10}  {1}  {2}" -f $_.Length, (Get-FileHash $_.FullName -Algorithm SHA256).Hash, $_.Name }
if (-not (Get-NetTCPConnection -LocalPort 8765 -State Listen -ErrorAction SilentlyContinue)) {
  Start-Process powershell.exe -ArgumentList '-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', 'C:\sbx\in\wgwin\httpd.ps1' -WindowStyle Hidden; Start-Sleep 3
}
Run "Get-NetTCPConnection -LocalPort 8765 -State Listen | Format-Table LocalAddress, LocalPort, State -AutoSize"
"chiave pubblica nell'app (usa e getta, solo spike): " + (Get-Content C:\gl\keys\spike.key.pub -Raw).Trim().Substring(0, 40) + "..."
"chiave pubblica sbagliata:                          " + (Get-Content C:\gl\keys\wrong.key.pub -Raw).Trim().Substring(0, 40) + "..."

Hdr "1.7a firma sbagliata: MSI 0.0.2 buono, firmato con un'altra chiave -> rifiutato"
Latest "$REL\wrong\app.msi.sig" 'app-0.0.2.msi'
RunApp $false
State; Hello

Hdr "1.7b file alterato: firma giusta dell'MSI 0.0.2, ma il file scaricato ha un byte diverso -> rifiutato"
Latest "$((Msi 0.0.2)).sig" 'app-tampered.msi'
RunApp $false
State; Hello

Hdr "1.7c firma giusta: aggiornamento da 0.0.1 a 0.0.2"
$n = LogCount $SLOG; $t0 = Get-Date
Latest "$((Msi 0.0.2)).sig" 'app-0.0.2.msi'
RunApp $true
State; Hello
Run "sc.exe qc galleriad"
$dir = InstallDir
"--- il binario del servizio installato e' quello della 0.0.2 ---"
"installato: " + (Get-FileHash "$dir\galleriad.exe" -Algorithm SHA256).Hash
"0.0.1:      " + (Get-FileHash C:\sbx\in\msi\galleriad-0.0.1.exe -Algorithm SHA256).Hash
"0.0.2:      " + (Get-FileHash C:\sbx\in\msi\galleriad-0.0.2.exe -Algorithm SHA256).Hash
"--- registro del servizio durante l'aggiornamento ---"
LogLines $SLOG $n
"--- eventi MsiInstaller ---"
Get-WinEvent -FilterHashtable @{ LogName = 'Application'; ProviderName = 'MsiInstaller'; StartTime = $t0 } -ErrorAction SilentlyContinue | Sort-Object TimeCreated |
  ForEach-Object { "$($_.TimeCreated.ToUniversalTime().ToString('s'))Z id=$($_.Id) $(($_.Message -split "`n")[0].Trim())" }

Hdr "1.7d dopo l'aggiornamento: GUI e servizio alla stessa versione, nessun altro aggiornamento"
RunApp $false
State
