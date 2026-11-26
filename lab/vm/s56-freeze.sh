#!/usr/bin/env bash
# VM, as lab: S56 test C. Freeze gnome-shell (SIGSTOP: its StatusNotifierWatcher stays owned but cannot answer), start the app,
# thaw it after FREEZE s, and see which warning the app prints and whether the icon registers once the shell is back.
export XDG_RUNTIME_DIR=/run/user/$(id -u) DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/$(id -u)/bus WAYLAND_DISPLAY=wayland-0 XDG_SESSION_TYPE=wayland
pkill -x galleria-gui; systemctl --user stop spike-app.service 2>/dev/null; sleep 2; systemctl --user reset-failed 2>/dev/null
shell=$(pgrep -x gnome-shell | head -1)
echo "gnome-shell pid $shell"
: > ~/spike/s56-app.log
trap 'kill -CONT "$shell" 2>/dev/null' EXIT
kill -STOP "$shell"
systemd-run --user --quiet --collect --unit=spike-app -E WAYLAND_DISPLAY -E XDG_SESSION_TYPE=wayland bash -c 'exec /usr/local/bin/galleria-gui 2> ~/spike/s56-app.log'
sleep ${FREEZE:-55}
echo "while frozen: warnings=$(grep -c 'Notification Watcher' ~/spike/s56-app.log)"
grep -v MESA ~/spike/s56-app.log
kill -CONT "$shell"; sleep 6
echo "thawed: $(gdbus call --session --dest org.kde.StatusNotifierWatcher --object-path /StatusNotifierWatcher --method org.freedesktop.DBus.Properties.Get org.kde.StatusNotifierWatcher RegisteredStatusNotifierItems 2>&1 | cut -c1-140)"
