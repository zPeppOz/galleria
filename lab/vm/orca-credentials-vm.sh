#!/usr/bin/env bash
# Dentro la VM, da utente `lab`, nella sessione GNOME vera (docs/08, Q49): apre la finestra delle credenziali della GUI di B5
# (profilo «Datacenter Milano», scenario `credentials` del demone finto) e stampa quello che Orca dice. Come `orca-spike-vm.sh`
# ma con la rubrica seminata (`seed-rubrica`) e il clic sul pulsante «Connect» fatto da AT-SPI, non con la tastiera.
# Servono in ~/spike/new: release/galleria-gui, debug/examples/fake-daemon, debug/examples/seed-rubrica (dall'immagine
# `galleria-gui-dev`: target/gui-image/…); in ~/spike: orca-line-buffered.py, atspi-dump.py, orca-spike-vm.sh.
#
#   orca-credentials-vm.sh run         Orca nuovo, rubrica, demone, finestra; apre le credenziali; stampa il parlato
set -uo pipefail
export XDG_RUNTIME_DIR=/run/user/$(id -u)
export DBUS_SESSION_BUS_ADDRESS=unix:path=$XDG_RUNTIME_DIR/bus WAYLAND_DISPLAY=wayland-0 XDG_SESSION_TYPE=wayland
S=$HOME/spike N=$S/new
CFG=$S/cfg
ENVS=(-E XDG_RUNTIME_DIR -E DBUS_SESSION_BUS_ADDRESS -E WAYLAND_DISPLAY -E XDG_SESSION_TYPE)
run() { systemd-run --user --quiet --collect "${ENVS[@]}" "$@"; }
stop_unit() { systemctl --user stop "$1" 2>/dev/null; systemctl --user reset-failed "$1" 2>/dev/null; return 0; }
LOG=$S/orca-debug.log

click_connect() {
  python3 - <<'PY'
import gi
gi.require_version("Atspi", "2.0")
from gi.repository import Atspi

def find(node, role, name, depth=0):
    if depth > 40:
        return None
    if node.get_role_name() in (role, "push button" if role == "button" else role) and (node.get_name() or "") == name:
        return node
    for i in range(node.get_child_count()):
        child = node.get_child_at_index(i)
        if child is None:
            continue
        found = find(child, role, name, depth + 1)
        if found:
            return found
    return None

desktop = Atspi.get_desktop(0)
for i in range(desktop.get_child_count()):
    app = desktop.get_child_at_index(i)
    if app and "galleria" in (app.get_name() or "").lower():
        button = find(app, "button", "Connect")
        if button:
            print("click", button.get_name())
            button.get_action_iface().do_action(0)
            raise SystemExit(0)
raise SystemExit("no Connect button")
PY
}

case ${1:-} in
  run|prep|open)
    # prep: Orca, rubrica, demone finto, finestra (si ferma qui, così chi guida può rendere attivo il frame);
    # open: stampa se il frame è `active`, poi apre le credenziali e stampa il parlato (Q49: apertura da frame già attivo);
    # run: i due di fila, come prima.
    if [[ $1 != open ]]; then
    for unit in spike-app spike-daemon spike-orca daily-app; do stop_unit "$unit.service"; done
    pkill -x galleria-gui 2>/dev/null
    pkill -x orca 2>/dev/null; sleep 1
    gsettings set org.gnome.desktop.interface toolkit-accessibility true
    rm -rf "$CFG" "$S/run"; mkdir -p "$CFG" "$S/run"
    XDG_CONFIG_HOME=$CFG "$N/debug/examples/seed-rubrica" > /dev/null || exit 1
    : > "$LOG"
    run --unit=spike-orca python3 "$S/orca-line-buffered.py" --debug-file="$LOG"
    for _ in $(seq 1 60); do grep -q 'SPEECH OUTPUT' "$LOG" 2>/dev/null && break; sleep 0.5; done
    run --unit=spike-daemon "$N/debug/examples/fake-daemon" "$S/run" --scenario credentials
    for _ in $(seq 1 50); do [[ -S $S/run/galleriad.sock ]] && break; sleep 0.2; done
    # NO_KEYRING=1: on a desktop whose login keyring is locked (Fedora, automatic login) the window of Rust would ask to unlock it
    # before the credentials; without the session bus the store is the file, as in the image of the screen reader check. The
    # AT-SPI bus is given by its address, so Orca still sees the window.
    nokeyring=()
    if [[ ${NO_KEYRING:-0} == 1 ]]; then
      nokeyring=(-E DBUS_SESSION_BUS_ADDRESS=unix:path=/nonexistent -E AT_SPI_BUS_ADDRESS="$(busctl --user call org.a11y.Bus /org/a11y/bus org.a11y.Bus GetAddress | sed -E 's/^s "(.*)"$/\1/')")
    fi
    run --unit=spike-app -E GALLERIA_SOCKET="$S/run/galleriad.sock" -E XDG_CONFIG_HOME="$CFG" "${nokeyring[@]}" "$N/release/galleria-gui"
    sleep 6
    fi
    [[ $1 == prep ]] && exit 0
    echo "frame active before opening: $(python3 "$S/atspi-dump.py" | awk -F'\t' '$2 == "frame" {print ($4 ~ /active/) ? "yes" : "NO"; exit}')"
    for round in 1; do
      before=$(grep -c 'SPEECH OUTPUT' "$LOG")
      # the profile that asks for credentials is «Datacenter Milano»: the row is selected first
      python3 - <<'PY'
import gi
gi.require_version("Atspi", "2.0")
from gi.repository import Atspi
def walk(n, d=0):
    if d > 40: return
    for i in range(n.get_child_count()):
        c = n.get_child_at_index(i)
        if c is None: continue
        if (c.get_name() or "") == "Datacenter Milano" and c.get_role_name() in ("list item", "table row", "list box item", "menu item", "push button", "radio button", "link"):
            try:
                c.get_action_iface().do_action(0); print("selected", c.get_role_name()); raise SystemExit(0)
            except SystemExit: raise
            except Exception as e: print("no action", e)
        walk(c, d + 1)
d = Atspi.get_desktop(0)
for i in range(d.get_child_count()):
    a = d.get_child_at_index(i)
    if a and "galleria" in (a.get_name() or "").lower(): walk(a)
PY
      sleep 2
      click_connect || exit 1
      sleep 5
      echo "--- round $round: what Orca said after «Connect»"
      grep 'SPEECH OUTPUT' "$LOG" | tail -n +$((before + 1)) | sed -E "s/^.*SPEECH OUTPUT: //; s/ ?[{]'established'.*$//"
      python3 "$S/atspi-dump.py" | awk -F'\t' '$4 ~ /focused/ && $2 != "frame" {print "focus: " $2 " «" $3 "»"}'
      python3 "$S/atspi-dump.py" | awk -F'\t' '$2 == "dialog" || $2 == "alert" {print "role: " $2 " «" $3 "»"}'
      break
    done ;;
  *) sed -n '2,9p' "$0"; exit 2 ;;
esac
