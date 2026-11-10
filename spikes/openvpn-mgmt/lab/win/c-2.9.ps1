. C:\sbx\in\win\lib.ps1
Hdr "2.9 errori su Windows: password sbagliata, OTP sbagliato, server irraggiungibile"
"--- password sbagliata ---"
[void](Start-Conn '2.9-badpass' 30); Wait-Conn '2.9-badpass'
Grep-Log '2.9-badpass' '"event":"(mgmt_password|error|attempt|retry|exit)"|AUTH_FAILED|EXITING' 300
""
"--- OTP sbagliato ---"
[void](Start-Conn '2.9-badotp' 30); Wait-Conn '2.9-badotp'
Grep-Log '2.9-badotp' '"event":"(mgmt_password|error|attempt|retry|exit)"|AUTH_FAILED' 300
""
"--- server irraggiungibile (relay senza nessuno dietro: silenzio), max_attempts=4 ---"
$t = [Diagnostics.Stopwatch]::StartNew()
[void](Start-Conn '2.9-unreachable' 90); Wait-Conn '2.9-unreachable'
"durata $([int]$t.Elapsed.TotalSeconds) s"
Grep-Log '2.9-unreachable' '"event":"(attempt|retry|error|exit|openvpn_exited)"|poll timeout|restarting|Exiting' 300
