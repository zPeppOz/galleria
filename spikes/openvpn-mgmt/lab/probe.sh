#!/bin/bash
# Prova singola: connette, mostra le rotte IPv6 durante la connessione, disconnette.
# uso: probe.sh 26|27 nome profilo.ovpn [chiave=valore ...]
set -uo pipefail
cd "$(dirname "$0")"
V=$1 N=$2 P=$3; shift 3
C=gl-client$V; EV="$(cd .. && pwd)/evidence/ovpn$V"; LOG="$EV/$N.log"; mkdir -p "$EV"
{
  echo "=== probe $N ($P $(printf "%s " "$@" | sed -E "s/(password|key_passphrase|otp)=[^ ]*/\1=***/g")) ==="; docker exec $C openvpn --version | head -1
  python3 req.py "$P" "$@" hold_secs=5 | docker exec -i $C /usr/local/lib/galleria/galleriad-spike connect > "$LOG" 2>&1 &
  BG=$!
  for _ in $(seq 1 50); do grep -q '"event":"connected"' "$LOG" && break; sleep 0.2; done
  echo "\$ ip -6 route  (durante la connessione)"; docker exec $C ip -6 route
  echo "\$ curl http://dual.galleria.test/"; docker exec $C curl -s -o /dev/null -m 10 -w 'remote=%{remote_ip} total=%{time_total}s\n' http://dual.galleria.test/
  wait $BG; echo "[daemon exit $?]"
  echo "--- log openvpn (righe su redirect/ipv6) ---"
  grep -iE 'route_v6|add_route_ipv6|redirect|IPv6 route|block-ipv6' "$LOG" | grep -v '"event"' | cut -c1-220
} > "$EV/$N.txt" 2>&1
cat "$EV/$N.txt"
