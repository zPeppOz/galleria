. C:\sbx\in\win\lib.ps1
Hdr "2.6 chiavi inline senza file in chiaro su disco, Windows"
$key = (Get-Content "$REQ\needle-key.txt").Trim(); $ta = (Get-Content "$REQ\needle-ta.txt").Trim()
"--- a) configurazione via named pipe (--config \\.\pipe\...), password della management su stdin ---"
if (Start-Conn '2.6-pipe') { "connesso" } else { "(non connesso)" }
Procs
""
Find-OnDisk $key 'una riga della chiave privata del client'
Find-OnDisk $ta 'una riga di ta.key'
Run "[IO.Directory]::GetFiles('\\.\pipe\') | Where-Object { `$_ -match 'galleria' }"
Wait-Conn '2.6-pipe'
Grep-Log '2.6-pipe' '"event":"(openvpn_spawned|mgmt_connected|connected)"' 300
""
"--- a2) esperimento: --config stdin (configurazione e password della management sullo stesso stdin) ---"
[void](Start-Conn '2.6-stdin' 20)
Wait-Conn '2.6-stdin'
Grep-Log '2.6-stdin' '"event":"(openvpn_spawned|mgmt_connected|connected|error|retry|openvpn_exited)"|Options error|Enter Management|MANAGEMENT' 300 | Select-Object -First 14
""
"--- b) management-external-key: la chiave privata resta nel demone ---"
if (Start-Conn '2.6-extkey') { "connesso" } else { "(non connesso)" }
Wait-Conn '2.6-extkey'
Grep-Log '2.6-extkey' '"event":"(pk_sign_request|pk_sign_done|connected|error)"|profile_directive_removed.*key|management-external-key' 300
""
"--- c) ripiego: file temporaneo con ACL solo SYSTEM e Administrators, cancellato al primo HOLD ---"
if (Start-Conn '2.6-tempfile') { "connesso" } else { "(non connesso)" }
Run "Get-ChildItem C:\ProgramData\galleria-spike -Recurse -Force -ErrorAction SilentlyContinue | Format-Table FullName, Length -AutoSize"
Wait-Conn '2.6-tempfile'
Ev '2.6-tempfile' 'tempfile_written' | ForEach-Object { "tempfile_written t=$($_.t) $($_.path)"; $_.access }
Grep-Log '2.6-tempfile' '"event":"(tempfile_removed|hold|connected|error)"' 300
Run "Test-Path C:\ProgramData\galleria-spike\s*"
