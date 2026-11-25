#!/bin/bash
# Minimal test installation; run inside the client, or explicitly on a manual test VM.
set -euo pipefail
root=$(cd "$(dirname "$0")/../.." && pwd)
bin=${1:-$root/lab/out/bin}
install -Dm755 "$bin/galleriad" /usr/local/libexec/galleria/galleriad
install -Dm755 "$bin/galleria" /usr/local/bin/galleria
install -Dm644 "$root/packaging/linux/galleria.sysusers" /usr/lib/sysusers.d/galleria.conf
systemd-sysusers /usr/lib/sysusers.d/galleria.conf
install -Dm644 "$root/packaging/linux/90-galleria.conf" /etc/NetworkManager/conf.d/90-galleria.conf
# NetworkManager reads conf.d at start: without a reload, glr-* stays managed until the next boot.
if systemctl is-active --quiet NetworkManager; then systemctl reload NetworkManager; fi
install -Dm644 "$root/packaging/linux/galleriad.service" /etc/systemd/system/galleriad.service
systemctl daemon-reload
systemctl enable --now galleriad
