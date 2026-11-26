#!/usr/bin/env bash
# VM, as lab, graphical session: S56. Start the app N times in different ways and record, per start, whether the
# Notification Watcher warning shows up, how long the first window takes and what the registered item is.
#   s56-repeat.sh <mode> <gap-seconds> <times>      mode: menu (gtk-launch) | direct (systemd-run + binary, as gui-vm.sh app)
export XDG_RUNTIME_DIR=/run/user/$(id -u) DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/$(id -u)/bus WAYLAND_DISPLAY=wayland-0 XDG_SESSION_TYPE=wayland
mode=${1:-menu}; gap=${2:-1}; n=${3:-4}
for i in $(seq "$n"); do
  pkill ${SIG:--TERM} -x galleria-gui; systemctl --user stop daily-app.service spike-app.service 2>/dev/null; systemctl --user reset-failed 2>/dev/null
  sleep "$gap"
  : > ~/spike/s56-app.log
  t0=$(date +%s.%N)
  if [[ $mode == menu ]]; then
    systemd-run --user --quiet --collect --unit=daily-app -E WAYLAND_DISPLAY -E XDG_SESSION_TYPE=wayland bash -c 'gtk-launch io.github.zpeppoz.galleria 2> ~/spike/s56-app.log; sleep 2; while pgrep -x galleria-gui >/dev/null; do sleep 1; done'
  else
    systemd-run --user --quiet --collect --unit=spike-app -E WAYLAND_DISPLAY -E XDG_SESSION_TYPE=wayland bash -c 'exec /usr/local/bin/galleria-gui 2> ~/spike/s56-app.log'
  fi
  for _ in $(seq 60); do pgrep -x galleria-gui >/dev/null && break; sleep 0.25; done
  sleep ${WAIT:-30}
  reg=$(gdbus call --session --dest org.kde.StatusNotifierWatcher --object-path /StatusNotifierWatcher --method org.freedesktop.DBus.Properties.Get org.kde.StatusNotifierWatcher RegisteredStatusNotifierItems 2>&1 | cut -c1-120)
  echo "run $i mode=$mode gap=$gap start=$(date -d @${t0%.*} +%T) pid=$(pgrep -x galleria-gui | tr '\n' ' ') watcher-warning=$(grep -c 'Notification Watcher' ~/spike/s56-app.log) gtk-critical=$(grep -c Gtk-CRITICAL ~/spike/s56-app.log) registered=$reg"
  grep -v MESA ~/spike/s56-app.log | sed 's/^/    /'
done
