# Prepara la Sandbox per lo spike 2: OpenVPN ufficiale, binari, relay verso il laboratorio Docker, IPv6 finto.
$ErrorActionPreference = 'Continue'
New-Item -ItemType Directory -Force C:\gl, C:\gl\log, C:\sbx\out\ev, C:\sbx\out\relay | Out-Null
Copy-Item C:\sbx\in\bin\galleriad-spike.exe, C:\sbx\in\bin\filerelay.exe C:\gl\ -Force
if (-not (Test-Path 'C:\Program Files\OpenVPN\bin\openvpn.exe')) {
  $p = Start-Process msiexec -ArgumentList '/i','C:\sbx\in\OpenVPN-2.7.9-I001-amd64.msi','/qn','/l*v','C:\sbx\out\ev\ovpn-msi.log' -Wait -PassThru; "openvpn msi exit $($p.ExitCode)"
}
& 'C:\Program Files\OpenVPN\bin\openvpn.exe' --version | Select-Object -First 2
Get-Service OpenVPN* | Format-Table Name, Status, StartType -AutoSize | Out-String
# i relay verso il laboratorio Docker li avvia spikes/windows-lab/lab.sh relays (11194 -> gl-server, 11195 -> gl-server-dns,
# 11199 -> nessuno: "server irraggiungibile")
# sito dual-stack di prova: IPv4 raggiungibile solo dal tunnel, IPv6 "globale" finto sulla scheda fisica
Add-Content C:\Windows\System32\drivers\etc\hosts "`r`n198.51.100.1 dual.galleria.test`r`n2001:db8:99::1 dual.galleria.test"
$e = (Get-NetAdapter -Name Ethernet).ifIndex
New-NetIPAddress -InterfaceIndex $e -IPAddress 2001:db8:5::10 -PrefixLength 64 -DefaultGateway 2001:db8:5::1 | Out-Null
# vicino statico per il gateway finto: cosi' i pacchetti IPv6 globali escono davvero dalla scheda e pktmon li vede
New-NetNeighbor -InterfaceIndex $e -IPAddress 2001:db8:5::1 -LinkLayerAddress '02-00-00-00-00-01' -State Permanent | Out-Null
Start-Sleep 3
Get-NetIPAddress -InterfaceIndex $e | Format-Table IPAddress, PrefixLength, AddressState -AutoSize | Out-String
Get-NetRoute -DestinationPrefix '::/0','0.0.0.0/0' -ErrorAction SilentlyContinue | Format-Table ifIndex, DestinationPrefix, NextHop, RouteMetric -AutoSize | Out-String
Get-NetNeighbor -InterfaceIndex $e -IPAddress 2001:db8:5::1 | Format-Table IPAddress, LinkLayerAddress, State -AutoSize | Out-String
Get-Process filerelay | Format-Table Id, ProcessName -AutoSize | Out-String
