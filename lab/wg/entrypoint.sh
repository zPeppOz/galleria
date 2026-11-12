#!/bin/bash
set -euo pipefail
umask 077
server_key=$(wg genkey)
client_key=$(wg genkey)
server_public=$(printf '%s\n' "$server_key" | wg pubkey)
client_public=$(printf '%s\n' "$client_key" | wg pubkey)
ip link add wg0 type wireguard
ip addr add 10.99.0.1/24 dev wg0
printf '[Interface]\nPrivateKey = %s\nListenPort = 51820\n[Peer]\nPublicKey = %s\nAllowedIPs = 10.99.0.2/32\n' "$server_key" "$client_public" | wg setconf wg0 /dev/stdin
ip link set wg0 up
ip route add 198.51.100.1/32 via 172.31.0.20
iptables -t nat -A POSTROUTING -s 10.99.0.0/24 -o eth0 -j MASQUERADE
for mode in split full; do
  allowed='0.0.0.0/0'
  if [[ $mode == split ]]; then allowed='10.99.0.0/24, 198.51.100.1/32'; fi
  cat > "/out/wg-$mode.conf" <<PROFILE
[Interface]
PrivateKey = $client_key
Address = 10.99.0.2/24
DNS = 10.99.0.1
[Peer]
PublicKey = $server_public
Endpoint = 172.31.0.12:51820
AllowedIPs = $allowed
PersistentKeepalive = 5
PROFILE
done
unset server_key client_key
exec dnsmasq --keep-in-foreground --interface=wg0 --bind-dynamic --no-resolv --no-hosts --host-record=dual.galleria.test,198.51.100.1,2001:db8:99::1
