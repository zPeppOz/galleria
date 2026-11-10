#!/bin/bash
# Spike 2, parte Windows: prepara le richieste, lancia lab/win/c-<criterio>.ps1 nella Sandbox e raccoglie le prove.
# Uso: run-win.sh setup | 2.1 | 2.3 | 2.4 | 2.5 | 2.6 | 2.7 | 2.8 | 2.9
# Serve: Sandbox avviata con l'agent e relay su (spikes/windows-lab/lab.sh start, lab.sh relays),
# laboratorio Docker su (CLIENTS= lab/up.sh: senza i client Linux, vedi README).
set -uo pipefail
cd "$(dirname "$0")/.."
SBX=/mnt/c/Users/hackp/galleria-sbx
EVD="$(cd .. && pwd)/evidence/windows"; mkdir -p "$EVD" "$SBX/in/req" "$SBX/in/win" "$SBX/in/bin" "$SBX/out/relay"
TOTP=totp:JBSWY3DPEHPK3PXP
C="$1"

# profili per Windows: il "server" e' il relay locale (127.0.0.1) che porta i datagrammi al laboratorio Docker
# "redirect-gateway local": il server e' su loopback, OpenVPN non deve aggiungere la rotta /32 verso di lui via gateway
# (senza, una prova con remote 127.0.0.1 ha lasciato la Sandbox senza loopback TCP: vedi docs/05, ambiente)
wprofile() { sed -e 's/^remote 172\.30\.0\.10 1194/remote 127.0.0.1 11194/' -e 's/^remote 172\.30\.0\.11 1194/remote 127.0.0.1 11195/' \
                 -e 's/^remote 172\.30\.0\.99 1194/remote 127.0.0.1 11199/' -e '/^remote 127\.0\.0\.1/a redirect-gateway local' "profiles/$1.ovpn"; }
mkreq() { local name=$1 prof=$2; shift 2; wprofile "$prof" > /tmp/gl-wprof.ovpn; python3 req.py /tmp/gl-wprof.ovpn "$@" > "$SBX/in/req/$name.json"; }
job() { # job <script.ps1> [timeout]
  local n="w-$(basename "$1" .ps1)-$(date +%s)"
  cp win/lib.ps1 "$SBX/in/win/"; cp ../target/x86_64-pc-windows-gnu/release/galleriad-spike.exe "$SBX/in/bin/"; cp "$1" "$SBX/in/jobs/$n.ps1"
  for _ in $(seq 1 "${2:-300}"); do [ -f "$SBX/out/jobs/$n.done" ] && break; sleep 1; done
  tr -d '\r' < "$SBX/out/jobs/$n.log"; [ -s "$SBX/out/jobs/$n.err" ] && { echo "--- stderr"; tr -d '\r' < "$SBX/out/jobs/$n.err"; }
  [ -f "$SBX/out/jobs/$n.done" ] || echo "[JOB TIMEOUT]"
}
collect() { for f in "$SBX"/out/ev/$1*; do [ -f "$f" ] && tr -d '\r' < "$f" > "$EVD/$(basename "$f")"; done; }

case "$C" in
  setup)
    cp ../target/x86_64-pc-windows-gnu/release/galleriad-spike.exe "$SBX/in/bin/"
    job win/setup.ps1 400 | tee "$EVD/setup.txt" ;;
  2.1)
    mkreq 2.1-control malicious-win username=alice password=alice-secret unsafe_no_sanitize=true hold_secs=6
    mkreq 2.1-sanitized malicious-win username=alice password=alice-secret hold_secs=6
    job win/c-2.1.ps1 200 > "$EVD/2.1.txt"; collect 2.1 ;;
  2.3)
    mkreq 2.3 base username=alice password=alice-secret hold_secs=45 reveal_mgmt_password=true
    mkreq 2.3-delay base username=alice password=alice-secret hold_secs=5 reveal_mgmt_password=true mgmt_connect_delay_secs=25
    mkreq 2.3-client base username=alice password=alice-secret hold_secs=8 mgmt_client=true
    job win/c-2.3.ps1 400 > "$EVD/2.3.txt"; collect 2.3 ;;
  2.4)
    mkreq 2.4 base username=alice password=alice-secret hold_secs=25
    mkreq 2.4-nodco base username=alice password=alice-secret hold_secs=15 'test_extra=["disable-dco"]'
    job win/c-2.4.ps1 200 > "$EVD/2.4.txt"; collect 2.4 ;;
  2.5)
    mkreq 2.5-keypass enckey username=alice password=alice-secret key_passphrase=key-passphrase hold_secs=5
    mkreq 2.5-otp otp username=bob password=bob-secret otp=$TOTP hold_secs=5
    job win/c-2.5.ps1 200 > "$EVD/2.5.txt"; collect 2.5
    { echo; echo "--- log del server (verify.sh) ---"; docker logs gl-server 2>&1 | grep verify.sh | tail -3; } >> "$EVD/2.5.txt" ;;
  2.6)
    mkreq 2.6-pipe base username=alice password=alice-secret hold_secs=25
    mkreq 2.6-stdin base username=alice password=alice-secret hold_secs=5 config_via=stdin max_attempts=1
    mkreq 2.6-extkey base username=alice password=alice-secret hold_secs=6 key_mode=external
    mkreq 2.6-tempfile base username=alice password=alice-secret hold_secs=6 config_via=tempfile
    sed -n 3p pki/client.key > "$SBX/in/req/needle-key.txt"; sed -n 4p pki/ta.key > "$SBX/in/req/needle-ta.txt"
    job win/c-2.6.ps1 400 > "$EVD/2.6.txt"; collect 2.6 ;;
  2.7)
    mkreq 2.7-dhcpopt base username=alice password=alice-secret hold_secs=25
    mkreq 2.7-dnsopt dns-option username=alice password=alice-secret hold_secs=25
    mkreq 2.7-dhcpopt-noup base username=alice password=alice-secret hold_secs=20 unsafe_no_sanitize=true
    job win/c-2.7.ps1 400 > "$EVD/2.7.txt"; collect 2.7
    { echo; echo "--- query arrivate al dnsmasq dei server ---"; docker exec gl-server tail -5 /var/log/dnsmasq.log; docker exec gl-server-dns tail -5 /var/log/dnsmasq.log; } >> "$EVD/2.7.txt" 2>&1 ;;
  2.8)
    # native6-nodco: come native6 ma con --disable-dco (su Windows il DCO e' attivo di default)
    for mode in ${IPV6_MODES:-off native native6 native6-nodco}; do
      case $mode in
        native6-nodco) mkreq 2.8-$mode base username=alice password=alice-secret ipv6=native6 hold_secs=85 'test_extra=["disable-dco"]' ;;
        *) mkreq 2.8-$mode base username=alice password=alice-secret ipv6=$mode hold_secs=85 ;;
      esac
      sed "s/__MODE__/$mode/g" win/c-2.8.ps1 > /tmp/c-2.8-$mode.ps1
      job /tmp/c-2.8-$mode.ps1 400 > "$EVD/2.8-$mode.txt"; collect 2.8-$mode
      cp "$SBX/out/ev/2.8-$mode.pcapng" "$EVD/" 2>/dev/null
      { echo "--- pacchetti IPv6 globali nella cattura di 60 s (stesso filtro dello spike Linux), per destinazione ---"
        win/count6.sh "$EVD/2.8-$mode.pcapng"
        echo "--- richieste viste dal sito (sorgente) ---"; docker logs gl-target 2>&1 | tail -4; } >> "$EVD/2.8-$mode.txt" 2>&1
      echo "2.8 $mode done"
    done ;;
  2.9)
    mkreq 2.9-badpass base username=alice password=wrong
    mkreq 2.9-badotp otp username=bob password=bob-secret otp=000000
    mkreq 2.9-unreachable unreachable username=alice password=alice-secret max_attempts=4
    job win/c-2.9.ps1 300 > "$EVD/2.9.txt"; collect 2.9 ;;
  *) echo "criterio sconosciuto: $C"; exit 64 ;;
esac
echo "$C done"
