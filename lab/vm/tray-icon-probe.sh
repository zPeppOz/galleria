#!/usr/bin/env bash
# VM, da lab: IconName/IconThemePath dell'indicatore di Galleria e i file dell'icona, per capire che cosa vede l'estensione.
export XDG_RUNTIME_DIR=/run/user/$(id -u) DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/$(id -u)/bus
w=$(gdbus call --session --dest org.kde.StatusNotifierWatcher --object-path /StatusNotifierWatcher --method org.freedesktop.DBus.Properties.Get org.kde.StatusNotifierWatcher RegisteredStatusNotifierItems | grep -o ":[0-9.]*@/[^']*galleria[^']*" | head -1)
dest=${w%%@*}; path=${w#*@}
echo "item: $dest $path"
for p in IconName IconThemePath Status AttentionIconName; do printf '%s ' "$p"; gdbus call --session --dest "$dest" --object-path "$path" --method org.freedesktop.DBus.Properties.Get org.kde.StatusNotifierItem "$p" 2>&1 | cut -c1-160; done
ls -la --time-style=full-iso /run/user/$(id -u)/galleria | grep tray
for f in /run/user/$(id -u)/galleria/tray-icon-*; do md5sum "$f"; done 2>/dev/null
