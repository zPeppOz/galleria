#!/bin/bash
# S15 by hand on a desktop VM (run as root, after vm-lab.sh up): what NetworkManager and systemd-resolved
# say while each backend is connected and after it is torn down. The e2e tests prove the same facts
# with assertions; this prints the evidence (nmcli, resolvectl, timing of DNS vs `connected`).
set -u
lab=${LAB_ROOT:-/lab}
cli() { runuser -u alice -- env LC_ALL=C XDG_CONFIG_HOME=/home/alice/.config /usr/local/bin/galleria "$@"; }
say() { printf '\n### %s\n' "$*"; }
base_dns() { resolvectl dns; resolvectl domain; ip -br link | awk '{print $1, $2}'; ip route; ip -6 route; ip rule; ip -6 rule; }

say "ambiente"
. /etc/os-release; echo "$PRETTY_NAME, kernel $(uname -r)"
nmcli --version; resolvectl --version | head -1; openvpn --version | head -1
nmcli -g GENERAL.NM-MANAGED device show eth0 | sed 's/^/eth0 managed: /'
cat /etc/NetworkManager/conf.d/90-galleria.conf

rm -rf /home/alice/.config/galleria /home/alice/incoming
install -d -o alice -g alice -m700 /home/alice/incoming
for f in wg-split.conf base.ovpn dns-option.ovpn; do
  install -o alice -g alice -m600 "$lab/lab/out/$f" "/home/alice/incoming/$f"
done
cli import /home/alice/incoming/wg-split.conf --name wg >/dev/null
cli import /home/alice/incoming/base.ovpn --name ovpn >/dev/null
cli disconnect >/dev/null 2>&1; sleep 1
base_dns > /var/tmp/s15-before.txt
say "prima: nmcli device (nessuna glr-*) e DNS di base"
nmcli device; resolvectl dns

say "WireGuard: connect, nmcli device, DNS"
cli connect wg; cli status
nmcli device | grep -E 'DEVICE|glr-' ; nmcli -f GENERAL.DEVICE,GENERAL.STATE,GENERAL.NM-MANAGED device show glr-wg0 2>&1 | head -4
nmcli -t -f NAME connection show | grep -c glr- | sed 's/^/profili NM con glr-: /'
resolvectl dns glr-wg0; resolvectl domain glr-wg0
nmcli device show eth0 | grep -E 'GENERAL.STATE|IP4.ADDRESS'
cli disconnect; sleep 1
say "dopo disconnect WireGuard"
nmcli device | grep -c glr- | sed 's/^/righe glr- in nmcli device: /'
base_dns > /var/tmp/s15-after-wg.txt; diff /var/tmp/s15-before.txt /var/tmp/s15-after-wg.txt && echo "stato di rete e DNS identici al prima"

say "OpenVPN: DNS applicato prima di connected (campionamento ogni ~50 ms durante connect)"
( for i in $(seq 1 400); do
    s=$(cli status --json 2>/dev/null | python3 -c 'import sys,json; print(json.load(sys.stdin)["state"])' 2>/dev/null)
    d=$(resolvectl dns glr-ovpn0 2>/dev/null | sed 's/.*: //')
    echo "$(date +%s.%N) state=${s:-?} dns=[${d}]"
    [[ $s == connected ]] && break
    sleep 0.05
  done ) > /var/tmp/s15-ovpn-poll.txt &
poller=$!
printf 'alice-secret\n' | cli connect ovpn --username alice --password-stdin
wait $poller
awk '{ if ($2!=p2 || $3!=p3) print; p2=$2; p3=$3 }' /var/tmp/s15-ovpn-poll.txt
nmcli device | grep -E 'DEVICE|glr-'
resolvectl dns glr-ovpn0; resolvectl domain glr-ovpn0
resolvectl query intranet.galleria.test | head -3
cli disconnect; sleep 1
say "dopo disconnect OpenVPN"
nmcli device | grep -c glr- | sed 's/^/righe glr- in nmcli device: /'
base_dns > /var/tmp/s15-after-ovpn.txt; diff /var/tmp/s15-before.txt /var/tmp/s15-after-ovpn.txt && echo "stato di rete e DNS identici al prima"
