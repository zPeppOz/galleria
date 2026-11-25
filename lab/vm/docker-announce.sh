#!/usr/bin/env bash
# Spike Orca (docs/08 «Spike Orca»): lo stesso segnale ATK `announcement` dentro l'immagine `galleria-gui-dev` (Orca 46.1,
# at-spi2-core 2.52, Xvfb senza gestore di finestre, come il gate a11y), per sapere se il gate di Q22 potrebbe provare la
# strada. Da lanciare dall'host, dalla radice del repo: lab/vm/docker-announce.sh
set -euo pipefail
cd -- "$(dirname -- "$0")/../.."
out=${OUT:-/mnt/b/vm/spike/docker-announce.txt}
docker run --rm -v "$PWD/lab:/lab:ro" galleria-gui-dev bash -c '
set -u
export WEBKIT_DISABLE_SANDBOX_THIS_IS_DANGEROUS=1
cat > /tmp/inner.sh <<"INNER"
gsettings set org.gnome.desktop.interface toolkit-accessibility true
/usr/libexec/at-spi-bus-launcher --launch-immediately >/tmp/bus.log 2>&1 &
python3 /lab/vm/orca-line-buffered.py --debug-file=/tmp/orca.log >/tmp/orca.out 2>&1 &
for _ in $(seq 1 100); do grep -q "SPEECH OUTPUT" /tmp/orca.log 2>/dev/null && break; sleep 0.1; done
say() { grep "SPEECH OUTPUT" /tmp/orca.log | sed -E "s/^.*SPEECH OUTPUT: //; s/ ?[{]\x27established\x27.*$//"; }
n=$(grep -c "SPEECH OUTPUT" /tmp/orca.log)
echo "orca: $(dpkg-query -W -f="\${Version}" orca)  at-spi2-core: $(dpkg-query -W -f="\${Version}" at-spi2-core)"
echo "== C: ATK announcement dentro una finestra GTK3 (Xvfb, senza gestore di finestre)"
python3 /lab/vm/gtk-announce.py --delay 3 "Docker C: atk announcement from a GTK3 window" 2>&1 | grep -v -i "warn\|^$"
say | tail -n +$((n+1)); n=$(grep -c "SPEECH OUTPUT" /tmp/orca.log)
echo "== D: ATK notification assertive"
python3 /lab/vm/gtk-announce.py --signal notification --assertive --delay 3 "Docker D: atk notification assertive" 2>&1 | grep -v -i "warn\|^$"
say | tail -n +$((n+1))
echo "-- eventi announcement visti da Orca:"
grep "Dequeued object:announcement" /tmp/orca.log | cut -c1-200
INNER
dbus-run-session -- xvfb-run -a --server-args="-screen 0 1280x800x24" bash /tmp/inner.sh
' 2>&1 | tee "$out"
