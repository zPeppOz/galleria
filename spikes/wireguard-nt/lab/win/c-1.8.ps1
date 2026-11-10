. C:\sbx\in\wgwin\lib.ps1
Ensure-Service
Hdr "1.8 named pipe con ACL: un utente locale non autorizzato riceve accesso negato (M1)"
"--- ACL dichiarata dal servizio alla creazione della pipe ---"
Get-Content $SLOG | Select-String 'ipc: listening' | Select-Object -Last 1 | ForEach-Object { $_.Line }
Run "[IO.Directory]::GetFiles('\\.\pipe\') | Where-Object { `$_ -match 'galleria-spike' }"
"--- utente amministratore (WDAGUtilityAccount, elevato): autorizzato ---"
Run "whoami /groups | Select-String 'Administrators|S-1-5-32-544|Mandatory'"
Gcli hello
"--- secondo account locale, senza privilegi (creato ora, gruppo Users) ---"
$r = As-User $G 'cli hello'
Run "Get-LocalUser gltest | Format-Table Name, Enabled -AutoSize; (Get-LocalGroup | Where-Object { (Get-LocalGroupMember `$_ -ErrorAction SilentlyContinue).Name -match 'gltest' }).Name"
"PS> (come gltest) galleriad.exe cli hello"
$r
""
"--- stesso account: una richiesta che cambierebbe la rete ---"
"PS> (come gltest) galleriad.exe cli disconnect"
As-User $G 'cli disconnect'
""
"--- controllo: come gltest il programma parte davvero (un comando che non usa la pipe) ---"
"PS> (come gltest) probe.exe seqprobe 127.0.0.1 1 1"
As-User "$W\probe.exe" 'seqprobe 127.0.0.1 1 1'
""
"--- registro del servizio: le richieste arrivate (quelle di gltest non ci sono) ---"
Get-Content $SLOG | Select-String 'ipc: request' | Select-Object -Last 4 | ForEach-Object { $_.Line }
