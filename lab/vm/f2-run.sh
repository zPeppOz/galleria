#!/usr/bin/env bash
# Dentro la VM, da `lab`, nella sessione grafica vera: la prova di F2 (icona della tray che resta «…») senza tunnel vero.
# Un demone finto (examples/fake-daemon, scenario `day`) nel socket di ~/spike/run, l'app con il binario che si vuole provare,
# `icon-watch.py` che annota i file dell'icona, e per ogni passo un clic AT-SPI sulla finestra (connetti, disconnetti, due volte).
# Il giro dall'host (fotografie dell'icona nella barra) è in docs/08-b6-desktop-pass.md, sezione F2.
#
#   f2-run.sh <binario> <cartella-di-uscita> [attesa-in-secondi]
#
# Scrive in <cartella>: icon-watch.log, journal.txt (gnome-shell e l'estensione, dall'avvio), steps.txt (i clic e lo stato).
# Il giro finisce con: `unable to update icon` contati nel journal.
set -uo pipefail
bin=${1:?binario}; out=${2:?cartella}; wait_s=${3:-5}
export XDG_RUNTIME_DIR=/run/user/$(id -u)
export DBUS_SESSION_BUS_ADDRESS=unix:path=$XDG_RUNTIME_DIR/bus WAYLAND_DISPLAY=wayland-0 XDG_SESSION_TYPE=wayland
S=$HOME/spike
mkdir -p "$out"
systemctl --user stop f2-app.service spike-app.service daily-app.service 2>/dev/null; systemctl --user reset-failed 2>/dev/null
pkill -x galleria-gui; pkill -f icon-watch.py; sleep 1
since=$(date '+%Y-%m-%d %H:%M:%S')
python3 "$S/icon-watch.py" 600 > "$out/icon-watch.log" 2>&1 &
watch=$!
systemd-run --user --quiet --collect --unit=f2-app -E WAYLAND_DISPLAY -E XDG_SESSION_TYPE=wayland -E GALLERIA_SOCKET="$S/run/galleriad.sock" \
  bash -c "exec $bin 2> $out/app.log"
for _ in $(seq 30); do pgrep -x galleria-gui > /dev/null && break; sleep 0.5; done
sleep "$wait_s"
{
  echo "binary: $bin $(sha256sum "$bin" | cut -c1-16)"
  for step in Connect Disconnect Connect Disconnect; do
    echo "== $(date +%T.%N | cut -c1-12) click $step"
    python3 "$S/act.py" click "$step" 'push button' 2>&1 | tail -2
    sleep "$wait_s"
    echo "   icon files: $(ls /run/user/$(id -u)/galleria 2>/dev/null | grep tray | tr '\n' ' ')"
    bash "$S/tray-icon-probe.sh" 2>/dev/null | grep -E '^IconName' | cut -c1-120
  done
} > "$out/steps.txt" 2>&1
kill "$watch" 2>/dev/null
journalctl --user --since "$since" --no-pager 2>/dev/null | grep -iE 'tray-icon|appindicator' > "$out/journal.txt"
journalctl --since "$since" --no-pager 2>/dev/null | grep -iE 'tray-icon|appindicator' >> "$out/journal.txt"
echo "unable to update icon: $(grep -c 'unable to update icon' "$out/journal.txt")" | tee -a "$out/steps.txt"
