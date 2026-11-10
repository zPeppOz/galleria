# Dentro la Sandbox: (ri)avvia i relay locali. Ogni relay ascolta su 127.0.0.1:<porta> e porta i datagrammi
# nei file di C:\sbx\out\relay (vedi filerelay). I relay vecchi escono da soli quando trovano il file .stop.
$ErrorActionPreference = 'Continue'
$RD = 'C:\sbx\out\relay'; New-Item -ItemType Directory -Force $RD, C:\gl | Out-Null
$relays = @(@('11194','ovpn'), @('11195','ovpndns'), @('11199','void'), @('51820','wg'))
if (Get-Process filerelay -ErrorAction SilentlyContinue) {
  foreach ($r in $relays) { 'stop' | Set-Content "$RD\$($r[1]).c2s.stop" }
  $t = [Diagnostics.Stopwatch]::StartNew()
  while ((Get-Process filerelay -ErrorAction SilentlyContinue) -and $t.Elapsed.TotalSeconds -lt 15) { Start-Sleep -Milliseconds 300 }
}
$src = Get-Item C:\sbx\in\bin\filerelay.exe; $dir = "C:\gl\r$($src.LastWriteTimeUtc.Ticks)"
if (-not (Test-Path "$dir\filerelay.exe")) { New-Item -ItemType Directory -Force $dir | Out-Null; Copy-Item $src.FullName "$dir\filerelay.exe" }
foreach ($r in $relays) {
  Start-Process "$dir\filerelay.exe" -ArgumentList 'listen',"127.0.0.1:$($r[0])","$RD\$($r[1]).c2s","$RD\$($r[1]).s2c" -WindowStyle Hidden
}
Start-Sleep 1
Get-CimInstance Win32_Process -Filter "Name='filerelay.exe'" | ForEach-Object { $_.CommandLine }
