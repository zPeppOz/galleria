#!/bin/bash
# Esegue i criteri dello spike 2 su un client (26 = OpenVPN 2.6, 27 = OpenVPN 2.7)
# e salva le prove in evidence/ovpn<ver>/. Uso: run-criteria.sh 26|27 [criterio...]
set -uo pipefail
cd "$(dirname "$0")"
V="$1"; shift
ONLY="${*:-2.1 2.2 2.4 2.5 2.6 2.7 2.8 2.9}"
C=gl-client$V
EV="$(cd .. && pwd)/evidence/ovpn$V"; mkdir -p "$EV"
D=/usr/local/lib/galleria/galleriad-spike
TOTP=totp:JBSWY3DPEHPK3PXP

cx() { docker exec "$C" bash -c "$*"; }
run() { echo "\$ $*"; cx "$*" 2>&1; echo "[exit $?]"; echo; }
hdr() { echo; echo "=== $* ==="; date -u +%FT%TZ; docker exec $C openvpn --version | head -1; echo; }
want() { [[ " $ONLY " == *" $1 "* ]]; }
# connessione in background: log del demone in $EV/$1.log; ritorna quando arriva "connected"
start_conn() {
  local name=$1; shift
  python3 req.py "$@" | docker exec -i "$C" $D connect > "$EV/$name.log" 2>&1 &
  BG=$!
  for _ in $(seq 1 100); do
    grep -q '"event":"connected"' "$EV/$name.log" 2>/dev/null && return 0
    kill -0 $BG 2>/dev/null || return 1
    sleep 0.2
  done
  return 1
}
wait_conn() { wait "$BG"; echo "[daemon exit $?]"; }
ovpn_pid() { cx "pgrep -x openvpn | head -1"; }

if want 2.1; then {
  MAL=profiles/malicious.ovpn; [ "$V" = 79 ] && MAL=profiles/malicious-fedora.ovpn
  hdr "2.1 profilo malevolo: up/down/route-up/ipchange/plugin/management/status/writepid"
  echo "--- profilo (direttive non inline) ---"; awk '/^<[a-z-]+>$/{s=1} !s{print} /^<\/[a-z-]+>$/{s=0}' "$MAL"; echo
  run "rm -f /tmp/PWNED-*"
  echo "--- A) controllo: stesso profilo SENZA pulizia (unsafe_no_sanitize=true) ---"
  start_conn 2.1-control "$MAL" username=alice password=alice-secret unsafe_no_sanitize=true hold_secs=4 || echo "(non connesso)"
  run "ss -ltnup | grep -E '7505|openvpn' || true"
  wait_conn
  run "ls -l /tmp/PWNED-* 2>&1"
  run "rm -f /tmp/PWNED-*"
  echo "--- B) demone: profilo ripulito ---"
  start_conn 2.1-sanitized "$MAL" username=alice password=alice-secret hold_secs=4 || echo "(non connesso)"
  run "ss -ltnup | grep -E '7505|openvpn' || true"
  wait_conn
  grep -E 'profile_directive_removed|"connected"' "$EV/2.1-sanitized.log"
  echo
  run "ls -l /tmp/PWNED-* 2>&1"
} > "$EV/2.1.txt" 2>&1; echo "2.1 done"; fi

if want 2.2; then {
  hdr "2.2 socket di management: cartella 0700 di root + management-client-user root"
  start_conn 2.2 profiles/base.ovpn username=alice password=alice-secret hold_secs=12 || echo "(non connesso)"
  run "ls -ld /run/galleria /run/galleria/s*; ls -l /run/galleria/s*/"
  run "id alice"
  echo "--- utente normale, permessi normali ---"
  run "runuser -u alice -- socat -T2 - UNIX-CONNECT:\$(ls /run/galleria/s*/mgmt.sock 2>/dev/null || echo /run/galleria/x/mgmt.sock) </dev/null"
  run "runuser -u alice -- ls /run/galleria"
  echo "--- difesa in profondita': permessi allargati a mano (777), resta il controllo di OpenVPN sull'uid ---"
  run "chmod 755 /run/galleria /run/galleria/s*; chmod 777 /run/galleria/s*/mgmt.sock; ls -ld /run/galleria/s* /run/galleria/s*/mgmt.sock"
  run "echo 'state' | runuser -u alice -- socat -T2 - UNIX-CONNECT:\$(ls /run/galleria/s*/mgmt.sock)"
  echo "--- secondo client root mentre il demone e' collegato ---"
  run "echo 'state' | timeout 3 socat -T2 - UNIX-CONNECT:\$(ls /run/galleria/s*/mgmt.sock); echo rc=\$?"
  run "chmod 700 /run/galleria /run/galleria/s*"
  wait_conn
  echo "--- log di OpenVPN sui tentativi di connessione al management ---"
  grep -E 'MANAGEMENT: (Client|client|unix|.*uid|.*denied|.*reject)' "$EV/2.2.log"
  echo
  echo "--- D) solo --management-client-user root, senza la cartella 0700 e senza il demone collegato ---"
  run "M=\$(mktemp -d /tmp/mg.XXXX) && chmod 777 \$M
    (timeout 6 openvpn --config /profiles/base.ovpn --management \$M/m.sock unix --management-client-user root --management-hold --management-query-passwords --verb 3 > \$M/log 2>&1 &)
    sleep 1; ls -l \$M/m.sock
    echo '>> alice:'; echo help | runuser -u alice -- socat -T1 - UNIX-CONNECT:\$M/m.sock; echo rc=\$?
    sleep 1; echo '>> root:'; echo 'state' | socat -T1 - UNIX-CONNECT:\$M/m.sock | head -3
    sleep 5; echo '>> log openvpn:'; grep -E 'MANAGEMENT' \$M/log"
} > "$EV/2.2.txt" 2>&1; echo "2.2 done"; fi

if want 2.4; then {
  hdr "2.4 stato e byte: state on + bytecount 1"
  start_conn 2.4 profiles/base.ovpn username=alice password=alice-secret hold_secs=8 || echo "(non connesso)"
  run "ping -c 6 -i 0.5 10.8.0.1"
  wait_conn
  echo "--- eventi del demone (state, connected, bytecount) ---"
  grep -E '"event":"(state|connected|bytecount|openvpn_exited)"' "$EV/2.4.log"
  echo "--- DCO ---"
  grep -iE 'dco' "$EV/2.4.log"
  run "grep -i ovpn /proc/modules || echo 'nessun modulo ovpn/ovpn-dco caricato'; uname -r"
} > "$EV/2.4.txt" 2>&1; echo "2.4 done"; fi

if want 2.5; then {
  hdr "2.5 credenziali senza file: Auth, OTP (static-challenge), passphrase della chiave"
  for t in "auth profiles/base.ovpn username=alice password=alice-secret" \
           "otp profiles/otp.ovpn username=bob password=bob-secret otp=$TOTP" \
           "keypass profiles/enckey.ovpn username=alice password=alice-secret key_passphrase=key-passphrase"; do
    set -- $t; n=$1; shift
    echo "--- $n ---"
    start_conn 2.5-$n "$@" hold_secs=2 || echo "(non connesso)"
    wait_conn
    grep -E '"event":"(mgmt_password|credentials_sent|connected|error)"|MANAGEMENT: CMD .(username|password)' "$EV/2.5-$n.log"
    echo
  done
  echo "--- log del server (verify.sh) ---"
  docker logs gl-server 2>&1 | grep 'verify.sh' | tail -5
  echo "--- nessun file di credenziali: auth-user-pass e askpass senza argomento nella config generata ---"
  grep -h '"event":"generated_config"' "$EV"/2.5-*.log | python3 -c 'import sys,json; [print([d for d in json.loads(l)["profile_directives"] if "auth" in d or "askpass" in d or "static" in d]) for l in sys.stdin]'
} > "$EV/2.5.txt" 2>&1; echo "2.5 done"; fi

if want 2.6; then {
  hdr "2.6 chiavi inline senza file in chiaro"
  KM="$(sed -n 2p pki/client.key)"; TM="$(grep -v '^#' pki/ta.key | sed -n 3p)"
  echo "marcatori: 2a riga della chiave privata e 3a riga di ta.key (non stampati)"
  scan() { cx "grep -rlF -e '$KM' -e '$TM' / --exclude-dir=proc --exclude-dir=sys --exclude-dir=pki --exclude-dir=profiles --exclude-dir=evidence --exclude-dir=dev 2>/dev/null; echo \"[file con materiale della chiave: \$(grep -rlF -e '$KM' -e '$TM' / --exclude-dir=proc --exclude-dir=sys --exclude-dir=pki --exclude-dir=profiles --exclude-dir=evidence --exclude-dir=dev 2>/dev/null | wc -l)]\""; }
  echo "--- A) configurazione via pipe (stdin) ---"
  start_conn 2.6-pipe profiles/base.ovpn username=alice password=alice-secret hold_secs=10 || echo "(non connesso)"
  P=$(ovpn_pid)
  run "tr '\0' ' ' < /proc/$P/cmdline; echo; ls -l /proc/$P/fd"
  echo "\$ grep -rlF <marcatori> / (esclusi proc, sys, dev e i mount di prova pki/profiles/evidence)"; scan; echo
  echo "--- SIGUSR1 (riavvio soft) con config da pipe ---"
  run "kill -USR1 $P; sleep 4; pgrep -x openvpn"
  grep -E '"state":"(RECONNECTING|CONNECTED)"' "$EV/2.6-pipe.log" | tail -3
  echo "--- SIGHUP (riavvio con rilettura della config) con config da pipe ---"
  run "kill -HUP $P; sleep 3; pgrep -x openvpn || echo 'openvpn non gira piu'"
  wait_conn
  grep -iE 'SIGHUP|restart|Options error|error|EXITING' "$EV/2.6-pipe.log" | tail -8
  echo
  echo "--- B) --management-external-key: <key> tolta dalla config, firma fatta dal demone ---"
  start_conn 2.6-extkey profiles/base.ovpn username=alice password=alice-secret key_mode=external hold_secs=3 || echo "(non connesso)"
  wait_conn
  grep -E '"event":"(profile_directive_removed|pk_sign_request|pk_sign_done|connected|error)"|management-external-key' "$EV/2.6-extkey.log" | cut -c1-300
  echo
  echo "--- C) ripiego: file temporaneo 0600, cancellato al primo HOLD ---"
  start_conn 2.6-tempfile profiles/base.ovpn username=alice password=alice-secret config_via=tempfile hold_secs=3 || echo "(non connesso)"
  echo "\$ grep -rlF <marcatori> / (durante la connessione)"; scan; echo
  wait_conn
  grep -E '"event":"(openvpn_spawned|tempfile_removed|connected)"' "$EV/2.6-tempfile.log"
} > "$EV/2.6.txt" 2>&1; echo "2.6 done"; fi

if want 2.7; then {
  hdr "2.7 DNS e rotte spinti dal server -> modulo comune"
  for t in "dhcpopt profiles/base.ovpn" "dnsopt profiles/dns-option.ovpn"; do
    set -- $t; n=$1; prof=$2
    echo "##### server che spinge $( [ $n = dhcpopt ] && echo 'dhcp-option DNS/DOMAIN (gl-server)' || echo 'dns server/search-domains (gl-server-dns)') #####"
    echo "--- A) con il nostro programma fisso come --up ---"
    start_conn 2.7-$n $prof username=alice password=alice-secret hold_secs=6 || echo "(non connesso)"
    run "resolvectl status tun0"
    run "resolvectl query intranet.galleria.test"
    run "ip route; ip -6 route"
    wait_conn
    grep -E '"event":"(netcfg_env|netcfg_cmd)"' "$EV/2.7-$n.log" | cut -c1-1200
    run "resolvectl status tun0 2>&1 | head -3"
    echo "--- B) controllo: senza il nostro --up (unsafe run), cosa fa OpenVPN da solo ---"
    start_conn 2.7-$n-noup $prof username=alice password=alice-secret unsafe_no_sanitize=true hold_secs=5 || echo "(non connesso)"
    run "resolvectl status tun0"
    wait_conn
    grep -iE 'dns-updown|dns_updown|/libexec/openvpn|resolvectl' "$EV/2.7-$n-noup.log" | head
    echo
  done
  echo "--- query arrivate ai dnsmasq dei server ---"
  docker exec gl-server tail -n 4 /var/log/dnsmasq.log; docker exec gl-server-dns tail -n 4 /var/log/dnsmasq.log
} > "$EV/2.7.txt" 2>&1; echo "2.7 done"; fi

if want 2.8; then
  for mode in ${IPV6_MODES:-off native common reject}; do {
    hdr "2.8 IPv6 con profilo 'tutto nel tunnel' solo IPv4, modalita' ipv6=$mode"
    [ $mode = off ] && { echo "--- prima della VPN: il sito dual-stack risponde in IPv6 dalla LAN ---"; run "curl -s -o /dev/null -w 'remote=%{remote_ip} time=%{time_total}s\n' http://dual.galleria.test/"; }
    start_conn 2.8-$mode profiles/base.ovpn username=alice password=alice-secret ipv6=$mode hold_secs=75 || echo "(non connesso)"
    run "ip -6 route; ip route | head -5"
    cx "timeout 62 tcpdump -i eth0 -nn -U -w /evidence/ovpn$V/2.8-$mode.pcap 'ip6 and not net fe80::/10 and not net ff00::/8' 2>/dev/null" &
    TD=$!; sleep 1
    for i in 1 2 3 4 5 6; do
      run "curl -s -o /dev/null -m 10 -w 'remote=%{remote_ip} connect=%{time_connect}s total=%{time_total}s\n' http://dual.galleria.test/"
      sleep 8
    done
    echo "--- client senza Happy Eyeballs (python socket.create_connection, prova gli indirizzi in ordine) ---"
    docker exec -i "$C" bash -c "cat > /tmp/seq.py" <<'PY'
import socket, time
print("ordine getaddrinfo:", [a[4][0] for a in socket.getaddrinfo("dual.galleria.test", 80, type=socket.SOCK_STREAM)])
t = time.time()
s = socket.create_connection(("dual.galleria.test", 80), timeout=10)
print("peer=%s secs=%.3f" % (s.getpeername()[0], time.time() - t))
PY
    run "python3 /tmp/seq.py"
    run "ping -6 -c 2 -W 1 2001:db8:99::1"
    wait $TD
    echo "--- pacchetti IPv6 globali su eth0 in 60 s ---"
    run "tcpdump -nn -r /evidence/ovpn$V/2.8-$mode.pcap 2>/dev/null | head -12; echo \"totale: \$(tcpdump -nn -r /evidence/ovpn$V/2.8-$mode.pcap 2>/dev/null | wc -l)\""
    wait_conn
    grep -E '"event":"netcfg_cmd"|block-ipv6|ipv6|::/|2000::' "$EV/2.8-$mode.log" | grep -viE 'link local|VERIFY' | cut -c1-300 | head -12
    echo "--- richieste viste dal sito (sorgente) ---"
    docker logs gl-target 2>&1 | tail -6
  } > "$EV/2.8-$mode.txt" 2>&1; echo "2.8 $mode done"; done
fi

if want 2.9; then {
  hdr "2.9 errori: password sbagliata, OTP sbagliato, server irraggiungibile"
  echo "--- password sbagliata ---"
  python3 req.py profiles/base.ovpn username=alice password=wrong | docker exec -i $C $D connect > "$EV/2.9-badpass.log" 2>&1; echo "[daemon exit $?]"
  grep -E '"event":"(mgmt_password|error|attempt|retry|exit)"|AUTH_FAILED|EXITING' "$EV/2.9-badpass.log"
  echo; echo "--- OTP sbagliato ---"
  python3 req.py profiles/otp.ovpn username=bob password=bob-secret otp=000000 | docker exec -i $C $D connect > "$EV/2.9-badotp.log" 2>&1; echo "[daemon exit $?]"
  grep -E '"event":"(mgmt_password|error|attempt|retry|exit)"|AUTH_FAILED' "$EV/2.9-badotp.log"
  echo; echo "--- server irraggiungibile (172.30.0.99), max_attempts=4 ---"
  start=$(date +%s)
  python3 req.py profiles/unreachable.ovpn username=alice password=alice-secret max_attempts=4 | docker exec -i $C $D connect > "$EV/2.9-unreachable.log" 2>&1; echo "[daemon exit $?] durata $(( $(date +%s) - start ))s"
  grep -E '"event":"(attempt|retry|error|exit|openvpn_exited)"|poll timeout|restarting|Exiting' "$EV/2.9-unreachable.log"
} > "$EV/2.9.txt" 2>&1; echo "2.9 done"; fi
