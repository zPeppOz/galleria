. C:\sbx\in\wgwin\msi-lib.ps1
Hdr "1.6 MSI di Tauri con frammento WiX (ServiceInstall/ServiceControl): installazione"
"--- prima ---"; State
Run "Get-AuthenticodeSignature '$(Msi 0.0.1)' | Format-List Status"
Msiexec '/i' (Msi 0.0.1) "$OUT\1.6-install.log"
"--- dopo l'installazione ---"; State
$dir = InstallDir
Run "sc.exe qc galleriad"
Run "Get-Service galleriad | Format-List Name, Status, StartType"
Run "Get-ChildItem '$dir' | Format-Table Length, Name -AutoSize"
Run "(Get-Acl '$dir\galleriad.exe').Access | Format-Table IdentityReference, FileSystemRights, AccessControlType -AutoSize"
Hello
"--- il servizio installato dall'MSI alza un tunnel (serve un adattatore da togliere) ---"
"PS> galleriad.exe cli connect wgtest.conf none"; & "$dir\galleriad.exe" cli connect C:\sbx\in\wg\wgtest.conf none 2>&1; "[exit $LASTEXITCODE]"; ""
Start-Sleep 3
Run "Get-NetAdapter | Format-Table Name, InterfaceDescription, Status -AutoSize"
Run "Get-PnpDevice -PresentOnly | Where-Object FriendlyName -match 'WireGuard' | Format-Table Status, Class, FriendlyName, InstanceId -AutoSize"

Hdr "1.6 disinstallazione: toglie servizio e adattatori"
$n = LogCount $SLOG
Msiexec '/x' (Msi 0.0.1) "$OUT\1.6-uninstall.log"
"--- dopo la disinstallazione ---"; State
Run "sc.exe query galleriad"
Run "Get-Process galleriad, galleria-spike-app -ErrorAction SilentlyContinue | Format-Table Name, Id -AutoSize; 'processi rimasti: ' + @(Get-Process galleriad, galleria-spike-app -ErrorAction SilentlyContinue).Count"
Run "Get-NetAdapter | Format-Table Name, InterfaceDescription, Status -AutoSize"
Run "'adattatori WireGuard presenti: ' + @(Get-PnpDevice -PresentOnly | Where-Object FriendlyName -match 'WireGuard').Count"
Run "'cartella di installazione presente: ' + (Test-Path '$dir'); Get-ChildItem '$dir' -ErrorAction SilentlyContinue | Format-Table Name"
"--- cosa resta nel sistema (non tolto dall'MSI) ---"
Run "pnputil.exe /enum-drivers | Select-String -Context 1,5 'wireguard'"
Run "Get-ChildItem C:\ProgramData\galleria-spike | Format-Table Length, Name -AutoSize"
"--- registro del servizio durante la disinstallazione ---"
LogLines $SLOG $n
