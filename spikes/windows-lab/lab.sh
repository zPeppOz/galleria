#!/bin/bash
# Comandi di servizio del laboratorio Windows, da WSL.
#   lab.sh start             avvia la Sandbox con l'agent (una sola istanza alla volta)
#   lab.sh stop              chiude la Sandbox (wsb stop: l'ospite sparisce con tutto quello che contiene)
#   lab.sh job <file.ps1> [timeout-secondi]   esegue uno script nella Sandbox e ne stampa l'output
#   lab.sh relays            (ri)avvia i relay UDP sui due lati: OpenVPN (2 server), WireGuard, e uno senza nessuno dietro
# Niente di tutto questo tocca la rete dell'host Windows: fra Sandbox e WSL passano solo file nelle cartelle mappate.
set -uo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
SBX=/mnt/c/Users/hackp/galleria-sbx
WSB=/mnt/c/Users/hackp/AppData/Local/Microsoft/WindowsApps/wsb.exe
RELAY="$HERE/filerelay/target/release/filerelay"

job() {
  local n; n="j-$(basename "$1" .ps1)-$(date +%s)"
  cp "$1" "$SBX/in/jobs/$n.ps1"
  for _ in $(seq 1 "${2:-300}"); do [ -f "$SBX/out/jobs/$n.done" ] && break; sleep 1; done
  tr -d '\r' < "$SBX/out/jobs/$n.log" 2>/dev/null
  [ -s "$SBX/out/jobs/$n.err" ] && { echo "--- stderr"; tr -d '\r' < "$SBX/out/jobs/$n.err"; }
  [ -f "$SBX/out/jobs/$n.done" ] || echo "[JOB TIMEOUT]"
}

case "${1:-}" in
  start)
    mkdir -p "$SBX/in/jobs" "$SBX/in/bin" "$SBX/out/jobs" "$SBX/out/relay"
    cp "$HERE/sandbox/agent.ps1" "$SBX/in/"; cp "$HERE/sandbox/agent.wsb" "$SBX/"
    rm -f "$SBX/out/agent-up.txt"
    (cd /mnt/c/Users/hackp && /mnt/c/Windows/System32/cmd.exe /c start "" 'C:\Users\hackp\galleria-sbx\agent.wsb')
    for _ in $(seq 1 120); do [ -f "$SBX/out/agent-up.txt" ] && { cat "$SBX/out/agent-up.txt"; exit 0; }; sleep 2; done
    echo "agent not up"; exit 1 ;;
  stop)
    (cd /mnt/c/Users/hackp && for id in $("$WSB" list | tr -d '\r'); do "$WSB" stop --id "$id"; done) ;;
  job) shift; job "$@" ;;
  relays)
    cp "$HERE/filerelay/target/x86_64-pc-windows-gnu/release/filerelay.exe" "$SBX/in/bin/"
    # lato WSL: i relay vecchi escono da soli quando trovano il file .stop
    for n in ovpn ovpndns wg; do echo stop > "$SBX/out/relay/$n.s2c.stop"; done
    for _ in $(seq 1 30); do pgrep -f "[f]ilerelay connect" >/dev/null || break; sleep 0.3; done
    for r in "172.30.0.10:1194 ovpn" "172.30.0.11:1194 ovpndns" "172.30.0.12:51820 wg"; do set -- $r
      ( setsid nohup "$RELAY" connect "$1" "$SBX/out/relay/$2.s2c" "$SBX/out/relay/$2.c2s" >/dev/null 2>&1 < /dev/null & )
    done
    sleep 1; pgrep -af "[f]ilerelay connect" | sed 's|/mnt/c/Users/hackp/galleria-sbx/out/relay/||g'
    job "$HERE/sandbox/relays.ps1" 60 ;;
  *) sed -n 2,7p "$0"; exit 64 ;;
esac
