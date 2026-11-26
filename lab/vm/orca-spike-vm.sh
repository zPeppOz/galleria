#!/usr/bin/env bash
# Dentro la VM, da utente `lab`, per lo spike Orca (docs/08, «Spike Orca»): Orca e la GUI di Galleria dentro la
# sessione GNOME vera (Wayland, quella aperta dal login automatico di GDM), avviati con systemd-run --user.
# I file stanno in ~/spike: galleria-gui, fake-daemon, atspi-dump.py, orca-line-buffered.py (copiati dall'host).
#
#   orca-spike-vm.sh versions        distro, GNOME, Orca, WebKitGTK, AT-SPI
#   orca-spike-vm.sh orca            (ri)avvia Orca con il file di debug ~/spike/orca-debug.log (a righe)
#   orca-spike-vm.sh app             (ri)avvia il demone finto e la finestra di Galleria
#   orca-spike-vm.sh app-spike       come app, con la build della prova in-process (galleria-gui-spike)
#   orca-spike-vm.sh stop            ferma app, demone e Orca
#   orca-spike-vm.sh speech [N]      le ultime N frasi che Orca ha detto (righe SPEECH OUTPUT), default 20
#   orca-spike-vm.sh count           quante frasi ha detto Orca finora
#   orca-spike-vm.sh since N         le frasi dette dopo la N-esima
#   orca-spike-vm.sh daemon-down|daemon-up   ferma / riavvia il demone finto (la barra di stato cambia)
#   orca-spike-vm.sh dump            l'albero AT-SPI della finestra
#   orca-spike-vm.sh announce ARGS   emette un Announcement AT-SPI (atspi-announce.py)
set -uo pipefail
export XDG_RUNTIME_DIR=/run/user/$(id -u)
export DBUS_SESSION_BUS_ADDRESS=unix:path=$XDG_RUNTIME_DIR/bus WAYLAND_DISPLAY=wayland-0 XDG_SESSION_TYPE=wayland
S=$HOME/spike
LOG=$S/orca-debug.log
ENVS=(-E XDG_RUNTIME_DIR -E DBUS_SESSION_BUS_ADDRESS -E WAYLAND_DISPLAY -E XDG_SESSION_TYPE)
run() { systemd-run --user --quiet --collect "${ENVS[@]}" "$@"; }
stop_unit() { systemctl --user stop "$1" 2>/dev/null; systemctl --user reset-failed "$1" 2>/dev/null; return 0; }

case ${1:-} in
  versions)
    . /etc/os-release; echo "distro: $PRETTY_NAME  kernel: $(uname -r)  sessione: ${XDG_SESSION_TYPE}"
    if command -v rpm >/dev/null; then q() { rpm -q "$@" 2>/dev/null; }; else q() { dpkg-query -W -f='${Package} ${Version}\n' "$@" 2>/dev/null; }; fi
    echo "gnome-shell: $(gnome-shell --version 2>/dev/null)"
    for p in orca at-spi2-core mutter speech-dispatcher libwebkit2gtk-4.1-0 webkit2gtk4.1 libjavascriptcoregtk-4.1-0 javascriptcoregtk4.1 gtk3 libgtk-3-0t64; do q "$p"; done
    echo "orca --version: $(orca --version 2>&1 | tail -1)"
    ;;
  orca)
    stop_unit spike-orca.service; pkill -x orca 2>/dev/null; sleep 1
    gsettings set org.gnome.desktop.interface toolkit-accessibility true
    : > "$LOG"
    run --unit=spike-orca python3 "$S/orca-line-buffered.py" --debug-file="$LOG"
    for _ in $(seq 1 60); do grep -q 'SPEECH OUTPUT' "$LOG" 2>/dev/null && break; sleep 0.5; done
    grep 'SPEECH OUTPUT' "$LOG" | tail -3
    ;;
  app)
    stop_unit spike-app.service; stop_unit spike-daemon.service
    rm -rf "$S/run"; mkdir -p "$S/run"
    run --unit=spike-daemon "$S/fake-daemon" "$S/run"
    for _ in $(seq 1 50); do [[ -S $S/run/galleriad.sock ]] && break; sleep 0.2; done
    run --unit=spike-app -E GALLERIA_SOCKET="$S/run/galleriad.sock" -E XDG_CONFIG_HOME="$S/cfg" "$S/galleria-gui"
    ;;
  app-spike)
    # la build con la prova scritta nel patch lab/vm/orca-spike-inapp.patch: da dentro il processo, ogni 12 s, un segnale ATK
    stop_unit spike-app.service; stop_unit spike-daemon.service
    rm -rf "$S/run"; mkdir -p "$S/run"
    run --unit=spike-daemon "$S/fake-daemon" "$S/run"
    for _ in $(seq 1 50); do [[ -S $S/run/galleriad.sock ]] && break; sleep 0.2; done
    run --unit=spike-app -E GALLERIA_SOCKET="$S/run/galleriad.sock" -E GALLERIA_SPIKE_ANNOUNCE=1 "$S/galleria-gui-spike"
    ;;
  stop) stop_unit spike-app.service; stop_unit spike-daemon.service; stop_unit spike-orca.service; pkill -x orca 2>/dev/null; true ;;
  speech) grep 'SPEECH OUTPUT' "$LOG" | sed -E "s/^.*SPEECH OUTPUT: //; s/ ?\{'established'.*$//" | tail -n "${2:-20}" ;;
  count) grep -c 'SPEECH OUTPUT' "$LOG" ;;
  since) grep 'SPEECH OUTPUT' "$LOG" | tail -n +$(( ${2:-0} + 1 )) | sed -E "s/^.*SPEECH OUTPUT: //; s/ ?[{]'established'.*$//" ;;
  daemon-down) stop_unit spike-daemon.service ;;
  daemon-up) rm -f "$S/run/galleriad.sock"; run --unit=spike-daemon "$S/fake-daemon" "$S/run" ;;
  announce) python3 "$S/atspi-announce.py" "${@:2}" ;;
  focus)
    # Anchor a tour to the actual control, not the number of Tab stops of an older version of the app.
    python3 - "$2" <<'PY'
import sys
import gi
gi.require_version("Atspi", "2.0")
from gi.repository import Atspi
def walk(n, depth=0):
    if depth > 40: return False
    if n.get_name() == sys.argv[1] and n.get_role_name() in ("entry", "combo box", "push button", "button", "page tab", "list item"):
        if n.get_component_iface().grab_focus():
            print("focused", n.get_role_name(), repr(n.get_name()))
            return True
    for i in range(n.get_child_count()):
        c = n.get_child_at_index(i)
        if c and walk(c, depth + 1): return True
    return False
d = Atspi.get_desktop(0)
for i in range(d.get_child_count()):
    a = d.get_child_at_index(i)
    if a and "galleria" in (a.get_name() or "").lower() and walk(a): raise SystemExit(0)
raise SystemExit("no focus target " + sys.argv[1])
PY
    ;;
  dump) python3 "$S/atspi-dump.py" "${@:2}" ;;
  *) sed -n '2,15p' "$0"; exit 2 ;;
esac
