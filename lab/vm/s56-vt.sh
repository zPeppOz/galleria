#!/usr/bin/env bash
# VM, as lab: S56 test B. Put the VT of the greeter in front (as it was in T9, where the session of lab stood on tty2 behind the
# greeter on tty1), start the app, look at the log, bring the session back. Usage: s56-vt.sh <vt-away> <vt-back>
export XDG_RUNTIME_DIR=/run/user/$(id -u) DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/$(id -u)/bus WAYLAND_DISPLAY=wayland-0 XDG_SESSION_TYPE=wayland
away=${1:-1}; back=${2:-2}
loginctl list-sessions --no-legend
pkill -x galleria-gui; systemctl --user stop daily-app.service spike-app.service 2>/dev/null; sleep 2; systemctl --user reset-failed 2>/dev/null
sid=$(loginctl list-sessions --no-legend | awk '$3=="lab" && $4=="seat0"{print $1}')
echo "before: $(loginctl show-session "$sid" -p Active -p State | tr '\n' ' ') vt=$(cat /sys/class/tty/tty0/active)"
sudo chvt "$away"; sleep 3
echo "away:   $(loginctl show-session "$sid" -p Active -p State | tr '\n' ' ') vt=$(cat /sys/class/tty/tty0/active)"
: > ~/spike/s56-app.log
systemd-run --user --quiet --collect --unit=spike-app -E WAYLAND_DISPLAY -E XDG_SESSION_TYPE=wayland bash -c 'exec /usr/local/bin/galleria-gui 2> ~/spike/s56-app.log'
sleep 40
echo "watcher warnings: $(grep -c 'Notification Watcher' ~/spike/s56-app.log)  gtk-critical: $(grep -c Gtk-CRITICAL ~/spike/s56-app.log)"
grep -v MESA ~/spike/s56-app.log
echo "registered (away): $(gdbus call --session --dest org.kde.StatusNotifierWatcher --object-path /StatusNotifierWatcher --method org.freedesktop.DBus.Properties.Get org.kde.StatusNotifierWatcher RegisteredStatusNotifierItems 2>&1 | cut -c1-140)"
sudo chvt "$back"; sleep 5
echo "back:   $(loginctl show-session "$sid" -p Active -p State | tr '\n' ' ') vt=$(cat /sys/class/tty/tty0/active)"
echo "registered (back): $(gdbus call --session --dest org.kde.StatusNotifierWatcher --object-path /StatusNotifierWatcher --method org.freedesktop.DBus.Properties.Get org.kde.StatusNotifierWatcher RegisteredStatusNotifierItems 2>&1 | cut -c1-140)"
