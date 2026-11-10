#!/bin/bash
# uso: entrypoint.sh [server.conf|server-dns.conf]
set -e
CONF="${1:-server.conf}"
case "$CONF" in server-dns.conf) GW=10.9.0.1 ;; *) GW=10.8.0.1 ;; esac
mkdir -p /dev/net; [ -c /dev/net/tun ] || mknod /dev/net/tun c 10 200
iptables -t nat -A POSTROUTING -s 10.0.0.0/8 -o eth0 -j MASQUERADE
# la "internet" di prova (198.51.100.1 / 2001:db8:99::1) sta dietro al container target
ip route add 198.51.100.1/32 via 172.30.0.20
# DNS spinto ai client: dnsmasq sul gateway VPN (tun0 compare dopo l'avvio di openvpn)
dnsmasq --interface=tun0 --bind-dynamic --no-resolv --no-hosts \
  --address=/intranet.galleria.test/$GW --log-queries --log-facility=/var/log/dnsmasq.log
exec openvpn --config "/etc/openvpn/lab/$CONF"
