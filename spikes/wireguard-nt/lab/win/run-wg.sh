#!/bin/bash
# Spike 1 su Windows: lancia lab/win/c-<criterio>.ps1 nella Sandbox e raccoglie le prove in evidence/windows/.
# Uso: run-wg.sh 1.1 | 1.5 | 1.8      (1.1 copre anche 1.2 e 1.3; per 1.6 e 1.7 vedi msi/run-msi.sh)
# Serve: Sandbox con l'agent e relay su (spikes/windows-lab/lab.sh start, lab.sh relays), server di prova su (lab/up.sh).
set -uo pipefail
cd "$(dirname "$0")/.."
SBX=/mnt/c/Users/hackp/galleria-sbx
EVD="$(cd .. && pwd)/evidence/windows"; mkdir -p "$EVD" "$SBX/in/wgwin"
COUNT="$(cd ../../openvpn-mgmt/lab/win && pwd)/count6.sh"
job() { # job <script.ps1> [timeout]
  local n="g-$(basename "$1" .ps1)-$(date +%s)"
  cp win/lib.ps1 "$SBX/in/wgwin/"; cp "$1" "$SBX/in/jobs/$n.ps1"
  for _ in $(seq 1 "${2:-300}"); do [ -f "$SBX/out/jobs/$n.done" ] && break; sleep 1; done
  tr -d '\r' < "$SBX/out/jobs/$n.log"; [ -s "$SBX/out/jobs/$n.err" ] && { echo "--- stderr"; tr -d '\r' < "$SBX/out/jobs/$n.err"; }
  [ -f "$SBX/out/jobs/$n.done" ] || echo "[JOB TIMEOUT]"
}
case "$1" in
  1.1)
    job win/c-1.1.ps1 300 > "$EVD/1.1-1.2-1.3.txt"
    { echo; echo "--- lato server (docker exec gl-wg wg show wg0) ---"; docker exec gl-wg wg show wg0 | grep -v private; } >> "$EVD/1.1-1.2-1.3.txt" 2>&1 ;;
  1.5)
    for mode in ${IPV6_MODES:-none route firewall}; do
      sed "s/__MODE__/$mode/g" win/c-1.5.ps1 > /tmp/c-1.5-$mode.ps1
      job /tmp/c-1.5-$mode.ps1 400 > "$EVD/1.5-$mode.txt"
      cp "$SBX/out/ev1/1.5-$mode.pcapng" "$EVD/" 2>/dev/null
      { echo "--- pacchetti IPv6 globali nella cattura di 60 s (stesso filtro dello spike Linux), per destinazione ---"
        "$COUNT" "$EVD/1.5-$mode.pcapng"; } >> "$EVD/1.5-$mode.txt" 2>&1
      echo "1.5 $mode done"
    done ;;
  1.8) job win/c-1.8.ps1 200 > "$EVD/1.8.txt" ;;
  *) echo "criterio sconosciuto: $1"; exit 64 ;;
esac
echo "$1 done"
