. C:\sbx\in\win\lib.ps1
Hdr "2.1 profilo malevolo su Windows: up/down/route-up/ipchange/management/status/writepid"
"--- direttive pericolose del profilo ---"
(Get-Content "$REQ\2.1-control.json" | ConvertFrom-Json).profile -split "`n" | Select-String 'PWNED|7505|script-security|setenv'
""
Run "Remove-Item C:\gl\PWNED-* -ErrorAction SilentlyContinue"
"--- A) controllo: stesso profilo SENZA pulizia (unsafe_no_sanitize=true) ---"
if (Start-Conn '2.1-control' 25) { "connesso" } else { "(non connesso)" }
Start-Sleep 6
Run "netstat -ano | Select-String ':7505 '"
Wait-Conn '2.1-control'
Run "Get-ChildItem C:\gl\PWNED-* | Format-Table Name, Length -AutoSize"
Run "Remove-Item C:\gl\PWNED-* -ErrorAction SilentlyContinue"
"--- B) demone: profilo ripulito ---"
if (Start-Conn '2.1-sanitized') { "connesso" } else { "(non connesso)" }
Run "netstat -ano | Select-String ':7505 '"
Wait-Conn '2.1-sanitized'
Grep-Log '2.1-sanitized' 'profile_directive_removed|"event":"connected"'
""
Run "Get-ChildItem C:\gl\PWNED-* -ErrorAction SilentlyContinue | Format-Table Name, Length -AutoSize; 'file PWNED: ' + @(Get-ChildItem C:\gl\PWNED-* -ErrorAction SilentlyContinue).Count"
