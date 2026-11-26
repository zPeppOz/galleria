#!/bin/bash
# S14 by hand on a desktop VM (root, after vm-lab.sh up): DCO really opened, `connected`, and the
# byte counters grow with real traffic through the tunnel. Needs the in-kernel `ovpn` module.
set -u
lab=${LAB_ROOT:-/lab}
cli() { runuser -u alice -- env LC_ALL=C XDG_CONFIG_HOME=/home/alice/.config /usr/local/bin/galleria "$@"; }
say() { printf '\n### %s\n' "$*"; }
stats() { cli status --json | python3 -c 'import sys,json; s=json.load(sys.stdin); print(s["state"], "stats=", s.get("stats"))'; }

say "ambiente"
. /etc/os-release; echo "$PRETTY_NAME, kernel $(uname -r)"; openvpn --version | head -1
modprobe ovpn && lsmod | grep -E '^ovpn'
modinfo -F filename ovpn

[ -d /home/alice/.config/galleria ] && mv /home/alice/.config/galleria "/home/alice/galleria-config-old-$$"
install -d -o alice -g alice -m700 /home/alice/incoming
install -o alice -g alice -m600 "$lab/lab/out/base.ovpn" /home/alice/incoming/base.ovpn
cli import /home/alice/incoming/base.ovpn --name base >/dev/null
since=$(date '+%Y-%m-%d %H:%M:%S')

say "connect"
printf 'alice-secret\n' | cli connect base --username alice --password-stdin
stats
say "il tun è un'interfaccia ovpn (DCO), non un tun userspace"
ip -d link show glr-ovpn0 | sed -n '1,3p'
say "bytecount prima del traffico"
ip -s link show glr-ovpn0 | sed -n '3,6p'
stats
say "traffico vero nel tunnel: 200 ping da 1000 byte verso 10.8.0.1 e 20 richieste HTTP via il tunnel (198.51.100.1 passa dal full tunnel)"
ping -c 200 -i 0.02 -s 1000 -q 10.8.0.1 | tail -2
for i in $(seq 20); do curl --noproxy '*' -s -o /dev/null -m5 http://198.51.100.1/; done
ip route get 198.51.100.1 | head -1
say "bytecount dopo il traffico"
ip -s link show glr-ovpn0 | sed -n '3,6p'
stats
say "log del demone: DCO aperto"
journalctl -u galleriad --since "$since" --no-pager -o cat | grep -iE 'dco|ovpn' | head -6

say "disconnect e pulizia"
cli disconnect
sleep 1
ip -br link | grep -c glr- | sed 's/^/interfacce glr-: /'
ip rule; ip -6 rule
resolvectl dns | grep -c glr- | sed 's/^/righe resolvectl con glr-: /'
