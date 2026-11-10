. C:\sbx\in\wgwin\lib.ps1
Ensure-Service; Ensure-Net
Hdr "1.1 il servizio galleriad (LocalSystem, Rust) carica wireguard.dll, crea l'adattatore e alza il tunnel da un .conf"
Run "sc.exe qc galleriad"
Run "Get-CimInstance Win32_Service -Filter `"Name='galleriad'`" | Format-List Name, State, StartMode, StartName, ProcessId, PathName"
Run "Get-AuthenticodeSignature $W\wireguard.dll | Format-List Status, @{n='Signer';e={`$_.SignerCertificate.Subject}}; (Get-FileHash $W\wireguard.dll -Algorithm SHA256).Hash"
"--- profilo di prova (chiavi omesse) ---"
Get-Content C:\sbx\in\wg\wgtest.conf | Where-Object { $_ -notmatch 'PrivateKey' }
""
$before = Net-Snapshot
Gcli hello
Gcli connect C:\sbx\in\wg\wgtest.conf none
Start-Sleep 3
Run "Get-NetAdapter | Format-Table Name, InterfaceDescription, Status, ifIndex -AutoSize"
Run "Get-NetIPAddress -InterfaceAlias galleria-wg -AddressFamily IPv4 | Format-Table IPAddress, PrefixLength -AutoSize"
Run "Get-NetRoute -InterfaceAlias galleria-wg -AddressFamily IPv4 | Format-Table DestinationPrefix, NextHop, RouteMetric -AutoSize"
Run "(Get-Process galleriad).Modules | Where-Object ModuleName -match 'wireguard' | Format-Table ModuleName, FileName -AutoSize"
Run "ping -n 4 10.99.0.1"
Run "curl.exe -s -m 10 -w '  [remote=%{remote_ip} total=%{time_total}s]\n' http://198.51.100.1/"

Hdr "1.2 ultimo handshake e byte in/out letti ogni secondo, durante un ping continuo"
$ping = Start-Process ping.exe -ArgumentList '-n','14','-l','600','10.99.0.1' -WindowStyle Hidden -PassThru -RedirectStandardOutput C:\gl\log\ping-1.2.txt
1..6 | ForEach-Object { (& $G cli status 2>&1); Start-Sleep 2 }
$ping.WaitForExit(20000) | Out-Null
""
Get-Content C:\gl\log\ping-1.2.txt | Select-Object -Last 4
"--- registro del servizio: una riga 'stats' al secondo ---"
Get-Content $SLOG | Select-String 'stats:' | Select-Object -Last 16 | ForEach-Object { $_.Line }

Hdr "1.3 disconnessione pulita: adattatore rimosso, nessuna rotta residua"
Gcli disconnect
Start-Sleep 3
$after = Net-Snapshot
$diff = Compare-Object $before $after
"schede, rotte e indirizzi prima: $($before.Count) righe; dopo: $($after.Count) righe; differenze: $(@($diff).Count)"
$diff | ForEach-Object { "  $($_.SideIndicator) $($_.InputObject)" }
Run "Get-NetAdapter | Format-Table Name, InterfaceDescription, Status -AutoSize"
Run "route print -4 | Select-String '10.99|galleria'; 'righe con 10.99 o galleria in route print: ' + @(route print | Select-String '10\.99|galleria').Count"
"--- registro del servizio (senza le righe stats) ---"
Get-Content $SLOG | Where-Object { $_ -notmatch 'stats:' } | Select-Object -Last 20
