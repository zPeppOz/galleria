#!/bin/bash
# Genera i profili .ovpn di prova (chiavi inline) da lab/pki in lab/profiles.
set -euo pipefail
cd "$(dirname "$0")"
P=pki; O=profiles; mkdir -p "$O"
inline() { echo "<$1>"; cat "$2"; echo "</$1>"; }
base() { # $1 = file chiave privata, $2 = remote
cat <<EOF
client
dev tun
proto udp
remote ${2:-172.30.0.10} 1194
nobind
remote-cert-tls server
auth-user-pass
key-direction 1
verb 3
EOF
inline ca $P/ca.crt; inline cert $P/client.crt; inline key "$1"; inline tls-auth $P/ta.key
}
base $P/client.key > $O/base.ovpn
{ base $P/client.key; echo 'static-challenge "Enter OTP" 1'; } > $O/otp.ovpn
base $P/client-enc.key > $O/enckey.ovpn
base $P/client.key 172.30.0.99 > $O/unreachable.ovpn
base $P/client.key 172.30.0.11 > $O/dns-option.ovpn
# profilo malevolo (criterio 2.1): ogni direttiva prova a lasciare una traccia in /tmp
{ base $P/client.key; cat <<'EOF'
script-security 3
up "/usr/bin/touch /tmp/PWNED-up"
down "/usr/bin/touch /tmp/PWNED-down"
route-up "/usr/bin/touch /tmp/PWNED-route-up"
ipchange "/usr/bin/touch /tmp/PWNED-ipchange"
plugin /usr/lib/x86_64-linux-gnu/openvpn/plugins/openvpn-plugin-down-root.so "/usr/bin/touch /tmp/PWNED-plugin"
management 0.0.0.0 7505
status /tmp/PWNED-status 1
writepid /tmp/PWNED-writepid
setenv EVIL 1
EOF
} > $O/malicious.ovpn
# Fedora tiene i plugin in /usr/lib64
sed 's|/usr/lib/x86_64-linux-gnu/openvpn/plugins|/usr/lib64/openvpn/plugins|' $O/malicious.ovpn > $O/malicious-fedora.ovpn
# profilo malevolo per Windows: stesse direttive, tracce in C:\gl. Nei comandi la barra rovesciata e' un escape e
# OpenVPN li legge due volte: quattro barre nel profilo = una barra per cmd.exe (che nelle redirezioni non accetta "/")
{ base $P/client.key; cat <<'EOF'
script-security 3
up "C:/Windows/System32/cmd.exe /c echo x > C:\\\\gl\\\\PWNED-up.txt"
down "C:/Windows/System32/cmd.exe /c echo x > C:\\\\gl\\\\PWNED-down.txt"
route-up "C:/Windows/System32/cmd.exe /c echo x > C:\\\\gl\\\\PWNED-route-up.txt"
ipchange "C:/Windows/System32/cmd.exe /c echo x > C:\\\\gl\\\\PWNED-ipchange.txt"
management 0.0.0.0 7505
status C:/gl/PWNED-status.txt 1
writepid C:/gl/PWNED-writepid.txt
setenv EVIL 1
EOF
} > $O/malicious-win.ovpn
ls -l $O
