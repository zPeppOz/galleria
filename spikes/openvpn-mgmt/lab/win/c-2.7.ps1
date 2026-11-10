. C:\sbx\in\win\lib.ps1
Hdr "2.7 DNS e rotte spinte dal server su Windows: cosa arriva al demone, cosa applica OpenVPN da solo"
function Snap {
  Run "Get-NetAdapter | Format-Table Name, InterfaceDescription, Status, ifIndex -AutoSize"
  Run "Get-DnsClientServerAddress -AddressFamily IPv4 | Where-Object ServerAddresses | Format-Table InterfaceAlias, ServerAddresses -AutoSize"
  Run "Get-DnsClient | Where-Object ConnectionSpecificSuffix | Format-Table InterfaceAlias, ConnectionSpecificSuffix -AutoSize"
  Run "Get-DnsClientNrptRule | Format-List Namespace, NameServers, Comment; 'regole NRPT: ' + @(Get-DnsClientNrptRule).Count"
  Run "Get-NetRoute -AddressFamily IPv4 | Where-Object { `$_.DestinationPrefix -in '0.0.0.0/0','0.0.0.0/1','128.0.0.0/1','192.0.2.0/24','127.0.0.1/32' } | Format-Table ifIndex, InterfaceAlias, DestinationPrefix, NextHop, RouteMetric -AutoSize"
}
"--- prima ---"; Snap
foreach ($n in '2.7-dhcpopt', '2.7-dnsopt', '2.7-dhcpopt-noup') {
  "=================== $n ==================="
  if (Start-Conn $n) { "connesso" } else { "(non connesso)" }
  Snap
  Run "Resolve-DnsName intranet.galleria.test -Type A -DnsOnly -ErrorAction Continue | Format-Table Name, Type, IPAddress -AutoSize"
  Run "ping -n 2 10.8.0.1; ping -n 2 10.9.0.1"
  Wait-Conn $n
  "--- cosa ha ricevuto il demone dal programma fisso (--up / --down) ---"
  Ev $n 'netcfg_env' | ForEach-Object {
    "script_type=$($_.script_type) dns_channels=$($_.dns_channels -join ',') parsed=$($_.parsed | ConvertTo-Json -Compress)"
    $_.env.PSObject.Properties | Where-Object { $_.Name -match '^(foreign_option|dns_|route_(network|gateway|netmask|vpn_gateway|net_gateway|redirect)|redirect|dev$|ifconfig_local|ifconfig_netmask)' } | ForEach-Object { "    $($_.Name)=$($_.Value)" }
  }
  "--- cosa ha fatto OpenVPN da solo (log) ---"
  Grep-Log $n 'openvpn\[.*(dns|DNS|NRPT|netsh|route|ROUTE|Route|IPv4 MTU|address|interactive service|redirect|block)' 260 | Select-Object -First 30
  ""
}
"--- dopo l'ultima disconnessione ---"; Snap
