# Libreria dei criteri dello spike 1 su Windows (dentro la Sandbox). Dot-source: . C:\sbx\in\wgwin\lib.ps1
$ErrorActionPreference = 'Continue'
$W = 'C:\gl\wg'; $G = "$W\galleriad.exe"; $EV = 'C:\sbx\out\ev1'; $SLOG = 'C:\ProgramData\galleria-spike\galleriad.log'
New-Item -ItemType Directory -Force $W, $EV, C:\gl\log | Out-Null

function Hdr($t) { ""; "=== $t ==="; (Get-Date).ToUniversalTime().ToString('s') + 'Z'; "" }
function Run($cmd) { "PS> $cmd"; try { Invoke-Expression $cmd 2>&1 | Out-String -Width 220 } catch { "ERROR: $_" }; "" }
# "Gcli" e non "Cli": in PowerShell cli e' un alias di Clear-Item e vince sulle funzioni
function Gcli { "PS> galleriad.exe cli $($args -join ' ')"; & $G cli @args 2>&1; "[exit $LASTEXITCODE]"; "" }

# Il servizio di prova, registrato a mano (per l'MSI vedi i criteri 1.6 e 1.7): LocalSystem, avvio manuale
function Ensure-Service {
  if (-not (Get-Service galleriad -ErrorAction SilentlyContinue)) {
    Copy-Item C:\sbx\in\wg\galleriad.exe, C:\sbx\in\wg\wireguard.dll, C:\sbx\in\wg\probe.exe $W -Force
    sc.exe create galleriad binPath= "$G service" start= demand obj= LocalSystem | Out-Null
  }
  if ((Get-Service galleriad).Status -ne 'Running') { sc.exe start galleriad | Out-Null; Start-Sleep 2 }
}
# Sito dual-stack di prova e IPv6 "globale" finto sulla scheda fisica (come nello spike 2)
function Ensure-Net {
  $e = (Get-NetAdapter -Name Ethernet).ifIndex
  if (-not (Get-NetIPAddress -InterfaceIndex $e -IPAddress 2001:db8:5::10 -ErrorAction SilentlyContinue)) {
    Add-Content C:\Windows\System32\drivers\etc\hosts "`r`n198.51.100.1 dual.galleria.test`r`n2001:db8:99::1 dual.galleria.test"
    New-NetIPAddress -InterfaceIndex $e -IPAddress 2001:db8:5::10 -PrefixLength 64 -DefaultGateway 2001:db8:5::1 | Out-Null
    New-NetNeighbor -InterfaceIndex $e -IPAddress 2001:db8:5::1 -LinkLayerAddress '02-00-00-00-00-01' -State Permanent | Out-Null
    Start-Sleep 3
  }
}
# Schede e rotte, come righe confrontabili (criterio 1.3)
function Net-Snapshot {
  Get-NetAdapter | Sort-Object Name | ForEach-Object { "adapter: $($_.Name) | $($_.InterfaceDescription)" }
  Get-NetRoute | Sort-Object AddressFamily, DestinationPrefix, ifIndex | ForEach-Object { "route: $($_.DestinationPrefix) via $($_.NextHop) if=$($_.InterfaceAlias) metric=$($_.RouteMetric)" }
  Get-NetIPAddress | Sort-Object IPAddress | ForEach-Object { "address: $($_.IPAddress)/$($_.PrefixLength) if=$($_.InterfaceAlias)" }
  netsh advfirewall firewall show rule name=galleria-spike-block-ipv6 | Select-String 'galleria' | ForEach-Object { "firewall: $($_.Line.Trim())" }
}
function As-User($exe, $argline) {
  $pw = ConvertTo-SecureString 'Gl-spike-2026!x' -AsPlainText -Force
  if (-not (Get-LocalUser gltest -ErrorAction SilentlyContinue)) { New-LocalUser gltest -Password $pw -PasswordNeverExpires | Out-Null; Add-LocalGroupMember -Group 'Users' -Member gltest -ErrorAction SilentlyContinue }
  $cred = New-Object Management.Automation.PSCredential('gltest', $pw)
  $out = "C:\Users\Public\gltest-$([DateTime]::UtcNow.Ticks).txt"
  try {
    $p = Start-Process $exe -ArgumentList $argline -Credential $cred -WorkingDirectory C:\Users\Public -RedirectStandardOutput $out -RedirectStandardError "$out.err" -WindowStyle Hidden -PassThru -ErrorAction Stop
    $p.WaitForExit(20000) | Out-Null
    Get-Content $out, "$out.err" -ErrorAction SilentlyContinue
  } catch { "As-User: non riuscito a lanciare il processo come gltest: $_" }
}
