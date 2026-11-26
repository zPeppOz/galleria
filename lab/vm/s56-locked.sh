#!/usr/bin/env bash
# VM, as lab: S56 test A. Lock the graphical session (as the idle lock would), start the app, look at the log, unlock.
export XDG_RUNTIME_DIR=/run/user/$(id -u) DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/$(id -u)/bus WAYLAND_DISPLAY=wayland-0 XDG_SESSION_TYPE=wayland
pkill -x galleria-gui; systemctl --user stop daily-app.service spike-app.service 2>/dev/null; sleep 2; systemctl --user reset-failed 2>/dev/null
sid=$(loginctl list-sessions --no-legend | awk '$3=="lab" && $4=="seat0"{print $1}')
echo "session $sid; locked-hint before: $(loginctl show-session "$sid" -p LockedHint)"
loginctl lock-session "$sid"; sleep 3
echo "locked-hint after lock: $(loginctl show-session "$sid" -p LockedHint)"
: > ~/spike/s56-app.log
systemd-run --user --quiet --collect --unit=spike-app -E WAYLAND_DISPLAY -E XDG_SESSION_TYPE=wayland bash -c 'exec /usr/local/bin/galleria-gui 2> ~/spike/s56-app.log'
sleep 40
echo "watcher warnings: $(grep -c 'Notification Watcher' ~/spike/s56-app.log)  gtk-critical: $(grep -c Gtk-CRITICAL ~/spike/s56-app.log)"
grep -v MESA ~/spike/s56-app.log
echo "registered: $(gdbus call --session --dest org.kde.StatusNotifierWatcher --object-path /StatusNotifierWatcher --method org.freedesktop.DBus.Properties.Get org.kde.StatusNotifierWatcher RegisteredStatusNotifierItems 2>&1 | cut -c1-140)"
loginctl unlock-session "$sid"; sleep 2
echo "locked-hint after unlock: $(loginctl show-session "$sid" -p LockedHint)"
