. C:\sbx\in\wgwin\lib.ps1
Ensure-Service; Ensure-Net
$mode = '__MODE__'
Hdr "1.5 IPv6 con profilo WireGuard 'tutto nel tunnel' solo IPv4 (AllowedIPs = 0.0.0.0/0), blocco IPv6 = $mode"
"none     = nessuna protezione"
"route    = D15 com'e' scritta: ::/1 e 8000::/1 verso l'adattatore WireGuard, dove nessun peer li accetta"
"firewall = regola di uscita che blocca 2000::/3 e fc00::/7 (un filtro WFP)"
"Scheda fisica: IPv6 globale finto 2001:db8:5::10/64 con gateway e vicino statico: i pacchetti escono davvero (nessuno risponde)."
""
$curl = "curl.exe -s -o NUL -m 10 -w 'remote=%{remote_ip} connect=%{time_connect}s total=%{time_total}s exit=%{exitcode}\n' http://dual.galleria.test/"
Gcli connect C:\sbx\in\wg\wgtest.conf $mode
Start-Sleep 3
Run "Get-NetRoute -AddressFamily IPv6 | Where-Object { `$_.DestinationPrefix -notlike 'fe80*' -and `$_.DestinationPrefix -notlike 'ff00*' -and `$_.DestinationPrefix -ne '::1/128' } | Format-Table ifIndex, InterfaceAlias, DestinationPrefix, NextHop, RouteMetric -AutoSize"
Run "netsh advfirewall firewall show rule name=galleria-spike-block-ipv6 | Select-String 'Nome regola|Rule Name|Direzione|Direction|IP remoto|RemoteIP|Azione|Action'"
Run "ping -n 2 10.99.0.1 | Select-String 'Risposta|Reply|scaduta|timed'"
pktmon stop 2>&1 | Out-Null; pktmon filter remove | Out-Null
pktmon start --capture --comp nics --pkt-size 0 --file-name C:\gl\log\cap1-$mode.etl | Select-Object -First 2
$t = [Diagnostics.Stopwatch]::StartNew()
$seen = @{}
1..6 | ForEach-Object {
  Run $curl
  Get-NetTCPConnection -ErrorAction SilentlyContinue | Where-Object { $_.RemoteAddress -match ':' -and $_.RemoteAddress -notmatch '^(fe80|::1|::$)' } | ForEach-Object {
    $k = "$($_.RemoteAddress):$($_.RemotePort) stato=$($_.State) locale=$($_.LocalAddress) processo=$((Get-Process -Id $_.OwningProcess -ErrorAction SilentlyContinue).ProcessName) ($($_.OwningProcess))"
    if (-not $seen[$k]) { $seen[$k] = 1 }
  }
  Start-Sleep 7
}
"--- connessioni TCP IPv6 verso indirizzi globali viste durante la cattura (campionate) ---"
$seen.Keys | Sort-Object
""
"--- client senza Happy Eyeballs sul nome (indirizzi nell'ordine dato dal sistema, timeout 10 s per indirizzo) ---"
& $W\probe.exe seqprobe dual.galleria.test 80 10
"--- stesso client direttamente sull'indirizzo IPv6: quanto ci mette a fallire ---"
& $W\probe.exe seqprobe 2001:db8:99::1 80 10
""
Run "ping -6 -n 2 -w 1000 2001:db8:99::1"
while ($t.Elapsed.TotalSeconds -lt 60) { Start-Sleep -Milliseconds 500 }
pktmon stop | Select-Object -First 1
"cattura durata $([int]$t.Elapsed.TotalSeconds) s"
pktmon etl2pcap C:\gl\log\cap1-$mode.etl --out "$EV\1.5-$mode.pcapng" | Select-Object -Last 2
Gcli status
Gcli disconnect
Start-Sleep 2
Run "netsh advfirewall firewall show rule name=galleria-spike-block-ipv6 | Select-Object -First 2"
"--- registro del servizio ---"
Get-Content $SLOG | Where-Object { $_ -notmatch 'stats:' } | Select-Object -Last 8
