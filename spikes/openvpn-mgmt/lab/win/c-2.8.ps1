. C:\sbx\in\win\lib.ps1
$mode = '__MODE__'
Hdr "2.8 IPv6 con profilo 'tutto nel tunnel' solo IPv4 su Windows, modalita' $mode"
"Scheda fisica: IPv6 globale finto 2001:db8:5::10/64, gateway 2001:db8:5::1 con vicino statico: i pacchetti IPv6 globali"
"escono davvero dalla scheda (nessuno risponde). Il sito dual-stack: 198.51.100.1 (solo dal tunnel) e 2001:db8:99::1."
""
$curl = "curl.exe -s -o NUL -m 10 -w 'remote=%{remote_ip} connect=%{time_connect}s total=%{time_total}s exit=%{exitcode}\n' http://dual.galleria.test/"
if (Start-Conn "2.8-$mode") { "connesso" } else { "(non connesso)" }
Run "Get-NetRoute -AddressFamily IPv6 | Where-Object { `$_.DestinationPrefix -notlike 'fe80*' -and `$_.DestinationPrefix -notlike 'ff00*' -and `$_.DestinationPrefix -ne '::1/128' } | Format-Table ifIndex, InterfaceAlias, DestinationPrefix, NextHop, RouteMetric -AutoSize"
Run "Get-NetIPAddress -AddressFamily IPv6 | Where-Object { `$_.IPAddress -notlike 'fe80*' -and `$_.IPAddress -ne '::1' } | Format-Table InterfaceAlias, IPAddress, PrefixLength -AutoSize"
# cattura di 60 s: tutti i componenti di rete (pktmon), poi si contano i pacchetti con sorgente = indirizzo della scheda fisica
pktmon stop 2>&1 | Out-Null; pktmon filter remove | Out-Null
"--- schede viste da pktmon ---"
pktmon list | Select-String -Pattern '^\s+\d+\s' | Select-Object -First 12 | ForEach-Object { $_.Line }
pktmon start --capture --comp nics --pkt-size 0 --file-name C:\gl\log\cap-$mode.etl | Select-Object -First 2
$t = [Diagnostics.Stopwatch]::StartNew()
$seen = @{}
1..6 | ForEach-Object {
  Run $curl
  # chi apre connessioni IPv6 verso indirizzi globali in questo momento (processo proprietario)
  Get-NetTCPConnection -ErrorAction SilentlyContinue | Where-Object { $_.RemoteAddress -match ':' -and $_.RemoteAddress -notmatch '^(fe80|::1|::$)' } | ForEach-Object {
    $k = "$($_.RemoteAddress):$($_.RemotePort) stato=$($_.State) locale=$($_.LocalAddress) processo=$((Get-Process -Id $_.OwningProcess -ErrorAction SilentlyContinue).ProcessName) ($($_.OwningProcess))"
    if (-not $seen[$k]) { $seen[$k] = 1 }
  }
  Start-Sleep 7
}
"--- connessioni TCP IPv6 verso indirizzi globali viste durante la cattura (campionate) ---"
$seen.Keys | Sort-Object
""
"--- client senza Happy Eyeballs sul nome (prova gli indirizzi nell'ordine dato dal sistema, timeout 10 s per indirizzo) ---"
& $D seqprobe dual.galleria.test 80 10
"--- stesso client direttamente sull'indirizzo IPv6: quanto ci mette a fallire ---"
& $D seqprobe 2001:db8:99::1 80 10
""
Run "ping -6 -n 2 -w 1000 2001:db8:99::1"
while ($t.Elapsed.TotalSeconds -lt 60) { Start-Sleep -Milliseconds 500 }
pktmon stop | Select-Object -First 1
"cattura durata $([int]$t.Elapsed.TotalSeconds) s"
pktmon etl2pcap C:\gl\log\cap-$mode.etl --out "$EV\2.8-$mode.pcapng" | Select-Object -Last 2
Wait-Conn "2.8-$mode"
"--- log di OpenVPN su IPv6 e DCO ---"
Grep-Log "2.8-$mode" 'openvpn\[.*(block-ipv6|ipv6|IPv6|::/|2000::|fd15|DCO|dco|TAP-Windows|tap-windows)' 260 | Select-Object -First 18
