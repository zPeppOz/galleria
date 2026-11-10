#!/bin/bash
# Server WireGuard di prova: chiavi generate all'avvio, profilo del client scritto in /out/wgtest.conf.
set -e
umask 077
SK=$(wg genkey); CK=$(wg genkey)
ip link add wg0 type wireguard
ip addr add 10.99.0.1/24 dev wg0
wg set wg0 listen-port 51820 private-key <(echo "$SK") peer "$(echo "$CK" | wg pubkey)" allowed-ips 10.99.0.2/32
ip link set wg0 up
iptables -t nat -A POSTROUTING -s 10.99.0.0/24 -o eth0 -j MASQUERADE
# il "sito" dual-stack (198.51.100.1) sta dietro al container target, come nello spike 2
ip route add 198.51.100.1/32 via 172.30.0.20
# Endpoint 127.0.0.1: nella Sandbox il server e' raggiunto dal relay locale (spikes/windows-lab/filerelay).
# Profilo "tutto nel tunnel" solo IPv4, come chiede il criterio 1.5.
cat > /out/wgtest.conf <<EOF
[Interface]
PrivateKey = $CK
Address = 10.99.0.2/24

[Peer]
PublicKey = $(echo "$SK" | wg pubkey)
Endpoint = 127.0.0.1:51820
AllowedIPs = 0.0.0.0/0
PersistentKeepalive = 5
EOF
chmod 644 /out/wgtest.conf
echo "wg server up"; wg show wg0 | grep -v private
exec sleep infinity
