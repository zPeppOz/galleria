#!/usr/bin/env bash
# Dentro la VM, da `lab`, nella sessione grafica vera: i pezzi del giro quotidiano (B6, gruppo D) che non sono clic.
# Il pilota dall'host (tasti, fotografie) è lab/vm/daily-round.sh; il server WireGuard è wg-peer-vm.sh.
#
#   daily-vm.sh fresh            sposta in ~/old la configurazione e lo stato di Galleria, ferma l'app (non il demone)
#   daily-vm.sh launch [env..]   l'app dal menu delle applicazioni (gtk-launch), stderr in ~/spike/daily-app.log
#   daily-vm.sh quit             ferma l'app (kill), come farebbe uno che la chiude senza passare dalla tray
#   daily-vm.sh state            demone, interfaccia, handshake, ping, processo GUI, chiavi nel portachiavi (solo attributi)
#   daily-vm.sh keyring          elenca gli elementi del portachiavi (attributi, MAI il segreto)
#   daily-vm.sh dump             albero AT-SPI della finestra di Galleria
set -uo pipefail
export XDG_RUNTIME_DIR=/run/user/$(id -u)
export DBUS_SESSION_BUS_ADDRESS=unix:path=$XDG_RUNTIME_DIR/bus WAYLAND_DISPLAY=wayland-0 XDG_SESSION_TYPE=wayland
S=$HOME/spike
case ${1:-} in
fresh)
  t=$(date +%s); mkdir -p ~/old
  systemctl --user stop daily-app.service spike-app.service 2>/dev/null; systemctl --user reset-failed 2>/dev/null
  pkill -x galleria-gui 2>/dev/null; sleep 1
  [[ -d ~/.config/galleria ]] && mv ~/.config/galleria ~/old/cfg-$t
  [[ -d ~/.local/state/galleria ]] && mv ~/.local/state/galleria ~/old/state-$t
  echo fresh
  ;;
launch)
  shift; extra=(); for e in "$@"; do extra+=(-E "$e"); done
  systemd-run --user --quiet --collect --unit=daily-app -E WAYLAND_DISPLAY -E XDG_SESSION_TYPE=wayland "${extra[@]}" \
    bash -c 'gtk-launch io.github.zpeppoz.galleria 2> ~/spike/daily-app.log; sleep 2; while pgrep -x galleria-gui >/dev/null; do sleep 1; done'
  for _ in $(seq 30); do pgrep -x galleria-gui >/dev/null && break; sleep 0.5; done
  pgrep -ax galleria-gui || echo 'no galleria-gui'
  ;;
quit)
  pkill -x galleria-gui; sleep 1; pgrep -ax galleria-gui || echo 'galleria-gui gone'
  ;;
orca-on)
  # tour with the screen reader: no fake daemon, no seeded app; Orca with the debug file, then the app from the menu
  for u in spike-app spike-daemon; do systemctl --user stop "$u.service" 2>/dev/null; done
  pkill -x galleria-gui; sleep 1
  bash "$S/orca-spike-vm.sh" orca | tail -2
  bash "$0" launch
  sleep 4; echo "speech count: $(bash "$S/orca-spike-vm.sh" count)"
  ;;
said)
  # what Orca said after the N-th sentence (default: all since the start of the tour)
  bash "$S/orca-spike-vm.sh" since "${2:-0}"
  ;;
keyring)
  python3 - <<'PY'
import gi
gi.require_version("Secret", "1")
from gi.repository import Secret
svc = Secret.Service.get_sync(Secret.ServiceFlags.OPEN_SESSION | Secret.ServiceFlags.LOAD_COLLECTIONS, None)
for coll in svc.get_collections() or []:
    print("collection", coll.get_label(), "locked" if coll.get_locked() else "unlocked")
    for item in coll.get_items():
        print("  item", item.get_label(), dict(item.get_attributes()))
PY
  ;;
state)
  sg galleria -c "$S/bin/galleria status" 2>&1
  echo "wireguard links: $(ip -br link show type wireguard | awk '{print $1, $2}' | tr '\n' ' ')"
  hs=$(sudo ip netns exec glabpeer wg show wg0 latest-handshakes 2>/dev/null | awk '{print $2}')
  echo "server latest handshake: ${hs:-none}  now: $(date +%s)"
  ping -c1 -W2 10.99.0.1 >/dev/null 2>&1 && echo 'ping 10.99.0.1 through the tunnel: ok' || echo 'ping 10.99.0.1 through the tunnel: FAIL'
  echo "gui: $(pgrep -ax galleria-gui | tr '\n' ' ')"
  bash "$0" keyring
  ;;
dump) python3 "$S/atspi-dump.py" ;;
*) sed -n '2,13p' "$0"; exit 2 ;;
esac
