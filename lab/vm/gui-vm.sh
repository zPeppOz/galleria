#!/usr/bin/env bash
# Dall'host: mette l'app di Galleria in una delle tre VM desktop (plan B6, T8) e la prepara per le prove manuali (T9, T10).
# Dischi e prove stanno in /mnt/b/vm, mai nel repo. La VM deve essere accesa (`lab/vm/vmctl.sh start <vm>`).
#
#   gui-vm.sh <vm> install       copia i binari costruiti nell'immagine `galleria-gui-dev` (scripts/gui.sh build: galleria-gui,
#                                fake-daemon, seed-rubrica) e quelli del laboratorio (lab/out/bin: galleria, galleriad), il file
#                                .desktop e le icone; update-desktop-database; gli script dello spike in ~/spike
#   gui-vm.sh <vm> versions      GNOME Shell, WebKitGTK, Orca, libreria dell'indicatore trovata, estensione AppIndicator, sessione
#   gui-vm.sh <vm> daemon fake <scenario>   demone finto (examples/fake-daemon) nel socket di ~/spike/run, senza demone vero
#   gui-vm.sh <vm> daemon real             il demone vero: packaging/linux/install.sh, utente `lab` nel gruppo galleria
#   gui-vm.sh <vm> app [env...]  l'app dal menu delle applicazioni come la lancerebbe una persona (gtk-launch), `stderr` in ~/spike/app.log
#   gui-vm.sh <vm> sh <comando>  un comando in VM, da `lab`
#
# <vm> è ubuntu24 | ubuntu | fedora. Variabili: GUI_TARGET (default il target/gui-image del worktree, o quello del gruppo C),
# LAB_OUT (default lab/out del worktree).
set -euo pipefail
here=$(cd -- "$(dirname -- "$0")" && pwd)
repo=$(cd -- "$here/../.." && pwd)
B=/mnt/b/vm
vm=${1:?uso: gui-vm.sh <ubuntu24|ubuntu|fedora> install|versions|daemon|app|sh}; cmd=${2:?comando}; shift 2
case $vm in ubuntu) port=2201;; fedora) port=2202;; ubuntu24) port=2203;; *) echo "vm?" >&2; exit 2;; esac
SSHO=(-i "$B/ssh/id_vm" -o StrictHostKeyChecking=no -o "UserKnownHostsFile=$B/ssh/known_hosts_$vm" -o LogLevel=ERROR -o ConnectTimeout=5)
vssh() { ssh "${SSHO[@]}" -p "$port" lab@127.0.0.1 "$@"; }
target=${GUI_TARGET:-$repo/target/gui-image}
labout=${LAB_OUT:-$repo/lab/out}
# Point GUI_TARGET and LAB_OUT at a prior build only when its source commit is the one being checked; keep that commit and
# the binary hashes in the evidence. No implicit fallback to a different checkout.

case $cmd in
install)
  for f in "$target/release/galleria-gui" "$target/debug/examples/fake-daemon" "$target/debug/examples/seed-rubrica" "$labout/bin/galleria" "$labout/bin/galleriad"; do
    [[ -x $f ]] || { echo "manca $f" >&2; exit 1; }
  done
  vssh 'mkdir -p ~/spike/new/release ~/spike/new/debug/examples ~/spike/bin ~/.local/share/applications ~/.local/share/icons/hicolor/128x128/apps'
  rsync -a -e "ssh ${SSHO[*]} -p $port" "$target/release/galleria-gui" lab@127.0.0.1:spike/new/release/
  rsync -a -e "ssh ${SSHO[*]} -p $port" "$target/debug/examples/fake-daemon" "$target/debug/examples/seed-rubrica" lab@127.0.0.1:spike/new/debug/examples/
  rsync -a -e "ssh ${SSHO[*]} -p $port" "$labout/bin/galleria" "$labout/bin/galleriad" lab@127.0.0.1:spike/bin/
  rsync -a -e "ssh ${SSHO[*]} -p $port" "$repo/packaging/linux/" lab@127.0.0.1:spike/packaging-linux/
  rsync -a -e "ssh ${SSHO[*]} -p $port" "$repo/crates/galleria-gui/icons/128x128.png" lab@127.0.0.1:.local/share/icons/hicolor/128x128/apps/io.github.zpeppoz.galleria.png
  for f in orca-line-buffered.py orca-spike-vm.sh orca-credentials-vm.sh atspi-announce.py gtk-announce.py gui-vm.sh daily-vm.sh wg-peer-vm.sh keyring-create.py tray-icon-probe.sh; do
    [[ -f $here/$f ]] && rsync -a -e "ssh ${SSHO[*]} -p $port" "$here/$f" lab@127.0.0.1:spike/
  done
  rsync -a -e "ssh ${SSHO[*]} -p $port" "$repo/lab/tests/atspi-dump.py" lab@127.0.0.1:spike/
  # as a package would: the binary where the .desktop file finds it by name (the VM is disposable)
  vssh 'sudo install -m755 ~/spike/new/release/galleria-gui /usr/local/bin/galleria-gui
        cp ~/spike/new/release/galleria-gui ~/spike/galleria-gui; cp ~/spike/new/debug/examples/fake-daemon ~/spike/fake-daemon
        cp ~/spike/packaging-linux/io.github.zpeppoz.galleria.desktop ~/.local/share/applications/
        grep -q "^Icon=" ~/.local/share/applications/io.github.zpeppoz.galleria.desktop || echo "Icon=io.github.zpeppoz.galleria" >> ~/.local/share/applications/io.github.zpeppoz.galleria.desktop
        update-desktop-database ~/.local/share/applications; gtk-update-icon-cache -qf ~/.local/share/icons/hicolor 2>/dev/null || true
        chmod +x ~/spike/*.sh ~/spike/*.py; ldd /usr/local/bin/galleria-gui | grep "not found" || echo "ldd: nothing missing"'
  ;;
versions)
  vssh 'export XDG_RUNTIME_DIR=/run/user/$(id -u) DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/$(id -u)/bus
    bash ~/spike/orca-spike-vm.sh versions
    . /etc/os-release
    echo "indicator library mapped by a running app: $(for p in $(pgrep -x galleria-gui); do grep -ho "libayatana-appindicator3[^ ]*\|libappindicator3[^ ]*" /proc/$p/maps | sort -u; done | tr "\n" " ")"
    echo "indicator libraries installed: $(ldconfig -p | grep -o "lib\(ayatana-\)\?appindicator3\.so\.1" | sort -u | tr "\n" " ")"
    echo "extensions: $(gnome-extensions list --enabled 2>/dev/null | tr "\n" " ")"
    echo "appindicator extension: $(gnome-extensions info appindicatorsupport@rgcjonas.gmail.com 2>/dev/null | grep -E "State|Enabled" | tr "\n" " ") $(gnome-extensions info ubuntu-appindicators@ubuntu.com 2>/dev/null | grep -E "State|Enabled" | tr "\n" " ")"
    echo "SSH session type: $XDG_SESSION_TYPE (not the desktop)"
    for s in $(loginctl show-user lab -p Sessions --value); do loginctl show-session "$s" -p Id -p Type -p Remote -p Desktop; done'
  ;;
daemon)
  case ${1:-} in
  fake) vssh "export XDG_RUNTIME_DIR=/run/user/\$(id -u); systemctl --user stop spike-daemon.service 2>/dev/null; systemctl --user reset-failed spike-daemon.service 2>/dev/null; rm -rf ~/spike/run; mkdir -p ~/spike/run; systemd-run --user --quiet --collect --unit=spike-daemon ~/spike/new/debug/examples/fake-daemon ~/spike/run --scenario ${2:-default}; sleep 1; ls ~/spike/run" ;;
  real) vssh 'mkdir -p ~/spike/packaging/linux; cp ~/spike/packaging-linux/* ~/spike/packaging/linux/
    sudo bash ~/spike/packaging/linux/install.sh ~/spike/bin && sudo usermod -aG galleria lab && sudo systemctl is-active galleriad' ;;
  *) echo "daemon fake <scenario> | real" >&2; exit 2 ;;
  esac
  ;;
app) extra=''; if (($#)); then printf -v extra -- '-E %q ' "$@"; fi
  vssh "export XDG_RUNTIME_DIR=/run/user/\$(id -u); export DBUS_SESSION_BUS_ADDRESS=unix:path=\$XDG_RUNTIME_DIR/bus WAYLAND_DISPLAY=wayland-0; systemd-run --user --quiet --collect --unit=spike-app -E WAYLAND_DISPLAY -E XDG_SESSION_TYPE=wayland ${extra:-} bash -c 'exec /usr/local/bin/galleria-gui 2> ~/spike/app.log'" ;;
sh) vssh "$@" ;;
*) sed -n '2,16p' "$0"; exit 2 ;;
esac
