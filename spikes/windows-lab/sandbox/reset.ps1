# Dentro la Sandbox: ferma i processi di prova lanciati dagli spike e riavvia l'agent (dopo una modifica di agent.ps1)
Get-Process filerelay,openvpn,galleriad-spike -ErrorAction SilentlyContinue | Stop-Process
Get-CimInstance Win32_Process -Filter "Name='powershell.exe'" | Where-Object { $_.CommandLine -match 'agent\.ps1' -and $_.ProcessId -ne $PID } | ForEach-Object { Stop-Process -Id $_.ProcessId }
Start-Process powershell.exe -ArgumentList '-NoProfile','-ExecutionPolicy','Bypass','-WindowStyle','Hidden','-File','C:\sbx\in\agent.ps1' -WindowStyle Hidden
