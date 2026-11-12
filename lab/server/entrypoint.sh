#!/bin/bash
set -euo pipefail
conf=${1:-server.conf}
case "$conf" in server.conf) gw=10.8.0.1;; server-dns.conf) gw=10.9.0.1;; server-split.conf) gw=10.13.0.1;; server-v6.conf) gw=10.14.0.1;; *) exit 2;; esac
ip route add 198.51.100.1/32 via 172.31.0.20
ip -6 route add 2001:db8:99::1/128 via 2001:db8:31::20
iptables -t nat -A POSTROUTING -s 10.0.0.0/8 -o eth0 -j MASQUERADE
ip6tables -t nat -A POSTROUTING -s 2001:db8:88::/64 -o eth0 -j MASQUERADE
dnsmasq --interface=tun0 --bind-dynamic --no-resolv --no-hosts --address=/intranet.galleria.test/"$gw" --host-record=dual.galleria.test,198.51.100.1,2001:db8:99::1
exec openvpn --config "/lab/server/$conf"
