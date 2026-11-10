. C:\sbx\in\win\lib.ps1
Hdr "2.5 credenziali senza file su Windows: OTP con static-challenge, passphrase della chiave privata"
foreach ($n in '2.5-otp', '2.5-keypass') {
  "--- $n ---"
  if (Start-Conn $n) { "connesso" } else { "(non connesso)" }
  Wait-Conn $n
  Grep-Log $n '"event":"(mgmt_password|credentials_sent|connected|error)"|auth-user-pass|static-challenge|askpass' 300
  ""
}
