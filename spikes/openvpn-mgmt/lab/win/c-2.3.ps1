. C:\sbx\in\win\lib.ps1
Hdr "2.3 management interface su Windows: TCP 127.0.0.1, porta casuale, password di sessione con pw-file stdin"
"--- A) sessione normale: il demone (SYSTEM, sessione 0) e' collegato alla management interface ---"
if (Start-Conn '2.3') { "connesso" } else { "(non connesso)" }
$pw = (Ev '2.3' 'TEST_ONLY_mgmt_password').password; $port = Mgmt-Port '2.3'
"porta scelta dal demone: $port; password di sessione: $($pw.Length) caratteri esadecimali (256 bit), rivelata solo per questa prova"
Grep-Log '2.3' 'Enter Management Password|MANAGEMENT: (TCP|Client)|"event":"openvpn_spawned"'
""
"--- righe di comando (Get-CimInstance Win32_Process) ---"
Procs
"la password compare in una riga di comando: " + [bool](Procs | Select-String -SimpleMatch $pw)
""
Run "netstat -ano | Select-String ':$port '"
"--- secondo client mentre il demone e' collegato: senza password ---"
& $D mgmtprobe $port
"--- secondo client mentre il demone e' collegato: con la password giusta ---"
& $D mgmtprobe $port $pw
"--- secondo client da un utente locale senza privilegi ---"
As-User "mgmtprobe $port"
""
Find-OnDisk $pw 'password della management interface'
Wait-Conn '2.3'
"--- il demone ha continuato a ricevere eventi dopo i tentativi (ultimi byte) ---"
Grep-Log '2.3' '"event":"bytecount"' | Select-Object -Last 2
""
"--- B) porta non ancora presa dal demone (collegamento ritardato di 25 s per la prova): chi arriva prima deve dare la password ---"
[void](Start-Conn '2.3-delay' -NoWait)
[void](Wait-Event '2.3-delay' 'TEST_ONLY_mgmt_connect_delayed' 20)
Start-Sleep 2
$port2 = Mgmt-Port '2.3-delay'
"--- senza password (admin) ---"
& $D mgmtprobe $port2
"--- password sbagliata (admin) ---"
& $D mgmtprobe $port2 not-the-password
"--- password sbagliata (utente senza privilegi) ---"
As-User "mgmtprobe $port2 guess"
if (Wait-Event '2.3-delay' 'connected' 60) { "poi il demone si collega con la password giusta: connesso" } else { "(il demone non si e' collegato dopo i tentativi)" }
Wait-Conn '2.3-delay'
Grep-Log '2.3-delay' 'MANAGEMENT: (Client|CMD|TCP)|bad password|"event":"mgmt_connected"' | Select-Object -First 14
""
"--- C) management-client: e' OpenVPN a collegarsi al demone (che ascolta su una porta casuale) ---"
if (Start-Conn '2.3-client') { "connesso" } else { "(non connesso)" }
Run "netstat -ano | Select-String ':$(Mgmt-Port '2.3-client') '"
Procs
Wait-Conn '2.3-client'
Grep-Log '2.3-client' 'MANAGEMENT|Enter Management|"event":"mgmt_connected"|"event":"connected"' | Select-Object -First 10
