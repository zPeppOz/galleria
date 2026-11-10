. C:\sbx\in\win\lib.ps1
Hdr "2.4 stato e byte dalla management interface, con DCO (predefinito su Windows) e senza"
foreach ($n in '2.4', '2.4-nodco') {
  "--- $n ---"
  if (Start-Conn $n) { "connesso" } else { "(non connesso)" }
  Run "Get-NetAdapter | Format-Table Name, InterfaceDescription, Status -AutoSize"
  Run "ping -n 6 10.8.0.1"
  Wait-Conn $n
  "--- sequenza degli stati ---"
  Grep-Log $n '"event":"state"' | ForEach-Object { ($_ | ConvertFrom-Json).raw }
  "--- byte in/out (un evento al secondo) ---"
  Grep-Log $n '"event":"bytecount"' | ForEach-Object { $j = $_ | ConvertFrom-Json; "t=$($j.t) in=$($j.in) out=$($j.out)" }
  "--- DCO nel log di OpenVPN ---"
  Grep-Log $n 'dco|DCO|wintun|tap-windows|windows-driver|Data Channel' | Select-Object -First 12
  ""
}
